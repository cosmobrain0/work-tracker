use std::{error::Error, fmt::Display};

mod config;
mod payment;
mod project;
mod work_slice;

use chrono::{DateTime, Utc};
pub use payment::*;
pub use project::*;
use tokio_postgres::Client;
pub use work_slice::*;

use self::config::Config;

#[derive(Debug)]
pub struct State<T: Config> {
    config: T,
}
impl<T: Config> State<T> {
    pub fn new(config: T) -> Self {
        Self { config }
    }

    pub async fn projects(&self) -> Result<Vec<ProjectId>, Box<dyn Error + Send + Sync>> {
        self.config.project_ids().await
    }

    pub async fn start_work_now(
        &mut self,
        payment: Payment,
        id: ProjectId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        match self.config.project_from_id(id).await? {
            Some(project) => {
                project
                    .start_work_now(payment, self.config.new_work_slice_id())
                    .await?
            }
            None => Err(Box::new(WorkStartNowError::InvalidProjectId)),
        }
    }

    async fn project_exists(
        &mut self,
        id: ProjectId,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        Ok(self
            .config
            .project_ids()
            .await?
            .iter()
            .find(|x| x.id() == id)
            .is_some())
    }

    pub async fn end_work_now(
        &mut self,
        id: ProjectId,
    ) -> Result<WorkSliceId, Box<dyn Error + Send + Sync>> {
        match self.config.project_from_id(id).await? {
            Some(project) => project.complete_work_now().await,
            None => Err(Box::new(InvalidProjectId) as Error),
        }
    }

    pub async fn start_work(
        &mut self,
        start: DateTime<Utc>,
        payment: Payment,
        id: ProjectId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let work_id = self.config.new_work_slice_id();
        match self
            .config
            .project_from_id(id)
            .await?
            .map(|x| x.start_work(start, payment, id))
        {
            Some(x) => x.await,
            None => Err(Box::new(InvalidProjectId) as Box<dyn Error + Send + Sync>),
        }
    }

    pub async fn end_work(
        &mut self,
        id: ProjectId,
        end: DateTime<Utc>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        match self.config.project_from_id(id).await? {
            Some(project) => project.complete_work(end).await,
            None => Err(Box::new(InvalidProjectId) as Error),
        }
    }

    pub async fn work_slices(
        &self,
        id: ProjectId,
    ) -> Result<
        (
            Vec<&dyn CompleteWorkSlice>,
            Option<&dyn IncompleteWorkSlice>,
        ),
        Box<dyn Error + Send + Sync>,
    > {
        match self.config.project_from_id(id).await? {
            Some(project) => (
                project.complete_work_slices().await?,
                project.incomplete_work_slice().await?,
            ),
            None => InvalidProjectId,
        }
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

    pub async fn delete_project(
        &mut self,
        project_id: ProjectId,
    ) -> Result<bool, InvalidProjectId> {
        self.config.remove_project(project_id).await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkStartNowError {
    AlreadyStarted,
    InvalidProjectId,
    DatabaseError,
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
    DatabaseError,
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
