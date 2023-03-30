use num_traits::ToPrimitive;
use ordered_float::NotNan;

/// Pressure representation
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pressure {
    pascal: NotNan<f32>,
}

impl Pressure {
    /// hpa
    pub fn hpa(&self) -> f32 {
        *self.pascal / 100.0
    }
    /// kpa
    pub fn kpa(&self) -> f32 {
        *self.pascal / 1000.0
    }
    /// pascal
    pub fn pascal(&self) -> f32 {
        *self.pascal
    }
    /// construct from pascal
    pub fn from_pascal<T>(pascal: T) -> Self
    where
        T: ToPrimitive,
    {
        let pascal = pascal.to_f32().unwrap();
        let pascal = NotNan::new(pascal).unwrap();
        Pressure { pascal }
    }

    /// construct from hpa
    pub fn from_hpa<T>(hpa: T) -> Self
    where
        T: ToPrimitive,
    {
        let pascal = hpa.to_f32().unwrap() * 100.0;
        let pascal = NotNan::new(pascal).unwrap();
        Pressure { pascal }
    }
}

impl Default for Pressure {
    fn default() -> Self {
        Pressure {
            pascal: NotNan::new(0.0).unwrap(),
        }
    }
}

impl std::ops::Add for Pressure {
    type Output = Pressure;

    fn add(self, other: Pressure) -> Pressure {
        Pressure {
            pascal: self.pascal + other.pascal,
        }
    }
}

impl std::ops::AddAssign for Pressure {
    fn add_assign(&mut self, other: Pressure) {
        self.pascal += other.pascal;
    }
}

impl std::ops::Sub for Pressure {
    type Output = Pressure;

    fn sub(self, other: Pressure) -> Pressure {
        Pressure {
            pascal: self.pascal - other.pascal,
        }
    }
}

impl std::ops::Mul<f32> for Pressure {
    type Output = Pressure;

    fn mul(self, other: f32) -> Pressure {
        Pressure {
            pascal: self.pascal * other,
        }
    }
}

impl std::ops::Div<f32> for Pressure {
    type Output = Pressure;

    fn div(self, other: f32) -> Pressure {
        Pressure {
            pascal: self.pascal / other,
        }
    }
}
