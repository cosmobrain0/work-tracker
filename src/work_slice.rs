use std::time::{Duration, Instant};

use crate::payment::Payment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorkSliceId(u64);
impl WorkSliceId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, PartialOrd, Ord)]
pub struct IncompleteWorkSlice {
    start: Instant,
    payment: Payment,
    id: WorkSliceId,
}
impl IncompleteWorkSlice {
    pub fn new(start: Instant, payment: Payment, id: WorkSliceId) -> Self {
        Self { start, payment, id }
    }
}
impl PartialEq for IncompleteWorkSlice {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for IncompleteWorkSlice {}

#[derive(Debug, PartialOrd, Ord)]
pub struct CompleteWorkSlice {
    start: Instant,
    end: Instant,
    payment: Payment,
    id: WorkSliceId,
}
impl CompleteWorkSlice {
    pub fn new(work_slice: IncompleteWorkSlice, end: Instant) -> Option<Self> {
        if end > work_slice.start {
            Some(Self {
                end,
                start: work_slice.start,
                payment: work_slice.payment,
                id: work_slice.id,
            })
        } else {
            None
        }
    }

    pub fn duration(&self) -> Duration {
        self.end - self.start
    }
}
impl PartialEq for CompleteWorkSlice {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for CompleteWorkSlice {}
