//! `DocumentContent` 与 serde 双向序列化桥接层。
//!
//! 核心数据结构（`DocumentBlock` / `DocumentTable` 等）不依赖 serde，
//! 本模块用手动 `impl Serialize / Deserialize` 给它们提供序列化能力。
//!
//! # 使用场景
//!
//! - Web API 序列化：`DocumentContent` → JSON 响应
//! - 配置存储：`DocumentContent` → JSON 文件 → 读回
//! - MCP tool 返回 JSON（见 `crates/easydoc-mcp`）
//!
//! # JSON 格式约定
//!
//! `DocumentBlock` 使用 **tagged enum** 风格：每个块序列化为一个对象，
//! 包含 `"type"` 字段标识变体名。例如：
//!
//! ```json
//! {
//!   "type": "heading",
//!   "level": 1,
//!   "runs": [{"text": "Hello", "bold": true}]
//! }
//! ```

use serde::de::Deserializer;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

use crate::document::{
    DocumentBlock, DocumentContent, DocumentImage, DocumentList, DocumentListItem, DocumentTable,
    DocumentTableCell, DocumentTableRow, DocumentTextRun,
};
use crate::error::{DocError, Result};
use crate::metadata::DocumentMeta;

// ---------------------------------------------------------------------------
// 辅助转换函数
// ---------------------------------------------------------------------------

/// 把 `DocumentContent` 序列化为格式化的 JSON 字符串。
///
/// # Errors
///
/// 当序列化失败时返回 `DocError::Document`。
pub fn to_json(content: &DocumentContent) -> Result<String> {
    serde_json::to_string_pretty(content).map_err(|e| DocError::Document(e.to_string()))
}

/// 从 JSON 字符串反序列化 `DocumentContent`。
///
/// # Errors
///
/// 当 JSON 格式不合法或字段缺失时返回 `DocError::Document`。
pub fn from_json(s: &str) -> Result<DocumentContent> {
    serde_json::from_str(s).map_err(|e| DocError::Document(e.to_string()))
}

/// 把 `DocumentContent` 序列化为 `serde_json::Value`。
///
/// # Errors
///
/// 当序列化失败时返回 `DocError::Document`。
pub fn to_json_value(content: &DocumentContent) -> Result<serde_json::Value> {
    serde_json::to_value(content).map_err(|e| DocError::Document(e.to_string()))
}

/// 从 `serde_json::Value` 反序列化 `DocumentContent`。
///
/// # Errors
///
/// 当 JSON 值结构不匹配时返回 `DocError::Document`。
pub fn from_json_value(value: serde_json::Value) -> Result<DocumentContent> {
    serde_json::from_value(value).map_err(|e| DocError::Document(e.to_string()))
}

// ===========================================================================
// DocumentTextRun
// ===========================================================================

/// 内部辅助结构，用于 `DocumentTextRun` 的 serde 序列化 / 反序列化。
///
/// 与 `DocumentTextRun` 字段一一对应，但派生了 `Serialize` / `Deserialize`。
#[derive(Serialize, Deserialize)]
#[serde(rename = "DocumentTextRun")]
struct TextRunHelper {
    text: String,
    #[serde(default)]
    bold: bool,
    #[serde(default)]
    italic: bool,
    #[serde(default)]
    strikethrough: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hyperlink: Option<String>,
}

impl Serialize for DocumentTextRun {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let helper = TextRunHelper {
            text: self.text.clone(),
            bold: self.bold,
            italic: self.italic,
            strikethrough: self.strikethrough,
            hyperlink: self.hyperlink.clone(),
        };
        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DocumentTextRun {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let helper = TextRunHelper::deserialize(deserializer)?;
        Ok(DocumentTextRun {
            text: helper.text,
            bold: helper.bold,
            italic: helper.italic,
            strikethrough: helper.strikethrough,
            hyperlink: helper.hyperlink,
        })
    }
}

// ===========================================================================
// DocumentImage
// ===========================================================================

/// 内部辅助结构，用于 `DocumentImage` 的 serde 序列化 / 反序列化。
///
/// `data` 字段（`Option<Vec<u8>>`）在 JSON 中序列化为数字数组。
#[derive(Serialize, Deserialize)]
#[serde(rename = "DocumentImage")]
struct ImageHelper {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    alt_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    extension: Option<String>,
}

