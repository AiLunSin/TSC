use serde::{Deserialize, Serialize};

/// 置信度：闭区间 `[0, 1]`，拒绝 NaN 与无穷。
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct Confidence(f64);

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ConfidenceError {
    #[error("置信度必须为有限数值，实际为 {0}")]
    NotFinite(f64),
    #[error("置信度必须落在 [0, 1]，实际为 {0}")]
    OutOfRange(f64),
}

impl Confidence {
    pub const ZERO: Self = Self(0.0);
    pub const ONE: Self = Self(1.0);

    pub fn new(value: f64) -> Result<Self, ConfidenceError> {
        if !value.is_finite() {
            return Err(ConfidenceError::NotFinite(value));
        }
        if !(0.0..=1.0).contains(&value) {
            return Err(ConfidenceError::OutOfRange(value));
        }
        Ok(Self(value))
    }

    pub fn value(self) -> f64 {
        self.0
    }

    /// 低于阈值即视为"推测"，用于事实/推测分界（SPEC 5.2.2）。
    pub fn is_speculative(self, threshold: f64) -> bool {
        self.0 < threshold
    }
}

impl TryFrom<f64> for Confidence {
    type Error = ConfidenceError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Confidence> for f64 {
    fn from(value: Confidence) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bounds() {
        assert!(Confidence::new(0.0).is_ok());
        assert!(Confidence::new(1.0).is_ok());
        assert!(Confidence::new(0.5).is_ok());
    }

    #[test]
    fn rejects_out_of_range_and_nan() {
        assert!(Confidence::new(-0.01).is_err());
        assert!(Confidence::new(1.01).is_err());
        assert!(Confidence::new(f64::NAN).is_err());
        assert!(Confidence::new(f64::INFINITY).is_err());
    }
}
