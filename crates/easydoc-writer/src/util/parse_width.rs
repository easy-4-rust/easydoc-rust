//! CSS-like width string parsing into OOXML width components.

use super::parsed_width::ParsedWidth;

/// Parses a CSS-like width string into OOXML width components.
///
/// Supported formats:
/// - `"2cm"` -- centimetres (1 cm = 567 twips)
/// - `"80px"` -- pixels at 96 DPI (1 px = 15 twips, rounding 15.09)
/// - `"30%"` -- percentage (OOXML `pct` unit = 1/50 of a percent, so 30% = 1500)
/// - `"auto"` -- automatic width
///
/// Returns `None` for unrecognised or empty input.
///
/// # Examples
///
/// ```
/// use easydoc_writer::util::parse_width;
/// use docx_rs::WidthType;
///
/// let w = parse_width("2cm").unwrap();
/// assert_eq!(w.value, 1134);
/// assert_eq!(w.width_type, WidthType::Dxa);
/// ```
#[must_use]
pub fn parse_width(width: &str) -> Option<ParsedWidth> {
    let trimmed = width.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.eq_ignore_ascii_case("auto") {
        return Some(ParsedWidth {
            value: 0,
            width_type: docx_rs::WidthType::Auto,
        });
    }

    // Percentage: "30%"
    if let Some(pct_str) = trimmed.strip_suffix('%') {
        if let Ok(pct) = pct_str.trim().parse::<f64>() {
            // OOXML pct unit: 1% = 50 units, so value = pct * 50
            return Some(ParsedWidth {
                value: (pct * 50.0).round() as usize,
                width_type: docx_rs::WidthType::Pct,
            });
        }
        return None;
    }

    // Centimetres: "2cm"
    if let Some(cm_str) = trimmed.strip_suffix("cm") {
        if let Ok(cm) = cm_str.trim().parse::<f64>() {
            // 1 cm = 567 twips (1/20 of a point; 1 inch = 1440 twips, 1 inch = 2.54 cm)
            return Some(ParsedWidth {
                value: (cm * 567.0).round() as usize,
                width_type: docx_rs::WidthType::Dxa,
            });
        }
        return None;
    }

    // Pixels: "80px" (at 96 DPI: 1 px = 15 twips)
    if let Some(px_str) = trimmed.strip_suffix("px") {
        if let Ok(px) = px_str.trim().parse::<f64>() {
            return Some(ParsedWidth {
                value: (px * 15.0).round() as usize,
                width_type: docx_rs::WidthType::Dxa,
            });
        }
        return None;
    }

    // Bare number treated as twips
    if let Ok(twips) = trimmed.parse::<f64>() {
        return Some(ParsedWidth {
            value: twips.round() as usize,
            width_type: docx_rs::WidthType::Dxa,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_cm() {
        let w = parse_width("2cm").unwrap();
        assert_eq!(w.value, 1134);
        assert_eq!(w.width_type, docx_rs::WidthType::Dxa);
    }

    #[test]
    fn width_cm_decimal() {
        let w = parse_width("1.5cm").unwrap();
        assert_eq!(w.value, 851); // 1.5 * 567 = 850.5 -> 851
        assert_eq!(w.width_type, docx_rs::WidthType::Dxa);
    }

    #[test]
    fn width_px() {
        let w = parse_width("80px").unwrap();
        assert_eq!(w.value, 1200); // 80 * 15 = 1200
        assert_eq!(w.width_type, docx_rs::WidthType::Dxa);
    }

    #[test]
    fn width_pct() {
        let w = parse_width("50%").unwrap();
        assert_eq!(w.value, 2500); // 50 * 50 = 2500
        assert_eq!(w.width_type, docx_rs::WidthType::Pct);
    }

    #[test]
    fn width_pct_decimal() {
        let w = parse_width("33.33%").unwrap();
        assert_eq!(w.value, 1667); // 33.33 * 50 = 1666.5 -> 1667
        assert_eq!(w.width_type, docx_rs::WidthType::Pct);
    }

    #[test]
    fn width_auto() {
        let w = parse_width("auto").unwrap();
        assert_eq!(w.value, 0);
        assert_eq!(w.width_type, docx_rs::WidthType::Auto);
    }

    #[test]
    fn width_auto_case_insensitive() {
        let w = parse_width("Auto").unwrap();
        assert_eq!(w.width_type, docx_rs::WidthType::Auto);
    }

    #[test]
    fn width_empty_returns_none() {
        assert!(parse_width("").is_none());
        assert!(parse_width("  ").is_none());
    }

    #[test]
    fn width_garbage_returns_none() {
        assert!(parse_width("xyz").is_none());
    }

    #[test]
    fn width_whitespace_trimmed() {
        let w = parse_width("  2cm  ").unwrap();
        assert_eq!(w.value, 1134);
    }

    #[test]
    fn width_bare_number_as_twips() {
        let w = parse_width("1134").unwrap();
        assert_eq!(w.value, 1134);
        assert_eq!(w.width_type, docx_rs::WidthType::Dxa);
    }
}