impl Serialize for DocumentImage {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let helper = ImageHelper {
            alt_text: self.alt_text.clone(),
            data: self.data.clone(),
            extension: self.extension.clone(),
        };
        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DocumentImage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let helper = ImageHelper::deserialize(deserializer)?;
        Ok(DocumentImage {
            alt_text: helper.alt_text,
            data: helper.data,
            extension: helper.extension,
        })
    }
}

// ===========================================================================
// DocumentMeta
// ===========================================================================

/// 内部辅助结构，用于 `DocumentMeta` 的 serde 序列化 / 反序列化。
#[derive(Serialize, Deserialize)]
#[serde(rename = "DocumentMeta")]
struct MetaHelper {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    keywords: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    page_width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    page_height: Option<u32>,
    #[serde(default)]
    landscape: bool,
}

impl Serialize for DocumentMeta {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let helper = MetaHelper {
            title: self.title.clone(),
            author: self.author.clone(),
            subject: self.subject.clone(),
            keywords: self.keywords.clone(),
            page_width: self.page_width,
            page_height: self.page_height,
            landscape: self.landscape,
        };
        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DocumentMeta {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let helper = MetaHelper::deserialize(deserializer)?;
        Ok(DocumentMeta {
            title: helper.title,
            author: helper.author,
            subject: helper.subject,
            keywords: helper.keywords,
            page_width: helper.page_width,
            page_height: helper.page_height,
            landscape: helper.landscape,
        })
    }
}

// ===========================================================================
// DocumentTable / TableRow / TableCell
// ===========================================================================

impl Serialize for DocumentTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("DocumentTable", 1)?;
        s.serialize_field("rows", &self.rows)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for DocumentTable {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let rows = RowsOnly::deserialize(deserializer)?;
        Ok(DocumentTable { rows: rows.rows })
    }
}

#[derive(Deserialize)]
struct RowsOnly {
    rows: Vec<DocumentTableRow>,
}

impl Serialize for DocumentTableRow {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let field_count = if self.is_header { 2 } else { 1 };
        let mut s = serializer.serialize_struct("DocumentTableRow", field_count)?;
        s.serialize_field("cells", &self.cells)?;
        if self.is_header {
            s.serialize_field("is_header", &true)?;
        }
        s.end()
    }
}

impl<'de> Deserialize<'de> for DocumentTableRow {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct RowHelper {
            cells: Vec<DocumentTableCell>,
            #[serde(default)]
            is_header: bool,
        }
        let h = RowHelper::deserialize(deserializer)?;
        Ok(DocumentTableRow {
            cells: h.cells,
            is_header: h.is_header,
        })
    }
}

impl Serialize for DocumentTableCell {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut field_count = 1;
        if self.column_span != 1 {
            field_count += 1;
        }
        if self.row_span != 1 {
            field_count += 1;
        }
        let mut s = serializer.serialize_struct("DocumentTableCell", field_count)?;
        s.serialize_field("blocks", &self.blocks)?;
        if self.column_span != 1 {
            s.serialize_field("column_span", &self.column_span)?;
        }
        if self.row_span != 1 {
            s.serialize_field("row_span", &self.row_span)?;
        }
        s.end()
    }
}

impl<'de> Deserialize<'de> for DocumentTableCell {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct CellHelper {
            blocks: Vec<DocumentBlock>,
            #[serde(default = "one_u32")]
            column_span: u32,
            #[serde(default = "one_u32")]
            row_span: u32,
        }
        let h = CellHelper::deserialize(deserializer)?;
        Ok(DocumentTableCell {
            blocks: h.blocks,
            column_span: h.column_span,
            row_span: h.row_span,
        })
    }
}

fn one_u32() -> u32 {
    1
}

// ===========================================================================
// DocumentList / DocumentListItem
// ===========================================================================

impl Serialize for DocumentList {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut field_count = 2; // ordered + items
        if self.start_number.is_some() {
            field_count += 1;
        }
        let mut s = serializer.serialize_struct("DocumentList", field_count)?;
        s.serialize_field("ordered", &self.ordered)?;
        if let Some(sn) = &self.start_number {
            s.serialize_field("start_number", sn)?;
        }
        s.serialize_field("items", &self.items)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for DocumentList {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct ListHelper {
            ordered: bool,
            #[serde(default)]
            start_number: Option<u32>,
            items: Vec<DocumentListItem>,
        }
        let h = ListHelper::deserialize(deserializer)?;
        Ok(DocumentList {
            ordered: h.ordered,
            start_number: h.start_number,
            items: h.items,
        })
    }
}

