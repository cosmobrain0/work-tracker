use std::{fmt::Display, iter::Sum, ops::Add};

use chrono::TimeDelta;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Represents a whole number of pence.
/// If you want to store fractional components, see `MoneyExact`
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Money(u32);
impl Money {
    /// Returns a new instance of `Money`
    /// with `money` pence.
    pub fn new(money: u32) -> Self {
        Self(money)
    }

    /// Returns the number of pence
    /// that `self` represents.
    pub fn as_pence(&self) -> u32 {
        self.0
    }
}
impl Sum<Money> for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Self {
        Money(iter.map(|x| x.0).sum())
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

/// Represents some amount of money.
/// This can be used to store fractional pence.
/// If you want perfect precision (with no fractional components), see `Money`
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MoneyExact(f64);
impl MoneyExact {
    /// Returns a new instance of `MoneyExact` if `money` is non-negative,
    /// or `None` otherwise.
    pub fn new(money: f64) -> Option<Self> {
        if money >= 0.0 {
            Some(Self(money))
        } else {
            None
        }
    }

    /// Returns the number of pence that this value stores,
    /// which is guaranteed to be non-negative.
    pub fn as_pence(&self) -> f64 {
        self.0
    }
}
impl Display for MoneyExact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pounds = (self.0 / 100.0).floor();
        let pence = self.0 % 100.0;
        let pence = if pence >= 10.0 {
            (pence / 100.0).to_string().chars().skip(2).collect()
        } else if pence > 0.0 {
            format!(
                "0{pence}",
                pence = (pence / 100.0)
                    .to_string()
                    .chars()
                    .skip(2)
                    .collect::<String>()
            )
        } else {
            "00".to_string()
        };
        write!(f, "£{pounds}.{pence:.2}")
    }
}
impl Sum<MoneyExact> for MoneyExact {
    fn sum<I: Iterator<Item = MoneyExact>>(iter: I) -> Self {
        MoneyExact(iter.map(|x| x.0).sum())
    }
}

impl From<Money> for MoneyExact {
    fn from(value: Money) -> Self {
        MoneyExact(value.0.into())
    }
}

impl Add<Money> for Money {
    type Output = Money;

    fn add(self, rhs: Money) -> Self::Output {
        Money(self.0 + rhs.0)
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
        MoneyExact(self.0 + f64::from(rhs.0))
    }
}

impl Add<MoneyExact> for Money {
    type Output = MoneyExact;

    fn add(self, rhs: MoneyExact) -> Self::Output {
        MoneyExact(f64::from(self.0) + rhs.0)
    }
}

/// Stores how the payment of a work slice will be calculated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Payment {
    /// Payment is a fixed amount of money per hour,
    /// with no rounding (so every second is paid for, even if it's less than one hour)
    Hourly(Money),
    /// Payment is a fixed amount of money,
    /// regardless of how long the work slice takes.
    Fixed(Money),
}
impl Payment {
    /// Calculate how much this payment method would pay
    /// for a work slice with a given duration.
    pub fn calculate(&self, time: TimeDelta) -> MoneyExact {
        match *self {
            Payment::Hourly(hourly) => MoneyExact(
                hourly.as_pence() as f64 * time.num_milliseconds() as f64 / 1000.0 / (60.0 * 60.0),
            ),
            Payment::Fixed(money) => money.into(),
        }
    }
}
impl Display for Payment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Hourly(money) => format!("{money} / hour"),
                Self::Fixed(money) => format!("fixed at {money}"),
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::{Money, Payment};

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
                TimeDelta::new(10, 23).unwrap(),
                8000.0,
            ),
            (
                Payment::Fixed(Money::new(4500)),
                TimeDelta::new(15, 28).unwrap(),
                4500.0,
            ),
            (
                Payment::Fixed(Money::new(23)),
                TimeDelta::new(13, 23).unwrap(),
                23.0,
            ),
            (
                Payment::Fixed(Money::new(45)),
                TimeDelta::new(118, 23).unwrap(),
                45.0,
            ),
            (
                Payment::Fixed(Money::new(0)),
                TimeDelta::new(12, 23).unwrap(),
                0.0,
            ),
            (
                Payment::Fixed(Money::new(1)),
                TimeDelta::new(1121, 23).unwrap(),
                1.0,
            ),
            (
                Payment::Fixed(Money::new(100)),
                TimeDelta::new(15, 23).unwrap(),
                100.0,
            ),
            (
                Payment::Fixed(Money::new(245)),
                TimeDelta::new(16, 23).unwrap(),
                245.0,
            ),
            (
                Payment::Fixed(Money::new(4563)),
                TimeDelta::new(3273, 393).unwrap(),
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
                    .calculate(TimeDelta::seconds(
                        (duration * 60.0f64 * 60.0).floor() as i64
                    ))
                    .as_pence(),
                total
            );
        }
    }
}
