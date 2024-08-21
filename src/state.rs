mod errors;
mod payment;
mod project;
mod work_slice;

use chrono::{DateTime, Utc};
pub use errors::*;
pub use payment::*;
pub use project::*;
pub use work_slice::*;

#[derive(Debug)]
pub struct State {
    previous_project_id: u64,
    previous_work_slice_id: u64,
    projects: Vec<Project>,
}
impl State {
    pub fn new() -> Self {
        Self {
            previous_project_id: 0,
            previous_work_slice_id: 0,
            projects: Vec::new(),
        }
    }

    pub fn new_project(&mut self, name: String, description: String) -> ProjectId {
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
    pub fn all_project_ids<'a>(&'a self) -> impl Iterator<Item = ProjectId> + 'a {
        self.projects.iter().map(|x| x.id())
    }

    pub fn project_from_id(&self, id: ProjectId) -> Option<&Project> {
        self.projects.iter().find(|x| x.id() == id)
    }

    fn project_from_id_mut(&mut self, id: ProjectId) -> Option<&mut Project> {
        self.projects.iter_mut().find(|x| x.id() == id)
    }

    pub fn all_projects<'a>(&'a self) -> impl Iterator<Item = &Project> + 'a {
        self.projects.iter()
    }

    fn all_projects_mut<'a>(&'a mut self) -> impl Iterator<Item = &mut Project> + 'a {
        self.projects.iter_mut()
    }

    pub fn project_exists(&self, id: ProjectId) -> bool {
        self.all_project_ids().any(|x| x == id)
    }

    pub fn project_id_from_work_slice(&self, work_slice_id: WorkSliceId) -> Option<ProjectId> {
        self.all_projects()
            .map(|x| {
                (
                    x.id(),
                    x.complete_work_slices().any(|x| x.id() == work_slice_id)
                        || x.current_slice().is_some_and(|x| x.id() == work_slice_id),
                )
            })
            .find(|(id, found)| *found)
            .map(|(id, _)| id)
    }
}
impl State {
    pub fn start_work(
        &mut self,
        id: ProjectId,
        payment: Payment,
        time: DateTime<Utc>,
    ) -> Result<(), WorkStartError> {
        match IncompleteWorkSlice::new(time, payment, self.new_work_slice_id()) {
            Some(work_slice) => match self.project_from_id_mut(id) {
                Some(project) => project
                    .start_work(work_slice)
                    .map_err(|_| WorkStartError::AlreadyStarted),
                None => Err(WorkStartError::InvalidProjectId),
            },
            None => Err(WorkStartError::InvalidStartTime),
        }
    }

    pub fn end_work(&mut self, id: ProjectId, time: DateTime<Utc>) -> Result<(), WorkEndError> {
        match self.project_from_id_mut(id) {
            Some(project) => project.complete_work(time).map_err(Into::into),
            None => Err(WorkEndError::NoWorkToComplete),
        }
    }

    pub fn delete_project(&mut self, id: ProjectId) -> bool {
        let index = self
            .all_project_ids()
            .enumerate()
            .find(|(i, project_id)| *project_id == id)
            .map(|(i, _)| i);
        match index {
            Some(i) => {
                self.projects.swap_remove(i);
                true
            }
            None => false,
        }
    }

    pub fn delete_work_slice(&mut self, id: WorkSliceId) -> bool {
        match self.project_id_from_work_slice(id) {
            Some(project_id) => self.delete_work_slice_from_project(project_id, id),
            None => false,
        }
    }

    pub fn delete_work_slice_from_project(
        &mut self,
        project_id: ProjectId,
        work_slice_id: WorkSliceId,
    ) -> bool {
        self.project_from_id_mut(project_id)
            .map(|project| project.delete_work_slice(work_slice_id).ok())
            .flatten()
            .is_some()
    }
}