impl Serialize for DocumentListItem {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut field_count = 1; // blocks
        if self.nested.is_some() {
            field_count += 1;
        }
        let mut s = serializer.serialize_struct("DocumentListItem", field_count)?;
        s.serialize_field("blocks", &self.blocks)?;
        if let Some(nested) = &self.nested {
            s.serialize_field("nested", nested)?;
        }
        s.end()
    }
}

impl<'de> Deserialize<'de> for DocumentListItem {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct ItemHelper {
            blocks: Vec<DocumentBlock>,
            #[serde(default)]
            nested: Option<Box<DocumentList>>,
        }
        let h = ItemHelper::deserialize(deserializer)?;
        Ok(DocumentListItem {
            blocks: h.blocks,
            nested: h.nested,
        })
    }
}

// ===========================================================================
// DocumentContent
// ===========================================================================

impl Serialize for DocumentContent {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("DocumentContent", 2)?;
        s.serialize_field("metadata", &self.metadata)?;
        s.serialize_field("blocks", &self.blocks)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for DocumentContent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct ContentHelper {
            #[serde(default)]
            metadata: DocumentMeta,
            #[serde(default)]
            blocks: Vec<DocumentBlock>,
        }
        let h = ContentHelper::deserialize(deserializer)?;
        Ok(DocumentContent {
            metadata: h.metadata,
            blocks: h.blocks,
        })
    }
}

// ===========================================================================
// DocumentBlock — tagged enum
// ===========================================================================

/// 序列化变体名称，与 JSON `"type"` 字段对应。
const TAG_HEADING: &str = "heading";
const TAG_PARAGRAPH: &str = "paragraph";
const TAG_TABLE: &str = "table";
const TAG_LIST: &str = "list";
const TAG_IMAGE: &str = "image";
const TAG_THEMATIC_BREAK: &str = "thematic_break";
const TAG_PAGE_BREAK: &str = "page_break";
const TAG_COLUMN_BREAK: &str = "column_break";
const TAG_CODE_BLOCK: &str = "code_block";
const TAG_TEXT_BOX: &str = "text_box";
const TAG_FOOTNOTE: &str = "footnote";
const TAG_ENDNOTE: &str = "endnote";
const TAG_SECTION: &str = "section";
const TAG_MATH: &str = "math";

