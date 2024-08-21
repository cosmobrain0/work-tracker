use chrono::{DateTime, Utc};

use crate::{
    state::payment::MoneyExact,
    state::work_slice::{CompleteWorkSlice, IncompleteWorkSlice, WorkSlice, WorkSliceId},
};

use super::CompleteWorkError;

/// Represents the id of a project
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectId(u64);
impl ProjectId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Represents a "project" - basically a group of work slices,
/// which can only have one current work slice at a time.
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

    pub(super) fn new_with_slices(
        name: String,
        description: String,
        id: ProjectId,
        work_slices: Vec<CompleteWorkSlice>,
        current_slice: Option<IncompleteWorkSlice>,
    ) -> Self {
        Self {
            name,
            description,
            id,
            work_slices,
            current_slice,
        }
    }

    /// Only returns the complete work slices in this project,
    /// ignoring the current one, if there is any.
    pub fn complete_work_slices(&self) -> impl Iterator<Item = &CompleteWorkSlice> {
        self.work_slices.iter()
    }

    /// Returns the current, incomplete work slice, if it exists.
    pub fn current_work_slice(&self) -> Option<&IncompleteWorkSlice> {
        self.current_slice.as_ref()
    }

    /// Returns the amount of money earned by the complete work slices in this project.
    /// ignoring the current work slice if there is one.
    pub fn total_payment(&self) -> MoneyExact {
        self.complete_work_slices()
            .map(|x| x.calculate_payment())
            .sum()
    }

    /// Tries to set the given work slice to the current work slice of this project,
    /// but fails if there is already a current work slice.
    pub fn start_work(&mut self, current_work: IncompleteWorkSlice) -> Result<(), ()> {
        if self.current_slice.is_none() {
            self.current_slice = Some(current_work);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Attempts to complete the current work slice,
    /// but fails if there is no current work to complete,
    /// and also fails if the end time provided is after the start time.
    pub fn complete_work(&mut self, end: DateTime<Utc>) -> Result<(), CompleteWorkError> {
        match self.current_slice.take() {
            Some(current_work) => match current_work.complete(end) {
                Ok(complete) => {
                    self.work_slices.push(complete);
                    self.current_slice = None;
                    Ok(())
                }
                Err(incomplete) => {
                    self.current_slice = Some(incomplete);
                    Err(CompleteWorkError::EndTimeTooEarly)
                }
            },
            None => Err(CompleteWorkError::NoWorkToComplete),
        }
    }

    /// Attempts to delete the specified work slice from this project,
    /// whether it's complete or incomplete,
    /// and returns true if it is found (and deleted)
    /// or false otherwise.
    pub fn delete_work_slice(&mut self, work_slice_id: WorkSliceId) -> bool {
        if self
            .current_slice
            .as_ref()
            .is_some_and(|x| x.id() == work_slice_id)
        {
            self.current_slice = None;
            true
        } else {
            match self
                .work_slices
                .iter()
                .enumerate()
                .find(|(_, x)| x.id() == work_slice_id)
                .map(|(i, _)| i)
            {
                Some(i) => {
                    self.work_slices.remove(i);
                    true
                }
                None => false,
            }
        }
    }
}
impl Project {
    /// Returns a reference to the work slice with the given ID,
    /// if it is in this project.
    pub fn work_slice_from_id(&self, id: WorkSliceId) -> Option<WorkSlice<'_>> {
        self.work_slices
            .iter()
            .map(WorkSlice::Complete)
            .find(|x| x.id() == id)
    }
}
