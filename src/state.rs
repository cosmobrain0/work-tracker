use std::time::Instant;

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

    pub fn process_message<'a>(&'a mut self, message: IncomingMessage) -> OutgoingMessage<'a> {
        match message {
            IncomingMessage::CreateProject { name, description } => {
                self.create_project(name, description);
                OutgoingMessage::ProjectCreated
            }
            IncomingMessage::StartWorkNow(id, payment) => {
                let work_id = self.create_work_slice_id();
                let project = self.get_project(id);
                match project.start_work_now(payment, work_id) {
                    Ok(()) => OutgoingMessage::WorkStarted,
                    Err(()) => OutgoingMessage::AlreadyStartedWork,
                }
            }
            IncomingMessage::EndWorkNow(id) => {
                let project = self.get_project(id);
                match project.complete_work_now() {
                    Ok(()) => OutgoingMessage::WorkEnded,
                    Err(()) => OutgoingMessage::NoCurrentWork,
                }
            }
            IncomingMessage::StartWork(id, payment, start) => {
                let work_id = self.create_work_slice_id();
                let project = self.get_project(id);
                let Some(work) = IncompleteWorkSlice::new(start, payment, work_id) else {
                    return OutgoingMessage::WorkStartTimeAfterNow;
                };
                let Ok(()) = project.start_work(work) else {
                    return OutgoingMessage::AlreadyStartedWork;
                };
                OutgoingMessage::WorkStarted
            }
            IncomingMessage::EndWork(id, end) => {
                let project = self.get_project(id);
                match project.complete_work(end) {
                    Ok(()) => OutgoingMessage::WorkEnded,
                    Err(CompleteWorkError::NoWorkToComplete) => OutgoingMessage::NoCurrentWork,
                    Err(CompleteWorkError::EndTimeTooEarly) => {
                        OutgoingMessage::WorkEndTimeBeforeStartTime
                    }
                }
            }
            IncomingMessage::GetWorkSlices(id) => {
                let project = self.get_project(id);
                OutgoingMessage::WorkSlices(
                    project.complete_work_slices(),
                    project.current_work_slice(),
                )
            }
        }
    }

    fn create_work_slice_id(&mut self) -> WorkSliceId {
        if self.previous_work_slice_id == u64::MAX {
            panic!("Can't generate a new work slice id!");
        }
        let id = self.previous_work_slice_id + 1;
        self.previous_work_slice_id = id;

        WorkSliceId::new(id)
    }

    fn create_project(&mut self, name: String, description: String) {
        if self.previous_project_id == u64::MAX {
            panic!("Can't generate a new project id!");
        }

        let id = self.previous_project_id + 1;
        let project = Project::new(name, description, ProjectId::new(id));
        self.projects.push(project);

        self.previous_project_id = id;
    }

    fn get_project(&mut self, id: ProjectId) -> &mut Project {
        self.projects.iter_mut().find(|x| x.id() == id).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncomingMessage {
    CreateProject { name: String, description: String },
    StartWorkNow(ProjectId, Payment),
    EndWorkNow(ProjectId),
    StartWork(ProjectId, Payment, Instant),
    EndWork(ProjectId, Instant),
    GetWorkSlices(ProjectId),
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutgoingMessage<'a> {
    ProjectCreated,

    WorkStarted,
    AlreadyStartedWork,
    WorkStartTimeAfterNow,

    WorkEnded,
    NoCurrentWork,

    WorkEndTimeBeforeStartTime,

    WorkSlices(Vec<&'a CompleteWorkSlice>, Option<&'a IncompleteWorkSlice>),
}
