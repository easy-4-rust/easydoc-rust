//! reader 提取器（numbering / image relationships）深度测试。

use easydoc_reader::Numbering;
use easydoc_reader::extractor::image::Relationships;

// ===========================================================================
// Numbering
// ===========================================================================

fn numbering_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
    </w:lvl>
    <w:lvl w:ilvl="1">
      <w:start w:val="1"/>
      <w:numFmt w:val="lowerLetter"/>
    </w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="1">
    <w:lvl w:ilvl="0">
      <w:start w:val="3"/>
      <w:numFmt w:val="bullet"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
  <w:num w:numId="2">
    <w:abstractNumId w:val="1"/>
  </w:num>
</w:numbering>"#
}

#[test]
fn numbering_parse_decimal_level() {
    let n = Numbering::parse(numbering_xml()).expect("parse");
    // numId 1 → abstractNum 0 → ilvl 0 = decimal
    let level = n.lookup(1, 0).expect("level 0 of num 1");
    assert!(level.ordered, "decimal is ordered");
    assert_eq!(level.start, Some(1));
}

#[test]
fn numbering_parse_lower_letter_level() {
    let n = Numbering::parse(numbering_xml()).expect("parse");
    let level = n.lookup(1, 1).expect("level 1 of num 1");
    assert!(level.ordered, "lowerLetter is ordered");
}

#[test]
fn numbering_parse_bullet_with_start_3() {
    let n = Numbering::parse(numbering_xml()).expect("parse");
    let level = n.lookup(2, 0).expect("level 0 of num 2");
    assert!(!level.ordered, "bullet is unordered");
    assert_eq!(level.start, Some(3));
}

#[test]
fn numbering_lookup_missing_num_returns_none() {
    let n = Numbering::parse(numbering_xml()).expect("parse");
    assert!(n.lookup(99, 0).is_none(), "unknown numId should be None");
}

#[test]
fn numbering_lookup_missing_level_returns_none() {
    let n = Numbering::parse(numbering_xml()).expect("parse");
    assert!(n.lookup(1, 5).is_none(), "unknown ilvl should be None");
}

#[test]
fn numbering_parse_empty_xml() {
    let n = Numbering::parse("<w:numbering/>").expect("parse empty");
    assert!(n.lookup(0, 0).is_none());
}

#[test]
fn numbering_parse_garbage_is_err_or_empty() {
    // 垃圾输入不应 panic
    let result = Numbering::parse("<not-numbering>");
    if let Ok(n) = result {
        assert!(n.lookup(0, 0).is_none());
    }
}

// ===========================================================================
// Relationships (image rels)
// ===========================================================================

fn rels_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image2.jpg"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
</Relationships>"#
}

#[test]
fn rels_parse_count() {
    let r = Relationships::parse(rels_xml()).expect("parse rels");
    assert_eq!(r.len(), 3);
    assert!(!r.is_empty());
}

#[test]
fn rels_resolve_image() {
    let r = Relationships::parse(rels_xml()).expect("parse rels");
    assert_eq!(r.resolve_image("rId1"), Some("word/media/image1.png"));
    assert_eq!(r.resolve_image("rId2"), Some("word/media/image2.jpg"));
}

#[test]
fn rels_resolve_hyperlink() {
    let r = Relationships::parse(rels_xml()).expect("parse rels");
    assert_eq!(r.resolve_hyperlink("rId3"), Some("https://example.com"));
}

#[test]
fn rels_resolve_generic() {
    let r = Relationships::parse(rels_xml()).expect("parse rels");
    assert_eq!(r.resolve("rId1"), Some("word/media/image1.png"));
}

#[test]
fn rels_resolve_missing_id() {
    let r = Relationships::parse(rels_xml()).expect("parse rels");
    assert_eq!(r.resolve("nope"), None);
    assert_eq!(r.resolve_image("nope"), None);
}

#[test]
fn rels_empty_relationships() {
    let r = Relationships::parse(r"<Relationships/>").expect("parse");
    assert_eq!(r.len(), 0);
    assert!(r.is_empty());
}

#[test]
fn rels_image_resolution_prefers_image_type() {
    let r = Relationships::parse(rels_xml()).expect("parse rels");
    // rId3 是 hyperlink，resolve_image 应返回 None
    assert_eq!(r.resolve_image("rId3"), None);
}