impl Serialize for DocumentBlock {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            DocumentBlock::Heading { level, runs } => {
                let mut s = serializer.serialize_struct("Block", 3)?;
                s.serialize_field("type", TAG_HEADING)?;
                s.serialize_field("level", level)?;
                s.serialize_field("runs", runs)?;
                s.end()
            }
            DocumentBlock::Paragraph(runs) => {
                let mut s = serializer.serialize_struct("Block", 2)?;
                s.serialize_field("type", TAG_PARAGRAPH)?;
                s.serialize_field("runs", runs)?;
                s.end()
            }
            DocumentBlock::Table(table) => {
                let mut s = serializer.serialize_struct("Block", 2)?;
                s.serialize_field("type", TAG_TABLE)?;
                s.serialize_field("rows", &table.rows)?;
                s.end()
            }
            DocumentBlock::List(list) => {
                let mut s = serializer.serialize_struct("Block", 4)?;
                s.serialize_field("type", TAG_LIST)?;
                s.serialize_field("ordered", &list.ordered)?;
                if let Some(sn) = &list.start_number {
                    s.serialize_field("start_number", sn)?;
                }
                s.serialize_field("items", &list.items)?;
                s.end()
            }
            DocumentBlock::Image(img) => {
                let mut field_count = 1; // type
                if img.alt_text.is_some() {
                    field_count += 1;
                }
                if img.data.is_some() {
                    field_count += 1;
                }
                if img.extension.is_some() {
                    field_count += 1;
                }
                let mut s = serializer.serialize_struct("Block", field_count)?;
                s.serialize_field("type", TAG_IMAGE)?;
                if let Some(alt) = &img.alt_text {
                    s.serialize_field("alt_text", alt)?;
                }
                if let Some(data) = &img.data {
                    s.serialize_field("data", data)?;
                }
                if let Some(ext) = &img.extension {
                    s.serialize_field("extension", ext)?;
                }
                s.end()
            }
            DocumentBlock::ThematicBreak => {
                let mut s = serializer.serialize_struct("Block", 1)?;
                s.serialize_field("type", TAG_THEMATIC_BREAK)?;
                s.end()
            }
            DocumentBlock::PageBreak => {
                let mut s = serializer.serialize_struct("Block", 1)?;
                s.serialize_field("type", TAG_PAGE_BREAK)?;
                s.end()
            }
            DocumentBlock::ColumnBreak => {
                let mut s = serializer.serialize_struct("Block", 1)?;
                s.serialize_field("type", TAG_COLUMN_BREAK)?;
                s.end()
            }
            DocumentBlock::CodeBlock { language, code } => {
                let mut field_count = 2; // type + code
                if language.is_some() {
                    field_count += 1;
                }
                let mut s = serializer.serialize_struct("Block", field_count)?;
                s.serialize_field("type", TAG_CODE_BLOCK)?;
                if let Some(lang) = language {
                    s.serialize_field("language", lang)?;
                }
                s.serialize_field("code", code)?;
                s.end()
            }
            DocumentBlock::TextBox(blocks) => {
                let mut s = serializer.serialize_struct("Block", 2)?;
                s.serialize_field("type", TAG_TEXT_BOX)?;
                s.serialize_field("blocks", blocks)?;
                s.end()
            }
            DocumentBlock::Footnote { id, blocks } => {
                let mut s = serializer.serialize_struct("Block", 3)?;
                s.serialize_field("type", TAG_FOOTNOTE)?;
                s.serialize_field("id", id)?;
                s.serialize_field("blocks", blocks)?;
                s.end()
            }
            DocumentBlock::Endnote { id, blocks } => {
                let mut s = serializer.serialize_struct("Block", 3)?;
                s.serialize_field("type", TAG_ENDNOTE)?;
                s.serialize_field("id", id)?;
                s.serialize_field("blocks", blocks)?;
                s.end()
            }
            DocumentBlock::Section {
                blocks,
                section_type,
            } => {
                let mut field_count = 2; // type + blocks
                if section_type.is_some() {
                    field_count += 1;
                }
                let mut s = serializer.serialize_struct("Block", field_count)?;
                s.serialize_field("type", TAG_SECTION)?;
                s.serialize_field("blocks", blocks)?;
                if let Some(st) = section_type {
                    s.serialize_field("section_type", st)?;
                }
                s.end()
            }
            DocumentBlock::Math {
                omml,
                latex,
                display,
            } => {
                let mut field_count = 2; // type + display
                if omml.is_some() {
                    field_count += 1;
                }
                if latex.is_some() {
                    field_count += 1;
                }
                let mut s = serializer.serialize_struct("Block", field_count)?;
                s.serialize_field("type", TAG_MATH)?;
                if let Some(o) = omml {
                    s.serialize_field("omml", o)?;
                }
                if let Some(l) = latex {
                    s.serialize_field("latex", l)?;
                }
                s.serialize_field("display", display)?;
                s.end()
            }
        }
    }
}

