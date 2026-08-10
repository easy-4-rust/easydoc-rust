//! Parser for `word/numbering.xml` in OOXML documents.
//!
//! Extracts the mapping from `numId` to abstract numbering definitions,
//! including whether each list level is ordered (decimal, roman, etc.) or
//! unordered (bullet), and the start value for ordered lists.

use std::borrow::Cow;
use std::collections::HashMap;

use easydoc_core::{DocError, Result};
use quick_xml::Reader as XmlReader;
use quick_xml::events::Event;

/// Top-level numbering definitions parsed from `word/numbering.xml`.
///
/// OOXML numbering has two layers:
/// - **abstractNum** defines the formatting for each indentation level.
/// - **num** maps a `numId` (referenced by paragraphs) to an `abstractNumId`.
///
/// This struct provides [`lookup`](Self::lookup) to resolve a `(numId, ilvl)`
/// pair to the corresponding [`Level`] information.
#[derive(Debug, Clone, Default)]
pub struct Numbering {
    /// Maps `numId` -> `abstractNumId`.
    pub num_to_abstract: HashMap<u32, u32>,
    /// Maps `abstractNumId` -> its level definitions.
    pub abstract_nums: HashMap<u32, AbstractNum>,
}

/// An abstract numbering definition containing per-level formatting.
#[derive(Debug, Clone, Default)]
pub struct AbstractNum {
    /// Level definitions keyed by indentation level (`ilvl`).
    pub levels: HashMap<u8, Level>,
}

/// Formatting information for a single indentation level in a list.
#[derive(Debug, Clone, Default)]
pub struct Level {
    /// Whether this level uses an ordered format (e.g. decimal, roman).
    /// `false` means bullet/unordered.
    pub ordered: bool,
    /// Start value for ordered lists (e.g. `Some(1)` for numbering starting
    /// at 1).  `None` for bullets or when no `<w:start>` is specified.
    pub start: Option<u32>,
    /// The indentation level (`ilvl`), 0-based.
    pub ilvl: u8,
}

