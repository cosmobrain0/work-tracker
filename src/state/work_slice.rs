use chrono::{DateTime, TimeDelta, Utc};

use crate::state::payment::{MoneyExact, Payment};

/// Represents a reference to a work slice
/// which may or may not be complete
#[derive(Debug)]
pub enum WorkSlice<'a> {
    Complete(&'a CompleteWorkSlice),
    Incomplete(&'a IncompleteWorkSlice),
}
impl<'a> WorkSlice<'a> {
    /// Returns the reference to the complete work slice that this holds,
    /// if it holds one.
    pub fn complete(self) -> Option<&'a CompleteWorkSlice> {
        match self {
            Self::Complete(x) => Some(x),
            Self::Incomplete(_) => None,
        }
    }

    /// Returns the reference to the incomplete work slice that this holds,
    /// if it holds one.
    pub fn incomplete(self) -> Option<&'a IncompleteWorkSlice> {
        match self {
            Self::Complete(_) => None,
            Self::Incomplete(x) => Some(x),
        }
    }

    /// Returns the reference to the complete work slice that this holds,
    /// or panics if it doesn't.
    pub fn unwrap(self) -> &'a CompleteWorkSlice {
        match self {
            Self::Complete(x) => x,
            Self::Incomplete(x) => panic!("Trying to unwrap a WorkSlice::Incomplete! {:#?}", x),
        }
    }

    /// Gets the start time for this work slice.
    pub fn start(&self) -> DateTime<Utc> {
        match self {
            WorkSlice::Complete(x) => x.start(),
            WorkSlice::Incomplete(x) => x.start(),
        }
    }

    /// Gets the duration of this work slice.
    /// This returns the time between `Utc::now()` and the start of the work slice,
    /// if the work slice is incomplete.
    pub fn duration(&self) -> TimeDelta {
        match self {
            WorkSlice::Complete(x) => x.duration(),
            WorkSlice::Incomplete(x) => x.duration(),
        }
    }

    /// Returns the method of calculating payment for this work slice.
    pub fn payment_rate(&self) -> Payment {
        match self {
            WorkSlice::Complete(x) => x.payment(),
            WorkSlice::Incomplete(x) => x.payment(),
        }
    }

    /// Returns the total payment required for this work slice
    /// or the total payment required *so far* for incomplete work slices.
    pub fn total_payment(&self) -> MoneyExact {
        match self {
            WorkSlice::Complete(x) => x.calculate_payment(),
            WorkSlice::Incomplete(x) => x.calculate_payment_so_far(),
        }
    }

    /// Returns the ID of this work slice
    pub fn id(&self) -> WorkSliceId {
        match self {
            WorkSlice::Complete(x) => x.id(),
            WorkSlice::Incomplete(x) => x.id(),
        }
    }
}

/// Represents the id of a work slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorkSliceId(u64);
impl WorkSliceId {
    pub unsafe fn new(id: u64) -> Self {
        Self(id)
    }

    pub unsafe fn inner(&self) -> u64 {
        self.0
    }
}

/// Represents a work slice which has started
/// but has not ended.
#[derive(Debug, PartialOrd, Ord)]
pub struct IncompleteWorkSlice {
    start: DateTime<Utc>,
    payment: Payment,
    id: WorkSliceId,
}
impl IncompleteWorkSlice {
    /// Returns the start time of this work slice.
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    /// Returns the method of calculating payment of this work slice.
    /// See `calculate_payment_so_far` to calculate the amount of money earned so far.
    pub fn payment(&self) -> Payment {
        self.payment
    }

    /// Returns the id of this work slice.
    pub fn id(&self) -> WorkSliceId {
        self.id
    }
}
impl IncompleteWorkSlice {
    /// Constructs a new incomplete work slice, if `start <= Utc::now()`
    /// and fails otherwise.
    pub(super) fn new(start: DateTime<Utc>, payment: Payment, id: WorkSliceId) -> Option<Self> {
        if start <= Utc::now() {
            Some(Self { start, payment, id })
        } else {
            None
        }
    }

