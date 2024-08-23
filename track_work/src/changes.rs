use chrono::{DateTime, Utc};

use super::Payment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    ProjectCreated {
        name: String,
        description: String,
        id: u64,
    },
    ProjectDeleted {
        id: u64,
    },
    WorkSliceCompleted {
        project_id: u64,
        work_slice_id: u64,
        end_time: DateTime<Utc>,
    },
    WorkSliceStarted {
        project_id: u64,
        work_slice_id: u64,
        start_time: DateTime<Utc>,
        payment: Payment,
    },
    WorkSliceDeleted {
        project_id: u64,
        work_slice_id: u64,
    },
}
