use std::{error::Error, fmt::Display};

mod payment;
mod project;
mod work_slice;

use chrono::{DateTime, Utc};
pub use payment::*;
pub use project::*;
use tokio_postgres::Client;
pub use work_slice::*;

#[derive(Debug)]
pub struct State {
    client: Client,
}
impl State {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn projects(&self) -> Result<Vec<ProjectId>, Box<dyn Error + Send + Sync>> {
        let rows = self
            .client
            .query("SELECT project_id FROM project", &[])
            .await?;
        Ok(rows
            .into_iter()
            .map(|x| x.get(0))
            .map(|x: i32| ProjectId::new(x as u64))
            .collect())
    }

    pub async fn start_work_now(
        &mut self,
        payment: Payment,
        id: ProjectId,
    ) -> Result<(), WorkStartNowError> {
        // check if the project exists
        let project_exists = self
            .project_exists(id)
            .await
            .map_err(|_| WorkStartNowError::DatabaseError)?;

        // if it doesn't, ERROR: invalid project ID
        if !project_exists {
            return Err(WorkStartNowError::InvalidProjectId);
        }

        // check if the project already has an incomplete work slice attached to it
        let project_has_incomplete_work_slice: bool = self.client.query_one(
            "SELECT EXISTS(SELECT 1 FROM work_slice WHERE completion IS NULL AND project_id = $1)",
            &[&(id.0 as i32)],
        ).await.map_err(|_| WorkStartNowError::DatabaseError)?.get(0);

        // if it does, ERROR: already started work
        if project_has_incomplete_work_slice {
            return Err(WorkStartNowError::AlreadyStarted);
        }

        // create a new incomplete work slice and add it to the work_slice table
        let now = Utc::now();
        let _ = self
            .client
            .query(
                "INSERT INTO work_slice (start, payment, project_id) VALUES ($1, ROW($2, $3), $4)",
                &[
                    &now,
                    &payment.is_hourly(),
                    &payment.rate().0,
                    &(id.0 as i32),
                ],
            )
            .await
            .map_err(|_| WorkStartNowError::DatabaseError)?;
        Ok(())
    }

    async fn project_exists(&mut self, id: ProjectId) -> Result<bool, tokio_postgres::Error> {
        self.client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM project WHERE project_id=$1)",
                &[&(id.0 as i32)],
            )
            .await
            .map(|x| x.get(0))
    }

    pub async fn end_work_now(&mut self, id: ProjectId) -> Result<(), WorkEndNowError> {
        // check if the project exists
        let project_exists = self
            .project_exists(id)
            .await
            .map_err(|_| WorkEndNowError::DatabaseError)?;
        // if the project doesn't exist, ERROR: invalid project ID
        if !project_exists {
            return Err(WorkEndNowError::InvalidProjectId);
        }
        // check if there is any current work, getting its ID
        let incomplete_work_id = self
            .client
            .query_opt(
                "SELECT work_id FROM work_slice WHERE completion IS NULL AND project_id = $1",
                &[&(id.0 as i32)],
            )
            .await
            .map_err(|_| WorkEndNowError::DatabaseError)?
            .map(|x| x.get(0))
            .map(|x: i32| WorkSliceId::new(x as u64));

        // if there isn't, ERROR: no current work
        if incomplete_work_id.is_none() {
            return Err(WorkEndNowError::NoCurrentWork);
        }
        let incomplete_work_id = incomplete_work_id.unwrap();
        // if there is, end it now
        let now = Utc::now();
        self.client
            .query(
                "UPDATE work_slice SET completion = $1 WHERE work_id = $2",
                &[&now, &(incomplete_work_id.0 as i32)],
            )
            .await
            .map_err(|_| WorkEndNowError::DatabaseError)
            .map(|_| ())
    }

    pub fn start_work(
        &mut self,
        start: DateTime<Utc>,
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

    pub fn end_work(&mut self, end: DateTime<Utc>, id: ProjectId) -> Result<(), WorkEndError> {
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
