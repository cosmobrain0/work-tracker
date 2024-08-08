use std::{error::Error, fmt::Display, time::Instant};

use crate::{
    payment::Payment,
    work_slice::{CompleteWorkSlice, IncompleteWorkSlice, WorkSlice, WorkSliceId},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteWorkError {
    NoWorkToComplete,
    EndTimeTooLate,
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
    pub fn new(name: String, description: String, id: ProjectId) -> Self {
        Self {
            name,
            description,
            id,
            work_slices: Vec::new(),
            current_slice: None,
        }
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
            self.current_slice =
                Some(IncompleteWorkSlice::new(Instant::now(), payment, id).unwrap());
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn complete_work(&mut self, end: Instant) -> Result<(), CompleteWorkError> {
        match self.current_slice.take() {
            Some(current_work) => match current_work.complete(end) {
                WorkSlice::Complete(complete) => {
                    self.work_slices.push(complete);
                    self.current_slice = None;
                    Ok(())
                }
                WorkSlice::Incomplete(incomplete) => {
                    self.current_slice = Some(incomplete);
                    Err(CompleteWorkError::EndTimeTooLate)
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
}
