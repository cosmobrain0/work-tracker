use std::fmt::Display;

use chrono::{DateTime, Utc};

use crate::{
    state::payment::{MoneyExact, Payment},
    state::work_slice::{CompleteWorkSlice, IncompleteWorkSlice, WorkSlice, WorkSliceId},
};

use super::{LocalCompleteWorkSlice, LocalIncompleteWorkSlice, WorkStartError};

type Error = Box<dyn std::error::Error + Send + Sync>;

pub trait Project {
    async fn name(&self) -> Result<String, Error>;
    async fn description(&self) -> Result<String, Error>;
    async fn complete_work_slice_ids(&self) -> Result<Vec<WorkSliceId>, Error>;
    async fn complete_work_slices(&self) -> Result<Vec<Box<dyn CompleteWorkSlice>>, Error>;
    async fn incomplete_work_slice_id(&self) -> Result<Option<WorkSliceId>, Error>;
    async fn incomplete_work_slice(&self) -> Result<Option<Box<dyn IncompleteWorkSlice>>, Error>;
    async fn project_id(&self) -> Result<ProjectId, Error>;
    async fn start_work(
        &self,
        start: DateTime<Utc>,
        payment: Payment,
        id: WorkSliceId,
    ) -> Result<(), Error>;
    async fn complete_work(&self, end: DateTime<Utc>) -> Result<WorkSliceId, Error>;
    async fn start_work_now(&self, payment: Payment, id: WorkSliceId) -> Result<(), Error> {
        self.start_work(Utc::now(), payment).await
    }
    async fn complete_work_now(&self) -> Result<WorkSliceId, Error> {
        self.complete_work(Utc::now()).await
    }
    async fn delete_work_slice(&mut self, work_slice_id: WorkSliceId) -> Result<WorkSlice, Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteWorkError {
    NoWorkToComplete,
    EndTimeTooEarly,
}
impl Display for CompleteWorkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl std::error::Error for CompleteWorkError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidWorkSliceId;
impl Display for InvalidWorkSliceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl std::error::Error for InvalidWorkSliceId {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectId(pub(super) u64);
impl ProjectId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug)]
pub struct LocalProject {
    name: String,
    description: String,
    work_slices: Vec<Box<dyn CompleteWorkSlice>>,
    current_slice: Option<Box<dyn IncompleteWorkSlice>>,
    id: ProjectId,
    previous_work_slice_id: WorkSliceId,
}
impl PartialEq for LocalProject {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for LocalProject {}
impl Project for LocalProject {
    async fn name(&self) -> Result<String, Error> {
        Ok(self.name.to_string())
    }

    async fn description(&self) -> Result<String, Error> {
        Ok(self.description.to_string())
    }

    async fn complete_work_slice_ids(&self) -> Result<Vec<WorkSliceId>, Error> {
        Ok(self.work_slices.iter().map(|x| x.id()).collect())
    }

    async fn complete_work_slices(&self) -> Result<Vec<Box<dyn CompleteWorkSlice>>, Error> {
        Ok(self.work_slices.iter().cloned().collect())
    }

    async fn incomplete_work_slice_id(&self) -> Result<Option<WorkSliceId>, Error> {
        Ok(self.current_slice.map(|x| x.id()))
    }

    async fn incomplete_work_slice(&self) -> Result<Option<Box<dyn IncompleteWorkSlice>>, Error> {
        Ok(self.current_slice.as_ref().map(Box::clone))
    }

    async fn project_id(&self) -> Result<ProjectId, Error> {
        Ok(self.id)
    }

    async fn start_work(
        &mut self,
        start: DateTime<Utc>,
        payment: Payment,
        id: WorkSliceId,
    ) -> Result<(), Error> {
        if self.current_slice.is_none() {
            if let Some(incomplete_work) = LocalIncompleteWorkSlice::new(start, payment, id) {
                self.current_slice = Some(Box::new(incomplete_work) as Error);
                Ok(())
            } else {
                Err(Box::new(WorkStartError::InvalidStartTime))
            }
        } else {
            Err(Box::new(WorkStartError::AlreadyStarted))
        }
    }

