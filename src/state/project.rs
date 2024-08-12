use std::{error::Error, fmt::Display};

use chrono::{DateTime, Utc};

use crate::{
    state::payment::{MoneyExact, Payment},
    state::work_slice::{CompleteWorkSlice, IncompleteWorkSlice, WorkSlice, WorkSliceId},
};

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
impl Error for CompleteWorkError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectId(u64);
impl ProjectId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug)]
pub struct Project {
    name: String,
    description: String,
    work_slices: Vec<CompleteWorkSlice>,
    current_slice: Option<IncompleteWorkSlice>,
    id: ProjectId,
}
impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Project {}
impl Project {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &String {
        &self.description
    }
    pub fn work_slices(&self) -> Vec<&CompleteWorkSlice> {
        self.work_slices.iter().collect()
    }
    pub fn current_slice(&self) -> Option<&IncompleteWorkSlice> {
        self.current_slice.as_ref()
    }
    pub fn id(&self) -> ProjectId {
        self.id
    }
}
impl Project {
    pub fn new(name: String, description: String, id: ProjectId) -> Self {
        Self {
            name,
            description,
            id,
            work_slices: Vec::new(),
            current_slice: None,
        }
    }

    pub fn complete_work_slices(&self) -> Vec<&CompleteWorkSlice> {
        self.work_slices.iter().collect()
    }

    pub fn current_work_slice(&self) -> Option<&IncompleteWorkSlice> {
        self.current_slice.as_ref()
    }

    pub fn add_slice(&mut self, work_slice: CompleteWorkSlice) {
        self.work_slices.push(work_slice);
    }

    pub fn start_work(&mut self, current_work: IncompleteWorkSlice) -> Result<(), ()> {
        if self.current_slice.is_none() {
            self.current_slice = Some(current_work);
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn start_work_now(&mut self, payment: Payment, id: WorkSliceId) -> Result<(), ()> {
        if self.current_slice.is_none() {
            self.current_slice = Some(IncompleteWorkSlice::new(Utc::now(), payment, id).unwrap());
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn complete_work(&mut self, end: DateTime<Utc>) -> Result<(), CompleteWorkError> {
        match self.current_slice.take() {
            Some(current_work) => match current_work.complete(end) {
                WorkSlice::Complete(complete) => {
                    self.work_slices.push(complete);
                    self.current_slice = None;
                    Ok(())
                }
                WorkSlice::Incomplete(incomplete) => {
                    self.current_slice = Some(incomplete);
                    Err(CompleteWorkError::EndTimeTooEarly)
                }
            },
            None => Err(CompleteWorkError::NoWorkToComplete),
        }
    }

    pub fn complete_work_now(&mut self) -> Result<(), ()> {
        match self.current_slice.take() {
            Some(x) => {
                self.work_slices.push(x.complete_now());
                Ok(())
            }
            None => Err(()),
        }
    }

    pub fn payment(&self) -> MoneyExact {
        self.work_slices
            .iter()
            .map(|slice| slice.calculate_payment())
            .sum()
    }

    pub fn delete_work_slice(&mut self, work_slice_id: WorkSliceId) -> Result<WorkSlice, ()> {
        if self
            .current_slice
            .as_ref()
            .is_some_and(|x| x.id() == work_slice_id)
        {
            Ok(WorkSlice::Incomplete(self.current_slice.take().unwrap()))
        } else {
            match self
                .work_slices
                .iter()
                .enumerate()
                .find(|(i, x)| x.id() == work_slice_id)
                .map(|(i, x)| i)
            {
                Some(i) => Ok(WorkSlice::Complete(self.work_slices.remove(i))),
                None => Err(()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use chrono::{TimeDelta, Utc};
    use tokio_postgres::Client;

    use crate::state::{Money, NotFoundError, Payment, Project, ProjectId, State};

    fn get_test_client() -> Client {
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
            Project::new(
                "hello".to_string(),
                "this is a test".to_string(),
                ProjectId::new(1),
            ),
            Project::new(
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

    #[test]
    fn state_create_single_project() {
        let mut state = State::new(get_test_client());
        let id = state.create_project(
            String::from("Example Project"),
            String::from("Example Description"),
        );

        let Ok(()) = state.start_work_now(Payment::Hourly(Money::new(800)), id) else {
            panic!("Couldn't start work!");
        };

        let Err(WorkStartError) = state.start_work_now(Payment::Hourly(Money::new(500)), id) else {
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
