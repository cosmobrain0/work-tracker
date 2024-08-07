mod payment {
    use std::{fmt::Display, ops::Add, time::Duration};

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
    pub struct Money(u32);
    impl Money {
        pub fn new(money: u32) -> Self {
            Self(money)
        }

        pub fn as_pence(&self) -> u32 {
            self.0
        }
    }
    impl Display for Money {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let pounds = self.0 / 100;
            let pence = self.0 % 100;
            let pence = if pence >= 10 {
                pence.to_string()
            } else {
                format!("0{pence}")
            };
            write!(f, "£{pounds}.{pence}")
        }
    }

    #[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
    pub struct MoneyExact(f64);
    impl MoneyExact {
        pub fn as_pence(&self) -> f64 {
            self.0
        }
    }

    impl From<Money> for MoneyExact {
        fn from(value: Money) -> Self {
            return MoneyExact(value.0.into());
        }
    }

    impl Add<Money> for Money {
        type Output = Money;

        fn add(self, rhs: Money) -> Self::Output {
            return Money(self.0 + rhs.0);
        }
    }

    impl Add<MoneyExact> for MoneyExact {
        type Output = MoneyExact;

        fn add(self, rhs: MoneyExact) -> Self::Output {
            MoneyExact(self.0 + rhs.0)
        }
    }

    impl Add<Money> for MoneyExact {
        type Output = MoneyExact;

        fn add(self, rhs: Money) -> Self::Output {
            return MoneyExact(self.0 + f64::from(rhs.0));
        }
    }

    impl Add<MoneyExact> for Money {
        type Output = MoneyExact;

        fn add(self, rhs: MoneyExact) -> Self::Output {
            return MoneyExact(f64::from(self.0) + rhs.0);
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Payment {
        Hourly(Money),
        Fixed(Money),
    }

    impl Payment {
        pub fn calculate(&self, time: Duration) -> MoneyExact {
            match *self {
                Payment::Hourly(hourly) => {
                    MoneyExact(hourly.as_pence() as f64 * time.as_secs_f64() / (60.0 * 60.0))
                }
                Payment::Fixed(money) => money.into(),
            }
        }
    }
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::payment::{Money, Payment};

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
}
