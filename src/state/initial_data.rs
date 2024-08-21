use chrono::{DateTime, Utc};

use super::Payment;

pub struct IncompleteWorkSliceData {
    pub start: DateTime<Utc>,
    pub payment: Payment,
    pub id: u64,
}

pub struct CompleteWorkSliceData {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub payment: Payment,
    pub id: u64,
}

pub struct ProjectData {
    pub name: String,
    pub description: String,
    pub work_slices: Vec<CompleteWorkSliceData>,
    pub current_slice: Option<IncompleteWorkSliceData>,
    pub id: u32,
}