/// 用于 `DocumentBlock` 反序列化的临时标记枚举。
///
/// 使用 `#[serde(tag = "type")]` 自动从 JSON `"type"` 字段判别变体。
#[derive(Deserialize)]
#[serde(tag = "type")]
enum BlockHelper {
    #[serde(rename = "heading")]
    Heading {
        level: u8,
        #[serde(default)]
        runs: Vec<DocumentTextRun>,
    },
    #[serde(rename = "paragraph")]
    Paragraph {
        #[serde(default)]
        runs: Vec<DocumentTextRun>,
    },
    #[serde(rename = "table")]
    Table {
        #[serde(default)]
        rows: Vec<DocumentTableRow>,
    },
    #[serde(rename = "list")]
    List {
        #[serde(default)]
        ordered: bool,
        #[serde(default)]
        start_number: Option<u32>,
        #[serde(default)]
        items: Vec<DocumentListItem>,
    },
    #[serde(rename = "image")]
    Image {
        #[serde(default)]
        alt_text: Option<String>,
        #[serde(default)]
        data: Option<Vec<u8>>,
        #[serde(default)]
        extension: Option<String>,
    },
    #[serde(rename = "thematic_break")]
    ThematicBreak {},
    #[serde(rename = "page_break")]
    PageBreak {},
    #[serde(rename = "column_break")]
    ColumnBreak {},
    #[serde(rename = "code_block")]
    CodeBlock {
        #[serde(default)]
        language: Option<String>,
        #[serde(default)]
        code: String,
    },
    #[serde(rename = "text_box")]
    TextBox {
        #[serde(default)]
        blocks: Vec<DocumentBlock>,
    },
    #[serde(rename = "footnote")]
    Footnote {
        #[serde(default)]
        id: u32,
        #[serde(default)]
        blocks: Vec<DocumentBlock>,
    },
    #[serde(rename = "endnote")]
    Endnote {
        #[serde(default)]
        id: u32,
        #[serde(default)]
        blocks: Vec<DocumentBlock>,
    },
    #[serde(rename = "section")]
    Section {
        #[serde(default)]
        blocks: Vec<DocumentBlock>,
        #[serde(default)]
        section_type: Option<String>,
    },
    #[serde(rename = "math")]
    Math {
        #[serde(default)]
        omml: Option<String>,
        #[serde(default)]
        latex: Option<String>,
        #[serde(default)]
        display: bool,
    },
}

impl<'de> Deserialize<'de> for DocumentBlock {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let helper = BlockHelper::deserialize(deserializer)?;
        Ok(match helper {
            BlockHelper::Heading { level, runs } => DocumentBlock::Heading { level, runs },
            BlockHelper::Paragraph { runs } => DocumentBlock::Paragraph(runs),
            BlockHelper::Table { rows } => DocumentBlock::Table(DocumentTable { rows }),
            BlockHelper::List {
                ordered,
                start_number,
                items,
            } => DocumentBlock::List(DocumentList {
                ordered,
                start_number,
                items,
            }),
            BlockHelper::Image {
                alt_text,
                data,
                extension,
            } => DocumentBlock::Image(DocumentImage {
                alt_text,
                data,
                extension,
            }),
            BlockHelper::ThematicBreak {} => DocumentBlock::ThematicBreak,
            BlockHelper::PageBreak {} => DocumentBlock::PageBreak,
            BlockHelper::ColumnBreak {} => DocumentBlock::ColumnBreak,
            BlockHelper::CodeBlock { language, code } => {
                DocumentBlock::CodeBlock { language, code }
            }
            BlockHelper::TextBox { blocks } => DocumentBlock::TextBox(blocks),
            BlockHelper::Footnote { id, blocks } => DocumentBlock::Footnote { id, blocks },
            BlockHelper::Endnote { id, blocks } => DocumentBlock::Endnote { id, blocks },
            BlockHelper::Section {
                blocks,
                section_type,
            } => DocumentBlock::Section {
                blocks,
                section_type,
            },
            BlockHelper::Math {
                omml,
                latex,
                display,
            } => DocumentBlock::Math {
                omml,
                latex,
                display,
            },
        })
    }
}

