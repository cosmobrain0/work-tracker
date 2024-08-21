#![allow(dead_code)]

mod changes;
mod errors;
mod initial_data;
mod payment;
mod project;
mod work_slice;

use std::collections::HashSet;

pub use changes::*;
use chrono::{DateTime, Utc};
pub use errors::*;
pub use initial_data::*;
pub use payment::*;
pub use project::*;
pub use work_slice::*;

/// Used to create, modify and delete projects.
#[derive(Debug)]
pub struct State {
    previous_project_id: u64,
    previous_work_slice_id: u64,
    projects: Vec<Project>,
    changes: Vec<Change>,
}
impl State {
    /// Returns an empty State, which has no projects.
    pub fn new(initial_data: Vec<ProjectData>) -> Result<Self, StateInitError> {
        let projects: Vec<_> = initial_data
            .into_iter()
            .map(ProjectData::into_project)
            .collect();

        if let Some(e) = projects.iter().find_map(|x| x.as_ref().err()) {
            return Err(StateInitError::ProjectLoadError(*e));
        }
        let projects: Vec<_> = projects.into_iter().map(Result::unwrap).collect();

        let previous_project_id = projects
            .iter()
            .map(Project::id)
            .max()
            .map(|x| unsafe { x.inner() })
            .unwrap_or(0);

        let previous_work_slice_id = projects
            .iter()
            .map(|x| {
                x.complete_work_slices().map(|x| x.id()).chain(
                    [x.current_work_slice()]
                        .into_iter()
                        .filter_map(|x| x.map(IncompleteWorkSlice::id)),
                )
            })
            .flat_map(|x| unsafe { x.map(|x| x.inner()) })
            .max()
            .unwrap_or(0);

        let mut hashmap = HashSet::new();
        unsafe {
            for id in projects.iter().map(Project::id).map(|x| x.inner()) {
                if !hashmap.insert(id) {
                    return Err(StateInitError::DuplicateProjectId);
                }
            }
            hashmap.clear();
            for id in projects
                .iter()
                .flat_map(|x| x.complete_work_slices())
                .map(CompleteWorkSlice::id)
                .map(|x| x.inner())
            {
                if !hashmap.insert(id) {
                    return Err(StateInitError::DuplicateWorkSliceId);
                }
            }
            for id in projects
                .iter()
                .filter_map(Project::current_work_slice)
                .map(|x| x.id().inner())
            {
                if !hashmap.insert(id) {
                    return Err(StateInitError::DuplicateWorkSliceId);
                }
            }
        }

        Ok(Self {
            previous_project_id,
            previous_work_slice_id,
            projects,
            changes: Vec::new(),
        })
    }

    /// Creates a new project, and returns its ID.
    pub fn new_project(&mut self, name: String, description: String) -> ProjectId {
        self.changes.push(Change::ProjectCreated {
            name: name.clone(),
            description: description.clone(),
            id: self.previous_project_id + 1,
        });
        let id = ProjectId::new(self.previous_project_id + 1);

        self.projects.push(Project::new(name, description, id));

        self.previous_project_id += 1;

        id
    }

    fn new_project_id(&mut self) -> ProjectId {
        self.previous_project_id += 1;
        ProjectId::new(self.previous_project_id)
    }

