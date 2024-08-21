use chrono::{DateTime, Utc};

use super::{CompleteWorkSlice, IncompleteWorkSlice, Payment, Project, ProjectId, WorkSliceId};

pub struct IncompleteWorkSliceData {
    pub start: DateTime<Utc>,
    pub payment: Payment,
    pub id: u64,
}
impl IncompleteWorkSliceData {
    pub(super) fn into_work_slice(self) -> Option<IncompleteWorkSlice> {
        IncompleteWorkSlice::new(self.start, self.payment, WorkSliceId::new(self.id))
    }
}

pub struct CompleteWorkSliceData {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub payment: Payment,
    pub id: u64,
}
impl CompleteWorkSliceData {
    pub(super) fn into_work_slice(self) -> Option<CompleteWorkSlice> {
        match IncompleteWorkSlice::new(self.start, self.payment, WorkSliceId::new(self.id)) {
            Some(incomplete) => match CompleteWorkSlice::new(incomplete, self.end) {
                Ok(complete) => Some(complete),
                Err(_) => None,
            },
            None => None,
        }
    }
}

pub struct ProjectData {
    pub name: String,
    pub description: String,
    pub work_slices: Vec<CompleteWorkSliceData>,
    pub current_slice: Option<IncompleteWorkSliceData>,
    pub id: u64,
}
impl ProjectData {
    pub(super) fn into_project(self) -> Option<Project> {
        let complete: Vec<_> = self
            .work_slices
            .into_iter()
            .map(CompleteWorkSliceData::into_work_slice)
            .collect();
        if complete.iter().any(Option::is_none) {
            return None;
        }
        let complete = complete.into_iter().map(Option::unwrap).collect();

        let current = match self.current_slice {
            None => None,
            Some(x) => match x.into_work_slice() {
                Some(x) => Some(x),
                None => {
                    return None;
                }
            },
        };

        Some(Project::new_with_slices(
            self.name,
            self.description,
            ProjectId::new(self.id),
            complete,
            current,
        ))
    }
}