    fn complete_work(&mut self, end: DateTime<Utc>) -> Result<WorkSliceId, Error> {
        match self.current_slice.take() {
            Some(current_work) => match current_work.complete(end) {
                WorkSlice::Complete(complete) => {
                    let id = complete.id();
                    self.work_slices.push(complete);
                    self.current_slice = None;
                    Ok(id)
                }
                WorkSlice::Incomplete(incomplete) => {
                    self.current_slice = Some(incomplete);
                    Err(Box::new(CompleteWorkError::EndTimeTooEarly as Error))
                }
            },
            None => Err(Box::new(CompleteWorkError::NoWorkToComplete as Error)),
        }
    }

    fn complete_work_now(&mut self) -> Result<WorkSliceId, Error> {
        match self.current_slice.take() {
            Some(x) => {
                self.work_slices.push(x.complete_now());
                Ok(self.work_slices[self.work_slices.len() - 1].id())
            }
            None => Err(()),
        }
    }

    fn delete_work_slice(&mut self, work_slice_id: WorkSliceId) -> Result<WorkSlice, Error> {
        if self
            .current_slice
            .as_ref()
            .is_some_and(|x| x.id() == work_slice_id)
        {
            Ok(WorkSlice::Incomplete(
                Box::new(self.current_slice.take().unwrap()) as Error,
            ))
        } else {
            match self
                .work_slices
                .iter()
                .enumerate()
                .find(|(i, x)| x.id() == work_slice_id)
                .map(|(i, x)| i)
            {
                Some(i) => Ok(Box::new(WorkSlice::Complete(self.work_slices.remove(i))) as Error),
                None => Err(Box::new(InvalidWorkSliceId) as Error),
            }
        }
    }
}
impl LocalProject {
    pub fn new(name: String, description: String, id: ProjectId) -> Self {
        Self {
            name,
            description,
            id,
            work_slices: Vec::new(),
            current_slice: None,
            previous_work_slice_id: 0,
        }
    }

    pub fn payment(&self) -> MoneyExact {
        self.work_slices
            .iter()
            .map(|slice| slice.calculate_payment())
            .sum()
    }

    pub fn new_work_slice_id(&self) -> WorkSliceId {
        let new_id = WorkSliceId::new(self.previous_work_slice_id.0 + 1);
        self.previous_work_slice_id = new_id;
        new_id
    }
}

#[cfg(test)]
mod tests {
    use std::{env, thread, time::Duration};

    use chrono::{TimeDelta, Utc};
    use dotenvy::dotenv;
    use tokio_postgres::{Client, NoTls};

    use crate::state::{LocalProject, Money, NotFoundError, Payment, ProjectId, State};