impl Numbering {
    /// Parses a `word/numbering.xml` string into a [`Numbering`] mapping.
    ///
    /// # Errors
    ///
    /// Returns [`DocError::Format`] on XML parse failure.
    pub fn parse(xml: &str) -> Result<Self> {
        let mut numbering = Numbering::default();
        let mut reader = XmlReader::from_reader(xml.as_bytes());
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();

        // State for parsing abstractNum levels.
        let mut current_abstract_id: Option<u32> = None;
        let mut current_ilvl: Option<u8> = None;
        let mut current_num_fmt: Option<String> = None;
        let mut current_start: Option<u32> = None;

        // State for parsing num -> abstractNumId mapping.
        let mut current_num_id: Option<u32> = None;
        let mut in_num: bool = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::Start(ref start)) => {
                    let name = start.name();
                    let local = name.as_ref();
                    match local {
                        b"w:abstractNum" => {
                            current_abstract_id = extract_u32_attr(start, b"w:abstractNumId");
                        }
                        b"w:lvl" => {
                            current_ilvl = extract_u8_attr(start, b"w:ilvl");
                            current_num_fmt = None;
                            current_start = None;
                        }
                        b"w:numFmt" => {
                            current_num_fmt = extract_val_attr(start);
                        }
                        b"w:start" => {
                            current_start =
                                extract_val_attr(start).and_then(|v| v.parse::<u32>().ok());
                        }
                        b"w:num" => {
                            current_num_id = extract_u32_attr(start, b"w:numId");
                            in_num = true;
                        }
                        b"w:abstractNumId" if in_num => {
                            // This is inside <w:num> -- read the val attribute.
                            if let (Some(abstract_id), Some(num_id)) =
                                (extract_val_attr_u32(start), current_num_id)
                            {
                                numbering.num_to_abstract.insert(num_id, abstract_id);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref empty)) => {
                    let name = empty.name();
                    let local = name.as_ref();
                    match local {
                        b"w:numFmt" => {
                            current_num_fmt = extract_val_attr(empty);
                        }
                        b"w:start" => {
                            current_start =
                                extract_val_attr(empty).and_then(|v| v.parse::<u32>().ok());
                        }
                        b"w:abstractNumId" if in_num => {
                            if let (Some(abstract_id), Some(num_id)) =
                                (extract_val_attr_u32(empty), current_num_id)
                            {
                                numbering.num_to_abstract.insert(num_id, abstract_id);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref end)) => {
                    let name = end.name();
                    let local = name.as_ref();
                    match local {
                        b"w:lvl" => {
                            // Finalize the level we were parsing.
                            if let (Some(abstract_id), Some(ilvl)) =
                                (current_abstract_id, current_ilvl)
                            {
                                let ordered =
                                    !matches!(current_num_fmt.as_deref(), Some("bullet") | None);
                                let level = Level {
                                    ordered,
                                    start: current_start,
                                    ilvl,
                                };
                                numbering
                                    .abstract_nums
                                    .entry(abstract_id)
                                    .or_default()
                                    .levels
                                    .insert(ilvl, level);
                            }
                            current_ilvl = None;
                            current_num_fmt = None;
                            current_start = None;
                        }
                        b"w:num" => {
                            current_num_id = None;
                            in_num = false;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    return Err(DocError::Format(format!(
                        "XML parse error in numbering: {e}"
                    )));
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(numbering)
    }

    /// Looks up the level information for a given `numId` and `ilvl`.
    ///
    /// Returns `None` if the `numId` is not mapped, the abstract definition
    /// is missing, or the specific level is not defined.
    #[must_use]
    pub fn lookup(&self, num_id: u32, ilvl: u8) -> Option<&Level> {
        let abstract_id = self.num_to_abstract.get(&num_id)?;
        let abstract_num = self.abstract_nums.get(abstract_id)?;
        abstract_num.levels.get(&ilvl)
    }
}

/// Extracts a `u32` value from a named attribute (e.g. `w:abstractNumId="0"`).
fn extract_u32_attr(tag: &quick_xml::events::BytesStart, attr_name: &[u8]) -> Option<u32> {
    for attr in tag.attributes().flatten() {
        if attr.key.as_ref() == attr_name {
            let val = attr
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()?;
            return val.parse::<u32>().ok();
        }
    }
    None
}

/// Extracts a `u8` value from a named attribute (e.g. `w:ilvl="0"`).
fn extract_u8_attr(tag: &quick_xml::events::BytesStart, attr_name: &[u8]) -> Option<u8> {
    for attr in tag.attributes().flatten() {
        if attr.key.as_ref() == attr_name {
            let val = attr
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()?;
            return val.parse::<u8>().ok();
        }
    }
    None
}

/// Extracts the `w:val` attribute as a `String`.
fn extract_val_attr(tag: &quick_xml::events::BytesStart) -> Option<String> {
    for attr in tag.attributes().flatten() {
        if attr.key.as_ref() == b"w:val" {
            return attr
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .map(Cow::into_owned);
        }
    }
    None
}

/// Extracts the `w:val` attribute as a `u32`.
fn extract_val_attr_u32(tag: &quick_xml::events::BytesStart) -> Option<u32> {
    extract_val_attr(tag).and_then(|v| v.parse::<u32>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_numbering_bullet_lists_unordered() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:numFmt w:val="bullet"/>
      <w:lvlText w:val="&#x2022;"/>
    </w:lvl>
    <w:lvl w:ilvl="1">
      <w:numFmt w:val="bullet"/>
      <w:lvlText w:val="&#x2013;"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;

        let numbering = Numbering::parse(xml).unwrap();
        let level0 = numbering.lookup(1, 0).unwrap();
        assert!(!level0.ordered, "bullet should be unordered");
        let level1 = numbering.lookup(1, 1).unwrap();
        assert!(!level1.ordered, "bullet should be unordered");
    }

    #[test]
    fn parse_numbering_decimal_lists_ordered() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="1">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1."/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="2">
    <w:abstractNumId w:val="1"/>
  </w:num>
</w:numbering>"#;

        let numbering = Numbering::parse(xml).unwrap();
        let level = numbering.lookup(2, 0).unwrap();
        assert!(level.ordered, "decimal should be ordered");
        assert_eq!(level.start, Some(1));
    }

    #[test]
    fn parse_numbering_decimal_with_start_value() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:start w:val="5"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1."/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;

        let numbering = Numbering::parse(xml).unwrap();
        let level = numbering.lookup(1, 0).unwrap();
        assert!(level.ordered);
        assert_eq!(level.start, Some(5), "start should be 5");
    }

    #[test]
    fn lookup_returns_correct_format_for_num_id() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:numFmt w:val="bullet"/>
    </w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="1">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
  <w:num w:numId="2">
    <w:abstractNumId w:val="1"/>
  </w:num>
</w:numbering>"#;

        let numbering = Numbering::parse(xml).unwrap();
        // numId=1 -> abstractNumId=0 -> bullet (unordered)
        assert!(!numbering.lookup(1, 0).unwrap().ordered);
        // numId=2 -> abstractNumId=1 -> decimal (ordered)
        assert!(numbering.lookup(2, 0).unwrap().ordered);
        // Non-existent numId
        assert!(numbering.lookup(99, 0).is_none());
    }

    #[test]
    fn parse_numbering_roman_is_ordered() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="upperRoman"/>
      <w:lvlText w:val="%1."/>
    </w:lvl>
    <w:lvl w:ilvl="1">
      <w:start w:val="1"/>
      <w:numFmt w:val="lowerLetter"/>
      <w:lvlText w:val="%2)"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;

        let numbering = Numbering::parse(xml).unwrap();
        assert!(numbering.lookup(1, 0).unwrap().ordered);
        assert!(numbering.lookup(1, 1).unwrap().ordered);
    }

    #[test]
    fn parse_empty_numbering() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
</w:numbering>"#;

        let numbering = Numbering::parse(xml).unwrap();
        assert!(numbering.num_to_abstract.is_empty());
        assert!(numbering.abstract_nums.is_empty());
        assert!(numbering.lookup(1, 0).is_none());
    }
}
