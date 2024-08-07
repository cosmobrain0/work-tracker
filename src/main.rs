mod payment;
mod work_slice;

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::{
        payment::{Money, Payment},
        work_slice::{IncompleteWorkSlice, WorkSliceId},
    };

    #[test]
    fn money_format() {
        let tests = [
            (Money::new(23), "£0.23"),
            (Money::new(0), "£0.00"),
            (Money::new(1), "£0.01"),
            (Money::new(100), "£1.00"),
            (Money::new(145), "£1.45"),
            (Money::new(123456), "£1234.56"),
        ];
        for (test, output) in tests {
            assert_eq!(test.to_string(), output);
        }
    }

    #[test]
    fn fixed_payment() {
        let tests = [
            (
                Payment::Fixed(Money::new(8000)),
                Duration::new(10, 23),
                8000.0,
            ),
            (
                Payment::Fixed(Money::new(4500)),
                Duration::new(15, 28),
                4500.0,
            ),
            (Payment::Fixed(Money::new(23)), Duration::new(13, 23), 23.0),
            (Payment::Fixed(Money::new(45)), Duration::new(118, 23), 45.0),
            (Payment::Fixed(Money::new(0)), Duration::new(12, 23), 0.0),
            (Payment::Fixed(Money::new(1)), Duration::new(1121, 23), 1.0),
            (
                Payment::Fixed(Money::new(100)),
                Duration::new(15, 23),
                100.0,
            ),
            (
                Payment::Fixed(Money::new(245)),
                Duration::new(16, 23),
                245.0,
            ),
            (
                Payment::Fixed(Money::new(4563)),
                Duration::new(3273, 393),
                4563.0,
            ),
        ];
        for (test, duration, output) in tests {
            assert_eq!(test.calculate(duration).as_pence(), output);
        }
    }

    #[test]
    fn hourly_payment() {
        let tests = [
            (3600, 2.0, 7200.0),
            (1250, 5.5, 6875.0),
            (1250, 5.25, 6562.50),
        ];
        for (hourly, duration, total) in tests {
            assert_eq!(
                Payment::Hourly(Money::new(hourly))
                    .calculate(Duration::from_secs_f64(duration * 60.0 * 60.0))
                    .as_pence(),
                total
            );
        }
    }

    #[test]
    fn incomplete_work_slice_eq() {
        let now = Instant::now();
        let after = now + Duration::from_secs(5 * 60 * 60);
        let before = now - Duration::from_secs(5 * 60 * 60);
        let tests = [
            IncompleteWorkSlice::new(
                before,
                Payment::Hourly(Money::new(1000)),
                WorkSliceId::new(0),
            ),
            IncompleteWorkSlice::new(now, Payment::Hourly(Money::new(2000)), WorkSliceId::new(1)),
            IncompleteWorkSlice::new(
                after,
                Payment::Fixed(Money::new(20000)),
                WorkSliceId::new(2),
            ),
        ];
        for test in &tests {
            assert_eq!(test, test);
        }
        assert_ne!(tests[0], tests[1]);
        assert_ne!(tests[1], tests[2]);
        assert_ne!(tests[2], tests[0]);
    }
}
