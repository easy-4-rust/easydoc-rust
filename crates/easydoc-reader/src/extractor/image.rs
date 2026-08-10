//! Image extraction helpers for DOCX archives.
//!
//! Parses `word/_rels/document.xml.rels` to build a `relId -> media part path`
//! mapping, and reads raw image bytes from the ZIP archive.

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Read;

use easydoc_core::{DocError, Result};
use quick_xml::Reader as XmlReader;
use quick_xml::events::Event;

/// Parsed relationship mapping from `word/_rels/document.xml.rels`.
///
/// Maps relationship IDs (e.g. `rId5`) to their targets for both image and
/// hyperlink relationship types.  Image targets are resolved to ZIP entry
/// paths (e.g. `word/media/image1.png`), while hyperlink targets are stored
/// as-is (typically an external URL).
pub struct Relationships {
    /// Image relationships: rId -> ZIP entry path (e.g. `word/media/image1.png`).
    rels: HashMap<String, String>,
    /// Hyperlink relationships: rId -> URL (e.g. `https://example.com`).
    hyperlinks: HashMap<String, String>,
}

/// Internal discriminant for relationship types we care about.
enum RelType {
    Image,
    Hyperlink,
    Other,
}

impl Relationships {
    /// Parses relationship XML into image and hyperlink mappings.
    ///
    /// Expects the standard OOXML relationships XML format:
    /// ```xml
    /// <Relationships>
    ///   <Relationship Id="rId5" Target="media/image1.png" Type="...image" />
    ///   <Relationship Id="rId10" Target="https://example.com"
    ///                 Type="...hyperlink" TargetMode="External" />
    /// </Relationships>
    /// ```
    ///
    /// Image targets are resolved to ZIP entry paths (prepended with `word/`).
    /// Hyperlink targets are stored as-is (typically external URLs).
    ///
    /// # Errors
    ///
    /// Returns [`DocError::Format`] on XML parse failure.
    pub fn parse(rels_xml: &str) -> Result<Self> {
        let mut rels = HashMap::new();
        let mut hyperlinks = HashMap::new();
        let mut reader = XmlReader::from_reader(rels_xml.as_bytes());
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::Empty(ref tag)) => {
                    let name = tag.name();
                    if name.as_ref() == b"Relationship" {
                        let mut id = None;
                        let mut target = None;
                        let mut rel_type = RelType::Other;

                        for attr in tag.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"Id" => {
                                    id = attr
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .ok()
                                        .map(Cow::into_owned);
                                }
                                b"Target" => {
                                    target = attr
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .ok()
                                        .map(Cow::into_owned);
                                }
                                b"Type" => {
                                    if let Ok(val) =
                                        attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    {
                                        if val.ends_with("/image") {
                                            rel_type = RelType::Image;
                                        } else if val.ends_with("/hyperlink") {
                                            rel_type = RelType::Hyperlink;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }

                        if let (Some(id), Some(target)) = (id, target) {
                            match rel_type {
                                RelType::Image => {
                                    // Target is relative like "media/image1.png";
                                    // prepend "word/" to form the ZIP entry name.
                                    let full_path = if target.starts_with("media/") {
                                        format!("word/{target}")
                                    } else {
                                        target
                                    };
                                    rels.insert(id, full_path);
                                }
                                RelType::Hyperlink => {
                                    // Hyperlink targets are typically external URLs
                                    // (TargetMode="External"). Store as-is.
                                    hyperlinks.insert(id, target);
                                }
                                RelType::Other => {}
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(DocError::Format(format!(
                        "XML parse error in relationships: {e}"
                    )));
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(Self { rels, hyperlinks })
    }

    /// Resolves a relationship ID to its media part path in the ZIP archive.
    ///
    /// Returns `None` if the relationship ID is not found among image
    /// relationships.  This method is kept for backward compatibility;
    /// prefer [`resolve_image`](Self::resolve_image) for new code.
    #[must_use]
    pub fn resolve(&self, rel_id: &str) -> Option<&str> {
        self.resolve_image(rel_id)
    }

    /// Resolves a relationship ID to its image part path in the ZIP archive.
    ///
    /// Returns `None` if the relationship ID is not found among image
    /// relationships.
    #[must_use]
    pub fn resolve_image(&self, rel_id: &str) -> Option<&str> {
        self.rels.get(rel_id).map(String::as_str)
    }

    /// Resolves a relationship ID to its hyperlink URL.
    ///
    /// Returns `None` if the relationship ID is not found among hyperlink
    /// relationships.
    #[must_use]
    pub fn resolve_hyperlink(&self, rel_id: &str) -> Option<&str> {
        self.hyperlinks.get(rel_id).map(String::as_str)
    }

    /// Returns the number of image relationships parsed (for diagnostics).
    #[must_use]
    pub fn len(&self) -> usize {
        self.rels.len() + self.hyperlinks.len()
    }

    /// Returns `true` if no relationships were parsed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rels.is_empty() && self.hyperlinks.is_empty()
    }
}

/// Reads the raw bytes of a ZIP entry by name.
///
/// # Errors
///
/// Returns [`DocError::Zip`] if the entry is not found or cannot be read.
pub fn read_zip_part<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    part_name: &str,
) -> Result<Vec<u8>> {
    let mut file = archive
        .by_name(part_name)
        .map_err(|e| DocError::Zip(format!("entry '{part_name}' not found: {e}")))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Extracts the file extension (lowercase, without dot) from a filename.
///
/// Returns `None` if the filename has no extension.
#[must_use]
pub fn extension_from_filename(name: &str) -> Option<String> {
    std::path::Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_typical_rels() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Target="styles.xml" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles"/>
  <Relationship Id="rId5" Target="media/image1.png" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"/>
  <Relationship Id="rId6" Target="media/image2.jpeg" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"/>
</Relationships>"#;

        let rels = Relationships::parse(xml).unwrap();
        assert_eq!(rels.len(), 2);
        assert_eq!(rels.resolve("rId5"), Some("word/media/image1.png"));
        assert_eq!(rels.resolve("rId6"), Some("word/media/image2.jpeg"));
        // Non-image relationship should be filtered out.
        assert_eq!(rels.resolve("rId1"), None);
    }

    #[test]
    fn parse_empty_rels() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
</Relationships>"#;

        let rels = Relationships::parse(xml).unwrap();
        assert!(rels.is_empty());
    }

    #[test]
    fn resolve_unknown_relid() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
</Relationships>"#;

        let rels = Relationships::parse(xml).unwrap();
        assert_eq!(rels.resolve("rId999"), None);
    }

    #[test]
    fn extension_from_filename_png() {
        assert_eq!(
            extension_from_filename("image1.png"),
            Some("png".to_owned())
        );
    }

    #[test]
    fn extension_from_filename_jpeg() {
        assert_eq!(
            extension_from_filename("photo.jpeg"),
            Some("jpeg".to_owned())
        );
    }

    #[test]
    fn extension_from_filename_uppercase() {
        assert_eq!(
            extension_from_filename("image1.PNG"),
            Some("png".to_owned())
        );
    }

    #[test]
    fn extension_from_filename_no_extension() {
        assert_eq!(extension_from_filename("README"), None);
    }

    #[test]
    fn extension_from_filename_dotfile() {
        assert_eq!(extension_from_filename(".gitignore"), None);
    }

    #[test]
    fn relationships_includes_hyperlinks() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Target="styles.xml" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles"/>
  <Relationship Id="rId5" Target="media/image1.png" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"/>
  <Relationship Id="rId10" Target="https://example.com" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" TargetMode="External"/>
</Relationships>"#;

        let rels = Relationships::parse(xml).unwrap();
        // 1 image + 1 hyperlink = 2 total
        assert_eq!(rels.len(), 2);
        assert_eq!(rels.resolve_image("rId5"), Some("word/media/image1.png"));
        assert_eq!(rels.resolve_hyperlink("rId10"), Some("https://example.com"));
        // Non-existent IDs return None
        assert_eq!(rels.resolve_hyperlink("rId999"), None);
        assert_eq!(rels.resolve_image("rId10"), None);
    }

    #[test]
    fn relationships_external_target_mode() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId3" Target="https://rust-lang.org" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" TargetMode="External"/>
  <Relationship Id="rId4" Target="mailto:user@example.com" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" TargetMode="External"/>
</Relationships>"#;

        let rels = Relationships::parse(xml).unwrap();
        assert_eq!(
            rels.resolve_hyperlink("rId3"),
            Some("https://rust-lang.org")
        );
        assert_eq!(
            rels.resolve_hyperlink("rId4"),
            Some("mailto:user@example.com")
        );
        // resolve (backward compat) should only find images
        assert_eq!(rels.resolve("rId3"), None);
    }

    #[test]
    fn relationships_backward_compat_resolve_is_alias() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId5" Target="media/image1.png" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"/>
</Relationships>"#;

        let rels = Relationships::parse(xml).unwrap();
        // resolve and resolve_image should return the same thing
        assert_eq!(rels.resolve("rId5"), rels.resolve_image("rId5"));
    }
}
