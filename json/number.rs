
use serde_json;
use std::cmp::Ordering;

/// `Number` holds possible numeric types of the JSON object model.
#[derive(Debug)]
pub enum Number {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
}
use Number::*;

impl From<&serde_json::Number> for Number {
    fn from(n: &serde_json::Number) -> Number {
        if let Some(n) = n.as_u64() {
            Unsigned(n)
        } else if let Some(n) = n.as_i64() {
            Signed(n)
        } else {
            Float(n.as_f64().unwrap())
        }
    }
}

impl std::ops::AddAssign<u64> for Number {
    fn add_assign(&mut self, rhs: u64) {
        match self {
            Unsigned(lhs) => *lhs += rhs,
            Signed(lhs) => *lhs += rhs as i64,
            Float(lhs) => *lhs += rhs as f64,
        }
    }
}

impl std::ops::AddAssign<i64> for Number {
    fn add_assign(&mut self, rhs: i64) {
        match self {
            Unsigned(lhs) => *self = Signed((*lhs as i64) + rhs),
            Signed(lhs) => *lhs += rhs,
            Float(lhs) => *lhs += rhs as f64,
        }
    }
}

impl std::ops::AddAssign<f64> for Number {
    fn add_assign(&mut self, rhs: f64) {
        match self {
            Unsigned(lhs) => *self = Float((*lhs as f64) + rhs),
            Signed(lhs) => *self = Float((*lhs as f64) + rhs),
            Float(lhs) => *lhs += rhs,
        }
    }
}

impl std::ops::AddAssign<Number> for Number {
    fn add_assign(&mut self, rhs: Number) {
        match rhs {
            Unsigned(rhs) => *self += rhs,
            Signed(rhs) => *self += rhs,
            Float(rhs) => *self += rhs,
        }
    }
}

impl std::ops::Add for Number {
    type Output = Self;

    fn add(self, rhs: Number) -> Self::Output {
        let mut lhs = self;
        lhs += rhs;
        lhs
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use Number::*;

        match *self {
            Unsigned(lhs) => match other {
                Unsigned(rhs) => lhs.partial_cmp(rhs),
                Signed(rhs) => (lhs as i64).partial_cmp(rhs),
                Float(rhs) => (lhs as f64).partial_cmp(rhs),
            },
            Signed(lhs) => match other {
                Unsigned(rhs) => lhs.partial_cmp(&(*rhs as i64)),
                Signed(rhs) => lhs.partial_cmp(rhs),
                Float(rhs) => (lhs as f64).partial_cmp(rhs),
            },
            Float(lhs) => match other {
                Unsigned(rhs) => lhs.partial_cmp(&(*rhs as f64)),
                Signed(rhs) => lhs.partial_cmp(&(*rhs as f64)),
                Float(rhs) => lhs.partial_cmp(rhs),
            },
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other)
            .map_or(false, |c| c == Ordering::Equal)
    }
}

impl Number {
    pub fn is_multiple_of(&self, d: &Self) -> bool {
        use Number::*;

        match *d {
            Unsigned(d) => match *self {
                Unsigned(n) => n % d == 0,
                Signed(n) => n % (d as i64) == 0,
                Float(n) => (n / (d as f64)).fract() == 0.0,
            },
            Signed(d) => match *self {
                Unsigned(n) => (n as i64) % d == 0,
                Signed(n) => n % d == 0,
                Float(n) => (n / (d as f64)).fract() == 0.0,
            },
            Float(d) => match *self {
                Unsigned(n) => (n as f64) % d == 0.0,
                Signed(n) => (n as f64) % d == 0.0,
                Float(n) => (n / d).fract() == 0.0,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_multiple_of() {
        assert!(Unsigned(32).is_multiple_of(&Unsigned(4)));
        assert!(Unsigned(32).is_multiple_of(&Signed(-4)));
        assert!(Unsigned(32).is_multiple_of(&Float(4.0)));
        assert!(!Unsigned(32).is_multiple_of(&Unsigned(5)));
        assert!(!Unsigned(32).is_multiple_of(&Signed(-5)));
        assert!(!Unsigned(32).is_multiple_of(&Float(4.5)));

        assert!(Signed(32).is_multiple_of(&Unsigned(4)));
        assert!(Signed(-32).is_multiple_of(&Signed(-4)));
        assert!(Signed(-32).is_multiple_of(&Float(4.0)));
        assert!(!Signed(32).is_multiple_of(&Unsigned(5)));
        assert!(!Signed(-32).is_multiple_of(&Signed(-5)));
        assert!(!Signed(-32).is_multiple_of(&Float(4.5)));

        assert!(Float(32.0).is_multiple_of(&Unsigned(4)));
        assert!(Float(-32.0).is_multiple_of(&Signed(-4)));
        assert!(Float(-32.0).is_multiple_of(&Float(4.0)));
        assert!(!Float(32.1).is_multiple_of(&Unsigned(4)));
        assert!(!Float(-32.1).is_multiple_of(&Signed(-4)));
        assert!(!Float(-32.1).is_multiple_of(&Float(4.0)));
    }

    #[test]
    fn test_equality() {
        is_eq(Unsigned(10), Unsigned(10));
        is_eq(Signed(-10), Signed(-10));
        is_eq(Float(1.0), Float(1.0));
        is_eq(Unsigned(20), Signed(20));
        is_eq(Unsigned(20), Float(20.00));
        is_eq(Signed(-20), Float(-20.00));
    }

    #[test]
    fn test_ordering() {
        is_lt(Unsigned(10), Unsigned(11));
        is_lt(Signed(-11), Signed(-10));
        is_lt(Float(1.0), Float(1.1));

        is_lt(Unsigned(10), Float(10.1));
        is_lt(Signed(-10), Float(-9.9));

        is_lt(Signed(10), Unsigned(11));
        is_lt(Signed(-1), Unsigned(0));
    }

    #[test]
    fn test_add() {
        is_eq(Unsigned(1) + Unsigned(2), Unsigned(3));
        is_eq(Signed(-1) + Signed(-2), Signed(-3));
        is_eq(Float(1.0) + Float(2.0), Float(3.0));

        is_eq(Unsigned(1) + Signed(-2), Signed(-1));
        is_eq(Signed(-2) + Unsigned(3), Signed(1));

        is_eq(Unsigned(1) + Float(0.1), Float(1.1));
        is_eq(Float(-0.1) + Unsigned(1), Float(0.9));

        is_eq(Signed(-1) + Float(2.1), Float(1.1));
        is_eq(Float(0.1) + Signed(-2), Float(-1.9));
    }

    fn is_lt(lhs: Number, rhs: Number) {
        assert_eq!(lhs.partial_cmp(&rhs), Some(Ordering::Less));
        assert_eq!(rhs.partial_cmp(&lhs), Some(Ordering::Greater));
    }

    fn is_eq(lhs: Number, rhs: Number) {
        assert_eq!(lhs, rhs);
        assert_eq!(rhs, lhs);
    }
}