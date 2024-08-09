use std::{error::Error, fmt::Display, time::Instant};

use crate::{
    payment::{MoneyExact, Payment},
    work_slice::{CompleteWorkSlice, IncompleteWorkSlice, WorkSlice, WorkSliceId},
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
