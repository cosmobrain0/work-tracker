use chrono::{DateTime, TimeDelta, Utc};

use crate::{
    pop_data, pop_u32,
    state::payment::{MoneyExact, Payment},
};

pub enum WorkSlice {
    Complete(CompleteWorkSlice),
    Incomplete(IncompleteWorkSlice),
}
impl WorkSlice {
    pub fn as_complete(self) -> Option<CompleteWorkSlice> {
        match self {
            Self::Complete(x) => Some(x),
            Self::Incomplete(_) => None,
        }
    }

    pub fn as_incomplete(self) -> Option<IncompleteWorkSlice> {
        match self {
            Self::Complete(_) => None,
            Self::Incomplete(x) => Some(x),
        }
    }

    pub fn unwrap(self) -> CompleteWorkSlice {
        match self {
            Self::Complete(x) => x,
            Self::Incomplete(x) => panic!("Trying to unwrap a WorkSlice::Incomplete! {:#?}", x),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorkSliceId(u64);
impl WorkSliceId {
    pub(super) fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, PartialOrd, Ord)]
pub struct IncompleteWorkSlice {
    start: DateTime<Utc>,
    payment: Payment,
    id: WorkSliceId,
}
impl IncompleteWorkSlice {
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn payment(&self) -> Payment {
        self.payment
    }

    pub fn id(&self) -> WorkSliceId {
        self.id
    }
}
impl IncompleteWorkSlice {
    pub(super) fn new(start: DateTime<Utc>, payment: Payment, id: WorkSliceId) -> Option<Self> {
        if start <= Utc::now() {
            Some(Self { start, payment, id })
        } else {
            None
        }
    }

    pub(super) fn complete(self, end: DateTime<Utc>) -> WorkSlice {
        CompleteWorkSlice::new(self, end)
    }

    pub(super) fn complete_now(self) -> CompleteWorkSlice {
        CompleteWorkSlice::new(self, Utc::now()).unwrap()
    }

    pub(super) fn payment_so_far(&self) -> Option<MoneyExact> {
        Some(self.payment.calculate(Utc::now() - self.start))
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
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    payment: Payment,
    id: WorkSliceId,
}
impl CompleteWorkSlice {
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn payment(&self) -> Payment {
        self.payment
    }

    pub fn id(&self) -> WorkSliceId {
        self.id
    }

    pub fn end(&self) -> DateTime<Utc> {
        self.end
    }
}
impl CompleteWorkSlice {
    pub(super) fn new(work_slice: IncompleteWorkSlice, end: DateTime<Utc>) -> WorkSlice {
        if end > work_slice.start {
            WorkSlice::Complete(Self {
                end,
                start: work_slice.start,
                payment: work_slice.payment,
                id: work_slice.id,
            })
        } else {
            WorkSlice::Incomplete(work_slice)
        }
    }

    pub fn duration(&self) -> TimeDelta {
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

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, Utc};

    use crate::state::{IncompleteWorkSlice, Money, MoneyExact, Payment, WorkSliceId};

    #[test]
    fn incomplete_work_slice_eq() {
        let now = Utc::now();
        let before = now - TimeDelta::seconds(5 * 60 * 60);
        let tests = [
            IncompleteWorkSlice::new(
                before,
                Payment::Hourly(Money::new(1000)),
                WorkSliceId::new(0),
            )
            .unwrap(),
            IncompleteWorkSlice::new(now, Payment::Hourly(Money::new(2000)), WorkSliceId::new(1))
                .unwrap(),
        ];
        for test in &tests {
            assert_eq!(test, test);
        }
        assert_ne!(tests[0], tests[1]);
        assert_ne!(tests[1], tests[0]);
    }

    fn almost_equal(a: f64, b: f64) -> bool {
        (a - b).abs() <= 0.0001
    }

    #[test]
    fn work_slice_payment_calculation() {
        let now = Utc::now();
        let after = now + TimeDelta::seconds(5 * 60 * 60);
        let before = now - TimeDelta::seconds(5 * 60 * 60);
        let mut tests = vec![
            (
                IncompleteWorkSlice::new(
                    before,
                    Payment::Hourly(Money::new(1000)),
                    WorkSliceId::new(0),
                ),
                Some(MoneyExact::new(5000.0).unwrap()),
            ),
            (
                IncompleteWorkSlice::new(
                    now,
                    Payment::Hourly(Money::new(2000)),
                    WorkSliceId::new(1),
                ),
                Some(MoneyExact::new(0.0).unwrap()),
            ),
            (
                IncompleteWorkSlice::new(
                    after,
                    Payment::Fixed(Money::new(20000)),
                    WorkSliceId::new(2),
                ),
                None,
            ),
        ];
        assert!(tests[0].0.is_some());
        assert!(tests[1].0.is_some());
        assert!(tests[2].0.is_none());
        tests.pop();
        let monies = tests.iter().map(|x| x.1.unwrap()).collect::<Vec<_>>();
        let tests = [
            (tests.pop().unwrap().0.unwrap(), monies[1]),
            (tests.pop().unwrap().0.unwrap(), monies[0]),
        ];
        for (test, payment) in tests {
            match (
                test.payment_so_far().map(|x| x.as_pence()),
                payment.as_pence(),
            ) {
                (None, x) => panic!("Should have gotten {:#?}, but got None", x),
                (Some(a), b) => assert!(almost_equal(a, b)),
            }

            assert!(almost_equal(
                test.complete_now().calculate_payment().as_pence(),
                payment.as_pence(),
            ));
        }
    }
}
