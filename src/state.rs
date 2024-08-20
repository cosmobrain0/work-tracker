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
}
impl State {
    pub fn delete_project(&mut self, id: ProjectId) -> bool {
        match self
            .projects
            .iter()
            .enumerate()
            .find(|(_, x)| x.id() == id)
            .map(|(i, _)| i)
        {
            Some(i) => {
                self.projects.swap_remove(i);
                true
            }
            None => false,
        }
    }
}
impl State {
    pub fn all_project_ids<'a>(&'a self) -> impl Iterator<Item = ProjectId> + 'a {
        self.projects.iter().map(|x| x.id())
    }

    pub fn project_from_id(&self, id: ProjectId) -> Option<&Project> {
        self.projects.iter().find(|x| x.id() == id)
    }

    pub fn all_projects<'a>(&'a self) -> impl Iterator<Item = &Project> + 'a {
        self.projects.iter()
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
