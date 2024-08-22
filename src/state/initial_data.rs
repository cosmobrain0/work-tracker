use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    CompleteWorkSlice, DataToCompleteWorkSliceError, DataToProjectError, IncompleteWorkSlice,
    Payment, Project, ProjectId, WorkSliceId,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IncompleteWorkSliceData {
    pub start: DateTime<Utc>,
    pub payment: Payment,
    pub id: u64,
}
impl IncompleteWorkSliceData {
    pub(super) fn into_work_slice(self) -> Option<IncompleteWorkSlice> {
        IncompleteWorkSlice::new(self.start, self.payment, unsafe {
            WorkSliceId::new(self.id)
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompleteWorkSliceData {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub payment: Payment,
    pub id: u64,
}
impl CompleteWorkSliceData {
    pub(super) fn into_work_slice(self) -> Result<CompleteWorkSlice, DataToCompleteWorkSliceError> {
        match IncompleteWorkSlice::new(self.start, self.payment, unsafe {
            WorkSliceId::new(self.id)
        }) {
            Some(incomplete) => match CompleteWorkSlice::new(incomplete, self.end) {
                Ok(complete) => Ok(complete),
                Err(_) => Err(DataToCompleteWorkSliceError::EndTimeBeforeStart),
            },
            None => Err(DataToCompleteWorkSliceError::StartTimeAfterNow),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectData {
    pub name: String,
    pub description: String,
    pub work_slices: Vec<CompleteWorkSliceData>,
    pub current_slice: Option<IncompleteWorkSliceData>,
    pub id: u64,
}
impl ProjectData {
    pub(super) fn into_project(self) -> Result<Project, DataToProjectError> {
        let complete: Vec<_> = self
            .work_slices
            .into_iter()
            .map(CompleteWorkSliceData::into_work_slice)
            .collect();
        if let Some(err) = complete.iter().find_map(|x| x.as_ref().err()) {
            return Err(DataToProjectError::CompleteWorkSlice(*err));
        }
        let complete = complete.into_iter().map(Result::unwrap).collect();

        let current = match self.current_slice {
            None => None,
            Some(x) => match x.into_work_slice() {
                Some(x) => Some(x),
                None => {
                    return Err(DataToProjectError::IncompleteWorkSlice);
                }
            },
        };

        Ok(Project::new_with_slices(
            self.name,
            self.description,
            unsafe { ProjectId::new(self.id) },
            complete,
            current,
        ))
    }
}
