use std::time::Instant;

use crate::{
    payment::Payment,
    work_slice::{CompleteWorkSlice, IncompleteWorkSlice, WorkSlice, WorkSliceId},
};

enum CompleteWorkError {
    NoWorkToComplete,
    EndTimeTooLate,
}

struct ProjectId(u64);

struct Project {
    name: String,
    description: String,
    work_slices: Vec<CompleteWorkSlice>,
    current_slice: Option<IncompleteWorkSlice>,
    id: ProjectId,
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