    #[tokio::test]
    async fn get_test_client() -> Client {
        dotenv().expect("Couldn't load .env!");
        let password = env::var("TESTPASSWORD").expect("Couldn't get the password from .env!");
        let host = env::var("TESTHOST").expect("Couldn't get the host from .env!");
        let user = env::var("TESTUSER").expect("Couldn't get the user from .env!");
        let dbname = env::var("TESTDBNAME").expect("Couldn't get the dbname from .env!");
        let (client, connection) = tokio_postgres::connect(
            format!("host={host} user={user} password={password} dbname={dbname}").as_str(),
            NoTls,
        )
        .await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection nerror: {}", e);
            }
        });
        client
    }

    #[test]
    fn project_equality() {
        let tests = [
            LocalProject::new(
                "hello".to_string(),
                "this is a test".to_string(),
                ProjectId::new(1),
            ),
            LocalProject::new(
                "hi".to_string(),
                "this is a test".to_string(),
                ProjectId::new(2),
            ),
        ];
        assert_eq!(tests[0], tests[0]);
        assert_ne!(tests[0], tests[1]);
        assert_ne!(tests[1], tests[0]);
    }

    #[test]
    fn state_creates_many_projects() {
        let mut state = State::new(get_test_client());
        for i in 0..10000 {
            state.create_project(
                String::from("Example Project"),
                String::from("Example description!"),
            );
        }
    }

    #[tokio::test]
    async fn state_create_single_project() {
        let mut state = State::new(get_test_client());
        let id = state.create_project(
            String::from("Example Project"),
            String::from("Example Description"),
        );

        let Ok(()) = state
            .start_work_now(Payment::Hourly(Money::new(800)), id)
            .await
        else {
            panic!("Couldn't start work!");
        };

        let Err(WorkStartError) = state
            .start_work_now(Payment::Hourly(Money::new(500)), id)
            .await
        else {
            panic!("Shouldn't be able to start work!");
        };

        let Ok((completed, Some(_))) = state.work_slices(id) else {
            panic!("There should be some current work!");
        };
        assert!(completed.len() == 0);

        thread::sleep(Duration::from_millis(5000));

        let Ok(()) = state.end_work_now(id) else {
            panic!("Should be able to end work!");
        };

        let Ok((completed, None)) = state.work_slices(id) else {
            panic!("there shouldn't be any work!");
        };
        assert_eq!(completed.len(), 1);
        assert!(completed[0].duration() >= TimeDelta::milliseconds(5000));
    }

    #[test]
    fn state_delete_project() {
        let now = Utc::now();

        let mut state = State::new(get_test_client());
        let project_0 = state.create_project(
            String::from("Project 0"),
            String::from("The first project, which will have two work slices."),
        );
        let project_1 = state.create_project(
            String::from("Project 1"),
            String::from(
                "The second project, which will have one work slice, and then be deleted.",
            ),
        );
        let project_2 = state.create_project(
            String::from("Project 2"),
            String::from(
                "The third project, which will have two work slices, one of which will be deleted.",
            ),
        );

        state
            .start_work(
                now - TimeDelta::seconds(2 * 60 * 60),
                Payment::Fixed(Money::new(4000)),
                project_0,
            )
            .unwrap();
        state
            .end_work(now - TimeDelta::seconds(60 * 60), project_0)
            .unwrap();
        state
            .start_work(
                now - TimeDelta::seconds(30 * 60),
                Payment::Hourly(Money::new(500)),
                project_0,
            )
            .unwrap();
        state.end_work_now(project_0).unwrap();

        state
            .start_work(
                now - TimeDelta::seconds(3 * 60 * 60),
                Payment::Hourly(Money::new(2000)),
                project_1,
            )
            .unwrap();
        state.end_work_now(project_1).unwrap();

        state
            .start_work(
                now - TimeDelta::seconds(5 * 60 * 60),
                Payment::Fixed(Money::new(5000)),
                project_2,
            )
            .unwrap();
        state
            .end_work(now - TimeDelta::seconds(4 * 60 * 60), project_2)
            .unwrap();

        let projects = state.projects();
        assert_eq!(projects.len(), 3);

        let project_0_work = state.work_slices(project_0).unwrap();
        assert_eq!(project_0_work.1, None);
        assert_eq!(project_0_work.0.len(), 2);

        let project_1_work = state.work_slices(project_1).unwrap();
        assert_eq!(project_1_work.1, None);
        assert_eq!(project_1_work.0.len(), 1);

        let project_2_work = state.work_slices(project_2).unwrap();
        assert_eq!(project_2_work.1, None);
        assert_eq!(project_2_work.0.len(), 1);

        state.delete_project(project_1).unwrap();
        assert_eq!(state.projects().len(), 2);
        assert!(matches!(
            state.work_slices(project_1),
            Err(InvalidProjectId)
        ));
        assert!(matches!(state.work_slices(project_0), Ok((_, None))));
        assert!(matches!(state.work_slices(project_2), Ok((_, None))));

        state
            .delete_work_slice_from_project(
                project_0,
                state.work_slices(project_0).unwrap().0[0].id(),
            )
            .unwrap();
        assert!(matches!(
            state.delete_work_slice_from_project(
                project_0,
                state.work_slices(project_2).unwrap().0[0].id()
            ),
            Err(NotFoundError::WorkSliceNotFound)
        ));
        state
            .delete_work_slice_from_project(
                project_2,
                state.work_slices(project_2).unwrap().0[0].id(),
            )
            .unwrap();

        let project_0_work = state.work_slices(project_0).unwrap();
        let project_2_work = state.work_slices(project_2).unwrap();
        assert_eq!(
            project_0_work.0[0].start(),
            now - TimeDelta::seconds(30 * 60)
        );
        assert!(project_2_work.0.is_empty());
        assert_eq!(project_0_work.0.len(), 1);
    }
}
