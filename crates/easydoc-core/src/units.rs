//! 面向用户的强类型度量单位。

/// 以二十分之一磅（twips）内部存储的物理长度。
///
/// 对应 OOXML 中的长度单位（如 `w:pgSz` 页面尺寸）。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Length(i32);

impl Length {
    /// 从原始 twip 值创建长度。
    #[must_use]
    pub const fn from_twips(value: i32) -> Self {
        Self(value)
    }

    /// 从毫米创建长度。
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn mm(value: f32) -> Self {
        Self((value * 1440.0 / 25.4).round() as i32)
    }

    /// 从磅创建长度。
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn pt(value: f32) -> Self {
        Self((value * 20.0).round() as i32)
    }

    /// 从英寸创建长度。
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn inches(value: f32) -> Self {
        Self((value * 1440.0).round() as i32)
    }

    /// 返回底层 twip 值。
    #[must_use]
    pub const fn twips(self) -> i32 {
        self.0
    }
}

/// 以磅为单位的字号。
///
/// 对应 OOXML `<w:sz w:val="..."/>` 中的半磅值。
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Pt(pub f32);

impl Pt {
    /// 将字号转换为 OOXML 半磅值。
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

/// 以 CSS 风格像素表示的图片尺寸。
///
/// 对应 OOXML `<a:ext cx="..." cy="..."/>` 中的 EMU 单位。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Px(pub u32);

impl Px {
    /// 在 96 DPI 下将像素转换为 EMU（English Metric Units）。
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