// ===========================================================================
// 测试
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- 辅助构造函数 --

    fn heading_block(level: u8, text: &str) -> DocumentBlock {
        DocumentBlock::Heading {
            level,
            runs: vec![DocumentTextRun {
                text: text.into(),
                bold: true,
                ..DocumentTextRun::default()
            }],
        }
    }

    fn paragraph_block(text: &str) -> DocumentBlock {
        DocumentBlock::Paragraph(vec![DocumentTextRun {
            text: text.into(),
            ..DocumentTextRun::default()
        }])
    }

    fn simple_table_block() -> DocumentBlock {
        DocumentBlock::Table(DocumentTable {
            rows: vec![
                DocumentTableRow {
                    cells: vec![
                        DocumentTableCell {
                            blocks: vec![paragraph_block("A")],
                            ..DocumentTableCell::default()
                        },
                        DocumentTableCell {
                            blocks: vec![paragraph_block("B")],
                            ..DocumentTableCell::default()
                        },
                    ],
                    is_header: true,
                },
                DocumentTableRow {
                    cells: vec![
                        DocumentTableCell {
                            blocks: vec![paragraph_block("1")],
                            ..DocumentTableCell::default()
                        },
                        DocumentTableCell {
                            blocks: vec![paragraph_block("2")],
                            ..DocumentTableCell::default()
                        },
                    ],
                    is_header: false,
                },
            ],
        })
    }

    // -- 测试用例 --

    #[test]
    fn roundtrip_heading_via_json() {
        let content = DocumentContent {
            metadata: DocumentMeta::default(),
            blocks: vec![heading_block(1, "Hello")],
        };
        let json = to_json(&content).expect("serialize");
        let back: DocumentContent = from_json(&json).expect("deserialize");
        assert_eq!(content, back);
    }

    #[test]
    fn roundtrip_paragraph_with_runs() {
        let content = DocumentContent {
            metadata: DocumentMeta::default(),
            blocks: vec![DocumentBlock::Paragraph(vec![
                DocumentTextRun {
                    text: "normal ".into(),
                    ..DocumentTextRun::default()
                },
                DocumentTextRun {
                    text: "bold".into(),
                    bold: true,
                    ..DocumentTextRun::default()
                },
                DocumentTextRun {
                    text: " link".into(),
                    hyperlink: Some("https://example.com".into()),
                    ..DocumentTextRun::default()
                },
            ])],
        };
        let json = to_json(&content).expect("serialize");
        let back: DocumentContent = from_json(&json).expect("deserialize");
        assert_eq!(content, back);
    }

    #[test]
    fn roundtrip_table_with_cells() {
        let content = DocumentContent {
            metadata: DocumentMeta::default(),
            blocks: vec![simple_table_block()],
        };
        let json = to_json(&content).expect("serialize");
        let back: DocumentContent = from_json(&json).expect("deserialize");
        assert_eq!(content, back);
    }

    #[test]
    fn roundtrip_nested_list() {
        let content = DocumentContent {
            metadata: DocumentMeta::default(),
            blocks: vec![DocumentBlock::List(DocumentList {
                ordered: true,
                start_number: Some(1),
                items: vec![
                    DocumentListItem {
                        blocks: vec![paragraph_block("Item 1")],
                        nested: None,
                    },
                    DocumentListItem {
                        blocks: vec![paragraph_block("Item 2")],
                        nested: Some(Box::new(DocumentList {
                            ordered: false,
                            start_number: None,
                            items: vec![DocumentListItem {
                                blocks: vec![paragraph_block("Nested")],
                                nested: None,
                            }],
                        })),
                    },
                ],
            })],
        };
        let json = to_json(&content).expect("serialize");
        let back: DocumentContent = from_json(&json).expect("deserialize");
        assert_eq!(content, back);
    }

    #[test]
    fn roundtrip_image_with_data() {
        let content = DocumentContent {
            metadata: DocumentMeta::default(),
            blocks: vec![DocumentBlock::Image(DocumentImage {
                alt_text: Some("photo".into()),
                data: Some(vec![0x89, 0x50, 0x4E, 0x47]),
                extension: Some("png".into()),
            })],
        };
        let json = to_json(&content).expect("serialize");
        let back: DocumentContent = from_json(&json).expect("deserialize");
        assert_eq!(content, back);
    }

    #[test]
    fn roundtrip_math_block() {
        let content = DocumentContent {
            metadata: DocumentMeta::default(),
            blocks: vec![DocumentBlock::Math {
                omml: None,
                latex: Some(r"\frac{1}{2}".into()),
                display: true,
            }],
        };
        let json = to_json(&content).expect("serialize");
        let back: DocumentContent = from_json(&json).expect("deserialize");
        assert_eq!(content, back);
    }

    #[test]
    fn roundtrip_all_block_variants() {
        let blocks = vec![
            heading_block(1, "Title"),
            paragraph_block("text"),
            simple_table_block(),
            DocumentBlock::List(DocumentList {
                ordered: false,
                start_number: None,
                items: vec![DocumentListItem {
                    blocks: vec![paragraph_block("item")],
                    nested: None,
                }],
            }),
            DocumentBlock::Image(DocumentImage {
                alt_text: Some("img".into()),
                data: Some(vec![1, 2, 3]),
                extension: Some("jpg".into()),
            }),
            DocumentBlock::ThematicBreak,
            DocumentBlock::PageBreak,
            DocumentBlock::ColumnBreak,
            DocumentBlock::CodeBlock {
                language: Some("rust".into()),
                code: "fn main() {}".into(),
            },
            DocumentBlock::TextBox(vec![paragraph_block("inside")]),
            DocumentBlock::Footnote {
                id: 1,
                blocks: vec![paragraph_block("note")],
            },
            DocumentBlock::Endnote {
                id: 2,
                blocks: vec![paragraph_block("end")],
            },
            DocumentBlock::Section {
                blocks: vec![paragraph_block("sec")],
                section_type: Some("nextPage".into()),
            },
            DocumentBlock::Math {
                omml: Some("<m:oMath/>".into()),
                latex: None,
                display: false,
            },
        ];
        let content = DocumentContent {
            metadata: DocumentMeta::default(),
            blocks,
        };
        let json = to_json(&content).expect("serialize");
        let back: DocumentContent = from_json(&json).expect("deserialize");
        assert_eq!(content, back);
    }

    #[test]
    fn roundtrip_complex_document() {
        let content = DocumentContent {
            metadata: DocumentMeta {
                title: Some("Test".into()),
                author: Some("Alice".into()),
                subject: Some("Demo".into()),
                keywords: Some("rust,docx".into()),
                page_width: Some(11906),
                page_height: Some(16838),
                landscape: false,
            },
            blocks: vec![
                heading_block(1, "Title"),
                paragraph_block("Body"),
                simple_table_block(),
                DocumentBlock::Math {
                    omml: None,
                    latex: Some("E=mc^2".into()),
                    display: true,
                },
            ],
        };
        let json = to_json(&content).expect("serialize");
        let back: DocumentContent = from_json(&json).expect("deserialize");
        assert_eq!(content, back);
    }

    #[test]
    fn to_json_pretty_format() {
        let content = DocumentContent {
            metadata: DocumentMeta {
                title: Some("Hello".into()),
                ..DocumentMeta::default()
            },
            blocks: vec![paragraph_block("World")],
        };
        let json = to_json(&content).expect("serialize");
        // 格式化 JSON 包含换行
        assert!(json.contains('\n'));
        assert!(json.contains("\"metadata\""));
        assert!(json.contains("\"blocks\""));
        assert!(json.contains("\"Hello\""));
    }

    #[test]
    fn from_json_invalid_input_returns_error() {
        let result = from_json("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn backward_compatible_default_fields() {
        // 缺少 metadata 和 blocks 时应使用默认值
        let json = "{}";
        let content: DocumentContent = from_json(json).expect("deserialize");
        assert_eq!(content.metadata, DocumentMeta::default());
        assert!(content.blocks.is_empty());
    }

    #[test]
    fn roundtrip_json_value() {
        let content = DocumentContent {
            metadata: DocumentMeta {
                title: Some("Value".into()),
                ..DocumentMeta::default()
            },
            blocks: vec![paragraph_block("test")],
        };
        let value = to_json_value(&content).expect("to value");
        let back = from_json_value(value).expect("from value");
        assert_eq!(content, back);
    }

    #[test]
    fn roundtrip_table_with_spans() {
        let content = DocumentContent {
            metadata: DocumentMeta::default(),
            blocks: vec![DocumentBlock::Table(DocumentTable {
                rows: vec![DocumentTableRow {
                    cells: vec![DocumentTableCell {
                        blocks: vec![paragraph_block("span")],
                        column_span: 2,
                        row_span: 3,
                    }],
                    is_header: false,
                }],
            })],
        };
        let json = to_json(&content).expect("serialize");
        assert!(json.contains("\"column_span\""));
        assert!(json.contains("\"row_span\""));
        let back: DocumentContent = from_json(&json).expect("deserialize");
        assert_eq!(content, back);
    }

    #[test]
    fn paragraph_type_tag_present() {
        let content = DocumentContent {
            metadata: DocumentMeta::default(),
            blocks: vec![paragraph_block("hello")],
        };
        let json = to_json(&content).expect("serialize");
        assert!(json.contains(r#""type": "paragraph""#));
    }

    #[test]
    fn heading_type_tag_and_level() {
        let content = DocumentContent {
            metadata: DocumentMeta::default(),
            blocks: vec![heading_block(3, "Sub")],
        };
        let json = to_json(&content).expect("serialize");
        assert!(json.contains(r#""type": "heading""#));
        assert!(json.contains(r#""level": 3"#));
    }
}
