/// A 24-bit RGB color.
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
        (self.r as u32) << 16 | (self.g as u32) << 8 | (self.b as u32)
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
