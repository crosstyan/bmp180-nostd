use num_traits::{ToPrimitive};

/// Pressure representation
#[derive(Debug, Copy, Clone)]
pub struct Pressure {
    pascal: f32,
}

impl Pressure {
    /// hpa
    pub fn hpa(&self) -> f32 {
        self.pascal / 100.0
    }
    /// kpa
    pub fn kpa(&self) -> f32 {
        self.pascal / 1000.0
    }
    /// pascal
    pub fn pascal(&self) -> f32 {
        self.pascal
    }
    /// construct from pascal
    pub fn from_pascal<T>(pascal: T) -> Self
    where
        T: ToPrimitive,
     {
        let pascal = pascal.to_f32().unwrap();
        Pressure { pascal }
    }
}

impl Default for Pressure {
    fn default() -> Self {
        Pressure { pascal: 0.0 }
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


