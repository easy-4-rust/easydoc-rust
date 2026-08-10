/// 24 位 RGB 颜色。
///
/// 对应 Java: `com.alibaba.excel.util.ColorUtil` / OOXML `srgbClr` 颜色值
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Red component (0–255).
    pub r: u8,
    /// Green component (0–255).
    pub g: u8,
    /// Blue component (0–255).
    pub b: u8,
}

impl Color {
    /// Creates a color from RGB components.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Creates a color from a 24-bit hex value (e.g. `0xFF0000` = red).
    #[must_use]
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    /// Returns the color as a 24-bit hex value.
    #[must_use]
    pub fn to_hex(self) -> u32 {
        u32::from(self.r) << 16 | u32::from(self.g) << 8 | u32::from(self.b)
    }

    /// Black.
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// White.
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Standard "blue" accent used for table headers.
    pub const HEADER_BLUE: Self = Self::from_hex(0x4472C4);
    /// Dark gray for body text.
    pub const DARK_GRAY: Self = Self::rgb(64, 64, 64);
    /// Red for emphasis.
    pub const RED: Self = Self::from_hex(0xFF0000);
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_constructor() {
        let c = Color::rgb(10, 20, 30);
        assert_eq!(c.r, 10);
        assert_eq!(c.g, 20);
        assert_eq!(c.b, 30);
    }

    #[test]
    fn from_hex_red() {
        let c = Color::from_hex(0xFF0000);
        assert_eq!(c, Color::rgb(255, 0, 0));
    }

    #[test]
    fn from_hex_green() {
        let c = Color::from_hex(0x00FF00);
        assert_eq!(c, Color::rgb(0, 255, 0));
    }

    #[test]
    fn from_hex_blue() {
        let c = Color::from_hex(0x0000FF);
        assert_eq!(c, Color::rgb(0, 0, 255));
    }

    #[test]
    fn to_hex_roundtrip() {
        let c = Color::rgb(0x44, 0x72, 0xC4);
        assert_eq!(c.to_hex(), 0x4472C4);
    }

    #[test]
    fn from_hex_to_hex_roundtrip() {
        let hex = 0xAABBCC;
        let c = Color::from_hex(hex);
        assert_eq!(c.to_hex(), hex);
    }

    #[test]
    fn constants() {
        assert_eq!(Color::BLACK, Color::rgb(0, 0, 0));
        assert_eq!(Color::WHITE, Color::rgb(255, 255, 255));
        assert_eq!(Color::RED, Color::from_hex(0xFF0000));
        assert_eq!(Color::DARK_GRAY, Color::rgb(64, 64, 64));
        assert_eq!(Color::HEADER_BLUE, Color::from_hex(0x4472C4));
    }

    #[test]
    fn default_is_black() {
        let c = Color::default();
        assert_eq!(c, Color::BLACK);
    }

    #[test]
    fn copy_and_eq() {
        let a = Color::rgb(1, 2, 3);
        let b = a;
        assert_eq!(a, b);
    }
}
