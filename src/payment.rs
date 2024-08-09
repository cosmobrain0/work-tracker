use std::{fmt::Display, iter::Sum, ops::Add, time::Duration};

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

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct MoneyExact(f64);
impl MoneyExact {
    pub fn new(money: f64) -> Option<Self> {
        if money >= 0.0 {
            Some(Self(money))
        } else {
            None
        }
    }
    pub fn as_pence(&self) -> f64 {
        self.0
    }
}
impl Sum<MoneyExact> for MoneyExact {
    fn sum<I: Iterator<Item = MoneyExact>>(iter: I) -> Self {
        MoneyExact(iter.map(|x| x.0).sum())
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