    /// Returns how much time has passed since the start of this work slice.
    pub fn duration(&self) -> TimeDelta {
        Utc::now() - self.start
    }

    /// Calculates how much money has been earned by this work slice so far.
    /// See `payment` to get the method of calculating payment for this work slice.
    pub fn calculate_payment_so_far(&self) -> MoneyExact {
        self.payment.calculate(self.duration())
    }

    /// Attempts to make a complete work slice out of this one, consuming it,
    /// and returns this work slice if that fails because the end time is before the start time.
    pub(super) fn complete(
        self,
        end: DateTime<Utc>,
    ) -> Result<CompleteWorkSlice, IncompleteWorkSlice> {
        CompleteWorkSlice::new(self, end)
    }

    /// Attempts to make a complete work slice out of this one, ending at Utc::now(),
    /// consuming this incomplete work slice,
    /// and returns the completed work slice.
    /// # Panics
    /// panics if the work slice is ended at the same time as when it starts.
    pub(super) fn complete_now(self) -> CompleteWorkSlice {
        CompleteWorkSlice::new(self, Utc::now()).unwrap()
    }
}
impl PartialEq for IncompleteWorkSlice {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for IncompleteWorkSlice {}

/// Represents a work slice which has been completed.
/// Only the `start` of this work slice is guaranteed to not be in the future.
#[derive(Debug, PartialOrd, Ord)]
pub struct CompleteWorkSlice {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    payment: Payment,
    id: WorkSliceId,
}
impl CompleteWorkSlice {
    /// Returns the start time of this work slice.
    /// This is guaranteed to not be in the future.
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    /// Returns the method of calculating payment of this work slice.
    /// See `calculate_payment` to get the amount of money earned.
    pub fn payment(&self) -> Payment {
        self.payment
    }

    /// Returns the id of this work slice.
    pub fn id(&self) -> WorkSliceId {
        self.id
    }

    /// Returns the end time of this work slice.
    /// This is **NOT** guaranteed to not be in the future.
    pub fn completion(&self) -> DateTime<Utc> {
        self.end
    }
}
impl CompleteWorkSlice {
    /// Constructs a complete work slice out of an incomplete one,
    /// if the end time is later than the start time.
    pub(super) fn new(
        work_slice: IncompleteWorkSlice,
        end: DateTime<Utc>,
    ) -> Result<CompleteWorkSlice, IncompleteWorkSlice> {
        if end > work_slice.start {
            Ok(Self {
                end,
                start: work_slice.start,
                payment: work_slice.payment,
                id: work_slice.id,
            })
        } else {
            Err(work_slice)
        }
    }

    /// Returns the time between the end and the start of this work slice.
    pub fn duration(&self) -> TimeDelta {
        self.end - self.start
    }

    /// Returns the amount of money earned by this work slice.
    /// See `payment` to get the method of calculating payment.
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
            IncompleteWorkSlice::new(before, Payment::Hourly(Money::new(1000)), unsafe {
                WorkSliceId::new(0)
            })
            .unwrap(),
            IncompleteWorkSlice::new(now, Payment::Hourly(Money::new(2000)), unsafe {
                WorkSliceId::new(1)
            })
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
                IncompleteWorkSlice::new(before, Payment::Hourly(Money::new(1000)), unsafe {
                    WorkSliceId::new(0)
                }),
                Some(MoneyExact::new(5000.0).unwrap()),
            ),
            (
                IncompleteWorkSlice::new(now, Payment::Hourly(Money::new(2000)), unsafe {
                    WorkSliceId::new(1)
                }),
                Some(MoneyExact::new(0.0).unwrap()),
            ),
            (
                IncompleteWorkSlice::new(after, Payment::Fixed(Money::new(20000)), unsafe {
                    WorkSliceId::new(2)
                }),
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
            assert!(almost_equal(
                test.calculate_payment_so_far().as_pence(),
                payment.as_pence(),
            ));

            assert!(almost_equal(
                test.complete_now().calculate_payment().as_pence(),
                payment.as_pence(),
            ));
        }
    }
}
