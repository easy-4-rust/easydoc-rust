//! Strongly typed user-facing measurement units.

/// A physical length stored internally in twentieths of a point (twips).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Length(i32);

impl Length {
    /// Creates a length from a raw twip value.
    #[must_use]
    pub const fn from_twips(value: i32) -> Self {
        Self(value)
    }

    /// Creates a length from millimetres.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn mm(value: f32) -> Self {
        Self((value * 1440.0 / 25.4).round() as i32)
    }

    /// Creates a length from points.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn pt(value: f32) -> Self {
        Self((value * 20.0).round() as i32)
    }

    /// Creates a length from inches.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn inches(value: f32) -> Self {
        Self((value * 1440.0).round() as i32)
    }

    /// Returns the underlying twip value.
    #[must_use]
    pub const fn twips(self) -> i32 {
        self.0
    }
}

/// A font size expressed in points.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Pt(pub f32);

impl Pt {
    /// Converts the size to OOXML half-points.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn half_points(self) -> usize {
        (self.0.max(0.0) * 2.0).round() as usize
    }
}

impl Default for Pt {
    fn default() -> Self {
        Self(12.0)
    }
}

/// An image dimension expressed in CSS-style pixels.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Px(pub u32);

impl Px {
    /// Converts pixels to English Metric Units at 96 DPI.
    #[must_use]
    pub const fn emu(self) -> u32 {
        self.0.saturating_mul(9_525)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_common_units() {
        assert_eq!(Length::inches(1.0).twips(), 1_440);
        assert_eq!(Length::mm(25.4).twips(), 1_440);
        assert_eq!(Pt(12.0).half_points(), 24);
        assert_eq!(Px(100).emu(), 952_500);
    }
}
