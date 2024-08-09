use std::{error::Error, fmt::Display, time::Instant};

use crate::{
    payment::Payment,
    project::{CompleteWorkError, Project, ProjectId},
    work_slice::{CompleteWorkSlice, IncompleteWorkSlice, WorkSlice, WorkSliceId},
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
    ) -> Result<(), WorkStartNowError> {
        let work_id = self.create_work_slice_id();
        let Some(project) = self.get_project_mut(id) else {
            return Err(WorkStartNowError::InvalidProjectId);
        };
        project
            .start_work_now(payment, work_id)
            .map_err(|_| WorkStartNowError::AlreadyStarted)
    }

    pub fn end_work_now(&mut self, id: ProjectId) -> Result<(), WorkEndNowError> {
        let Some(project) = self.get_project_mut(id) else {
            return Err(WorkEndNowError::NoCurrentWork);
        };
        project
            .complete_work_now()
            .map_err(|_| WorkEndNowError::NoCurrentWork)
    }

    pub fn start_work(
        &mut self,
        start: Instant,
        payment: Payment,
        id: ProjectId,
    ) -> Result<(), WorkStartError> {
        let work_id = self.create_work_slice_id();
        let Some(project) = self.get_project_mut(id) else {
            return Err(WorkStartError::InvalidProjectId);
        };
        let Some(work) = IncompleteWorkSlice::new(start, payment, work_id) else {
            return Err(WorkStartError::InvalidStartTime);
        };
        project
            .start_work(work)
            .map_err(|_| WorkStartError::AlreadyStarted)
    }

    pub fn end_work(&mut self, end: Instant, id: ProjectId) -> Result<(), WorkEndError> {
        let Some(project) = self.get_project_mut(id) else {
            return Err(WorkEndError::InvalidProjectId);
        };
        project.complete_work(end).map_err(|e| match e {
            CompleteWorkError::NoWorkToComplete => WorkEndError::NoWorkToComplete,
            CompleteWorkError::EndTimeTooEarly => WorkEndError::EndTimeTooEarly,
        })
    }

    pub fn work_slices(
        &self,
        id: ProjectId,
    ) -> Result<(Vec<&CompleteWorkSlice>, Option<&IncompleteWorkSlice>), InvalidProjectId> {
        let Some(project) = self.get_project(id) else {
            return Err(InvalidProjectId);
        };
        Ok((project.complete_work_slices(), project.current_work_slice()))
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

    fn get_project_mut(&mut self, id: ProjectId) -> Option<&mut Project> {
        self.projects.iter_mut().find(|x| x.id() == id)
    }

    fn get_project(&self, id: ProjectId) -> Option<&Project> {
        self.projects.iter().find(|x| x.id() == id)
    }

    pub fn delete_project(&mut self, project_id: ProjectId) -> Result<Project, InvalidProjectId> {
        match self
            .projects
            .iter()
            .enumerate()
            .find(|(i, x)| x.id() == project_id)
            .map(|(i, _)| i)
        {
            Some(i) => Ok(self.projects.remove(i)),
            None => Err(InvalidProjectId),
        }
    }

    pub fn delete_work_slice_from_project(
        &mut self,
        project_id: ProjectId,
        work_slice_id: WorkSliceId,
    ) -> Result<WorkSlice, NotFoundError> {
        let Some(project) = self.get_project_mut(project_id) else {
            return Err(NotFoundError::ProjectNotFound);
        };
        project
            .delete_work_slice(work_slice_id)
            .map_err(|_| NotFoundError::WorkSliceNotFound)
    }

    pub fn delete_work_slice(
        &mut self,
        work_slice_id: WorkSliceId,
    ) -> Result<WorkSlice, WorkSliceNotFoundError> {
        for project_id in self.projects.iter().map(|x| x.id()).collect::<Vec<_>>() {
            match self.delete_work_slice_from_project(project_id, work_slice_id) {
                Ok(slice) => {
                    return Ok(slice);
                }
                Err(_) => (),
            }
        }
        Err(WorkSliceNotFoundError)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkStartNowError {
    AlreadyStarted,
    InvalidProjectId,
}
impl Display for WorkStartNowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl Error for WorkStartNowError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkEndNowError {
    NoCurrentWork,
    InvalidProjectId,
}
impl Display for WorkEndNowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl Error for WorkEndNowError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotFoundError {
    ProjectNotFound,
    WorkSliceNotFound,
}
impl Display for NotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl Error for NotFoundError {}

#[derive(Debug, Clone, Copy)]
pub struct WorkSliceNotFoundError;
impl Display for WorkSliceNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl Error for WorkSliceNotFoundError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkStartError {
    AlreadyStarted,
    InvalidProjectId,
    InvalidStartTime,
}
impl Display for WorkStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl Error for WorkStartError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkEndError {
    EndTimeTooEarly,
    NoWorkToComplete,
    InvalidProjectId,
}
impl Display for WorkEndError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl Error for WorkEndError {}

#[derive(Debug, Clone, Copy)]
pub struct InvalidProjectId;
impl Display for InvalidProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl Error for InvalidProjectId {}
