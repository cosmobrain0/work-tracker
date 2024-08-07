use std::time::{Duration, Instant};

use crate::payment::{MoneyExact, Payment};

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
    pub fn new(start: Instant, payment: Payment, id: WorkSliceId) -> Option<Self> {
        if start <= Instant::now() {
            Some(Self { start, payment, id })
        } else {
            None
        }
    }

    pub fn complete(self, end: Instant) -> Option<CompleteWorkSlice> {
        CompleteWorkSlice::new(self, end)
    }

    pub fn complete_now(self) -> CompleteWorkSlice {
        CompleteWorkSlice::new(self, Instant::now()).unwrap()
    }

    pub fn payment_so_far(&self) -> Option<MoneyExact> {
        Some(self.payment.calculate(Instant::now() - self.start))
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

    pub fn calculate_payment(&self) -> MoneyExact {
        self.payment.calculate(self.duration())
    }
}
impl PartialEq for CompleteWorkSlice {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for CompleteWorkSlice {}
