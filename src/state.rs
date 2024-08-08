use std::{error::Error, fmt::Display, time::Instant};

use crate::{
    payment::Payment,
    project::{CompleteWorkError, Project, ProjectId},
    work_slice::{CompleteWorkSlice, IncompleteWorkSlice, WorkSliceId},
};

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

    pub fn start_work_now(
        &mut self,
        payment: Payment,
        id: ProjectId,
    ) -> Result<(), WorkAlreadyStartedError> {
        let work_id = self.create_work_slice_id();
        let project = self.get_project_mut(id);
        project
            .start_work_now(payment, work_id)
            .map_err(|_| WorkAlreadyStartedError)
    }

    pub fn end_work_now(&mut self, id: ProjectId) -> Result<(), NoCurrentWorkError> {
        let project = self.get_project_mut(id);
        project.complete_work_now().map_err(|_| NoCurrentWorkError)
    }

    pub fn start_work(
        &mut self,
        start: Instant,
        payment: Payment,
        id: ProjectId,
    ) -> Result<(), ()> {
        let work_id = self.create_work_slice_id();
        let project = self.get_project_mut(id);
        let Some(work) = IncompleteWorkSlice::new(start, payment, work_id) else {
            return Err(());
        };
        project.start_work(work)
    }

    pub fn end_work(&mut self, end: Instant, id: ProjectId) -> Result<(), CompleteWorkError> {
        let project = self.get_project_mut(id);
        project.complete_work(end)
    }

    pub fn work_slices(
        &self,
        id: ProjectId,
    ) -> (Vec<&CompleteWorkSlice>, Option<&IncompleteWorkSlice>) {
        let project = self.get_project(id);
        (project.complete_work_slices(), project.current_work_slice())
    }

    fn create_work_slice_id(&mut self) -> WorkSliceId {
        if self.previous_work_slice_id == u64::MAX {
            panic!("Can't generate a new work slice id!");
        }
        let id = self.previous_work_slice_id + 1;
        self.previous_work_slice_id = id;

        WorkSliceId::new(id)
    }

    pub fn create_project(&mut self, name: String, description: String) -> ProjectId {
        if self.previous_project_id == u64::MAX {
            panic!("Can't generate a new project id!");
        }

        let id = self.previous_project_id + 1;
        let project = Project::new(name, description, ProjectId::new(id));
        self.projects.push(project);

        self.previous_project_id = id;
        ProjectId::new(id)
    }

    fn get_project_mut(&mut self, id: ProjectId) -> &mut Project {
        self.projects.iter_mut().find(|x| x.id() == id).unwrap()
    }

    fn get_project(&self, id: ProjectId) -> &Project {
        self.projects.iter().find(|x| x.id() == id).unwrap()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WorkAlreadyStartedError;
impl Display for WorkAlreadyStartedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl Error for WorkAlreadyStartedError {}

#[derive(Debug, Clone, Copy)]
pub struct NoCurrentWorkError;
impl Display for NoCurrentWorkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl Error for NoCurrentWorkError {}
