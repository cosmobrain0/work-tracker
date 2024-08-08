mod payment;
mod project;
mod state;
mod work_slice;

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::{
        payment::{Money, MoneyExact, Payment},
        project::{Project, ProjectId},
        state::{IncomingMessage, OutgoingMessage, State},
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
        let before = now - Duration::from_secs(5 * 60 * 60);
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
        let now = Instant::now();
        let after = now + Duration::from_secs(5 * 60 * 60);
        let before = now - Duration::from_secs(5 * 60 * 60);
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

    #[test]
    fn project_equality() {
        let tests = [
            Project::new(
                "hello".to_string(),
                "this is a test".to_string(),
                ProjectId::new(1),
            ),
            Project::new(
                "hi".to_string(),
                "this is a test".to_string(),
                ProjectId::new(2),
            ),
        ];
        assert_eq!(tests[0], tests[0]);
        assert_ne!(tests[0], tests[1]);
        assert_ne!(tests[1], tests[0]);
    }

    #[test]
    fn state_creates_many_projects() {
        let mut state = State::new();
        for i in 0..10000 {
            let message = state.process_message(IncomingMessage::CreateProject {
                name: String::from("Example Project"),
                description: String::from("Example description!"),
            });
            assert_eq!(message, OutgoingMessage::ProjectCreated);
        }
    }
}