    fn new_work_slice_id(&mut self) -> WorkSliceId {
        self.previous_work_slice_id += 1;
        WorkSliceId::new(self.previous_work_slice_id)
    }
}
impl State {
    /// Returns the IDs of all projects.
    pub fn all_project_ids(&self) -> impl Iterator<Item = ProjectId> + '_ {
        self.projects.iter().map(|x| x.id())
    }

    /// Tries to find a project based on its ID,
    /// but fails if the project has been deleted.
    pub fn project_from_id(&self, id: ProjectId) -> Option<&Project> {
        self.projects.iter().find(|x| x.id() == id)
    }

    fn project_from_id_mut(&mut self, id: ProjectId) -> Option<&mut Project> {
        self.projects.iter_mut().find(|x| x.id() == id)
    }

    /// Returns all projects in its ID.
    pub fn all_projects(&self) -> impl Iterator<Item = &Project> + '_ {
        self.projects.iter()
    }

    fn all_projects_mut(&mut self) -> impl Iterator<Item = &mut Project> + '_ {
        self.projects.iter_mut()
    }

    /// Returns true if a project with the given ID exists,
    /// and returns false if the project has been deleted.
    pub fn project_exists(&self, id: ProjectId) -> bool {
        self.all_project_ids().any(|x| x == id)
    }

    /// Returns the project which the work slice specified by the given ID,
    /// but fails if the work slice has been deleted, or if the project has been deleted.
    pub fn project_id_from_work_slice(&self, work_slice_id: WorkSliceId) -> Option<ProjectId> {
        self.all_projects()
            .map(|x| {
                (
                    x.id(),
                    x.complete_work_slices().any(|x| x.id() == work_slice_id)
                        || x.current_work_slice()
                            .is_some_and(|x| x.id() == work_slice_id),
                )
            })
            .find(|(_, found)| *found)
            .map(|(id, _)| id)
    }
}
impl State {
    /// Tries to start a new incomplete work slice for a project,
    /// but can fail. See `WorkStartError` for information on how.
    pub fn start_work(
        &mut self,
        id: ProjectId,
        payment: Payment,
        time: DateTime<Utc>,
    ) -> Result<(), WorkStartError> {
        match IncompleteWorkSlice::new(time, payment, self.new_work_slice_id()) {
            Some(work_slice) => match self.project_from_id_mut(id) {
                Some(project) => {
                    let work_id = work_slice.id();
                    match project.start_work(work_slice) {
                        Ok(()) => {
                            self.changes.push(Change::WorkSliceStarted {
                                project_id: unsafe { id.inner() },
                                work_slice_id: unsafe { work_id.inner() },
                                start_time: time,
                                payment,
                            });
                            Ok(())
                        }
                        Err(_) => Err(WorkStartError::AlreadyStarted),
                    }
                }
                None => Err(WorkStartError::InvalidProjectId),
            },
            None => Err(WorkStartError::InvalidStartTime),
        }
    }

    /// Tries to end the current incomplete work slice for a project,
    // but can fail. See `WorkEndError` for more information on how.
    pub fn end_work(&mut self, id: ProjectId, time: DateTime<Utc>) -> Result<(), WorkEndError> {
        match self.project_from_id_mut(id) {
            Some(project) => {
                let work_id = project.current_work_slice().map(|x| x.id());
                match project.complete_work(time) {
                    Ok(()) => {
                        self.changes.push(Change::WorkSliceCompleted {
                            project_id: unsafe { id.inner() },
                            work_slice_id: unsafe { work_id.unwrap().inner() },
                            end_time: time,
                        });
                        Ok(())
                    }
                    Err(x) => Err(x.into()),
                }
            }
            None => Err(WorkEndError::NoWorkToComplete),
        }
    }

    /// Tries to delete a project, but can fail if the project has already been deleted.
    /// Returns true if the project deletion is successful, or false otherwise.
    pub fn delete_project(&mut self, id: ProjectId) -> bool {
        let index = self
            .all_project_ids()
            .enumerate()
            .find(|(_, project_id)| *project_id == id)
            .map(|(i, _)| i);
        match index {
            Some(i) => {
                self.changes.push(Change::ProjectDeleted {
                    id: unsafe { id.inner() },
                });
                self.projects.swap_remove(i);
                true
            }
            None => false,
        }
    }

    /// Tries to delete a work slice from its project, but fails if the work slice has been deleted,
    /// or if its project has been deleted. Returns true if it succeeds.
    pub fn delete_work_slice(&mut self, id: WorkSliceId) -> bool {
        match self.project_id_from_work_slice(id) {
            Some(project_id) => self.delete_work_slice_from_project(project_id, id),
            None => false,
        }
    }

    /// Tries to delete a work slice from the specified project, but fails if the work slice has been deleted,
    /// or if it is not part of the project, or if the project has already been deleted.
    pub fn delete_work_slice_from_project(
        &mut self,
        project_id: ProjectId,
        work_slice_id: WorkSliceId,
    ) -> bool {
        match self
            .project_from_id_mut(project_id)
            .map(|project| project.delete_work_slice(work_slice_id))
        {
            Some(true) => {
                self.changes.push(Change::WorkSliceDeleted {
                    project_id: unsafe { project_id.inner() },
                    work_slice_id: unsafe { work_slice_id.inner() },
                });
                true
            }
            None | Some(false) => false,
        }
    }
}
impl State {
    /// tries to return the work slice from its ID (complete or incomplete)
    /// but fails if it has been deleted or if its project has been deleted.
    pub fn work_slice_from_id(&self, id: WorkSliceId) -> Option<WorkSlice<'_>> {
        self.projects.iter().find_map(|x| x.work_slice_from_id(id))
    }
}
impl State {
    pub unsafe fn handle_changes(&mut self) -> Vec<Change> {
        std::mem::replace(&mut self.changes, vec![])
    }
}
