//! comrak 导入器测试：验证 AST→语义模型映射。

use easydoc_core::DocumentBlock;
use easydoc_markdown::import_with_comrak;

fn blocks(md: &str) -> Vec<DocumentBlock> {
    import_with_comrak(md).expect("import").blocks
}

// ===========================================================================
// 数学公式（comrak 的核心优势）
// ===========================================================================

#[test]
fn inline_math_dollar() {
    let bs = blocks("inline $x^2$ math");
    // 行内数学以 $...$ 文本形式保留在 run 中
    let DocumentBlock::Paragraph(runs) = &bs[0] else {
        panic!("expected Paragraph");
    };
    let all: String = runs.iter().map(|r| r.text.as_str()).collect();
    assert!(all.contains("$x^2$"), "all: {all}");
}

#[test]
fn block_math_single_line() {
    let bs = blocks("$$x^2$$");
    match &bs[0] {
        DocumentBlock::Math { latex, display, .. } => {
            assert_eq!(latex.as_deref(), Some("x^2"));
            assert!(*display);
        }
        other => panic!("expected Math, got {other:?}"),
    }
}

#[test]
fn block_math_multiline() {
    let bs = blocks("$$\n\\int_0^1 x^2 dx\n$$");
    match &bs[0] {
        DocumentBlock::Math { latex, display, .. } => {
            assert!(
                latex.as_deref().unwrap_or("").contains("\\int"),
                "latex: {latex:?}"
            );
            assert!(*display);
        }
        other => panic!("expected Math, got {other:?}"),
    }
}

#[test]
fn block_math_matrix() {
    let bs = blocks("$$\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}$$");
    match &bs[0] {
        DocumentBlock::Math { latex, .. } => {
            assert!(
                latex.as_deref().unwrap_or("").contains("pmatrix"),
                "latex: {latex:?}"
            );
        }
        other => panic!("expected Math, got {other:?}"),
    }
}

#[test]
fn math_display_vs_inline_distinction() {
    let bs = blocks("$a$ and $$b$$");
    // $a$ → 行内（段落内文本），$$b$$ → Math 块
    assert!(
        bs.iter()
            .any(|b| matches!(b, DocumentBlock::Math { display: true, .. }))
    );
}

// ===========================================================================
// 基础结构
// ===========================================================================

#[test]
fn heading_levels() {
    let bs = blocks("# H1\n## H2\n### H3");
    let levels: Vec<u8> = bs
        .iter()
        .filter_map(|b| match b {
            DocumentBlock::Heading { level, .. } => Some(*level),
            _ => None,
        })
        .collect();
    assert_eq!(levels, vec![1, 2, 3]);
}

#[test]
fn bold_italic_strike() {
    let bs = blocks("**bold** *italic* ~~strike~~");
    let DocumentBlock::Paragraph(runs) = &bs[0] else {
        panic!("expected Paragraph");
    };
    assert!(runs.iter().any(|r| r.bold && r.text == "bold"));
    assert!(runs.iter().any(|r| r.italic && r.text == "italic"));
    assert!(runs.iter().any(|r| r.strikethrough && r.text == "strike"));
}

#[test]
fn link_hyperlink() {
    let bs = blocks("[text](https://e.com)");
    let DocumentBlock::Paragraph(runs) = &bs[0] else {
        panic!("expected Paragraph");
    };
    assert!(
        runs.iter()
            .any(|r| r.hyperlink.as_deref() == Some("https://e.com") && r.text == "text"),
        "runs: {runs:?}"
    );
}

#[test]
fn code_span() {
    let bs = blocks("use `println!`");
    let DocumentBlock::Paragraph(runs) = &bs[0] else {
        panic!("expected Paragraph");
    };
    assert!(runs.iter().any(|r| r.text == "println!"), "runs: {runs:?}");
}

#[test]
fn code_block() {
    let bs = blocks("```rust\nfn main() {}\n```");
    match &bs[0] {
        DocumentBlock::CodeBlock { code, language } => {
            assert!(code.contains("fn main()"), "code: {code}");
            assert_eq!(language.as_deref(), Some("rust"));
        }
        other => panic!("expected CodeBlock, got {other:?}"),
    }
}

// ===========================================================================
// 列表 / 任务列表 / 表格
// ===========================================================================

#[test]
fn unordered_list() {
    let bs = blocks("- a\n- b");
    match &bs[0] {
        DocumentBlock::List(l) => {
            assert!(!l.ordered);
            assert_eq!(l.items.len(), 2);
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn ordered_list_start() {
    let bs = blocks("3. three\n4. four");
    match &bs[0] {
        DocumentBlock::List(l) => {
            assert!(l.ordered);
            assert_eq!(l.start_number, Some(3));
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn task_list_checked_prefix() {
    let bs = blocks("- [x] done\n- [ ] todo");
    match &bs[0] {
        DocumentBlock::List(l) => {
            assert_eq!(l.items.len(), 2);
            // 第一项带 ☑ 前缀
            let first_text: String = l.items[0]
                .blocks
                .iter()
                .filter_map(|b| match b {
                    DocumentBlock::Paragraph(runs) => {
                        Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                    }
                    _ => None,
                })
                .collect();
            assert!(first_text.contains("☑"), "first: {first_text}");
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn table_basic() {
    let bs = blocks("| A | B |\n| --- | --- |\n| 1 | 2 |");
    match &bs[0] {
        DocumentBlock::Table(t) => {
            assert_eq!(t.rows.len(), 2);
            assert!(t.rows[0].is_header);
            assert_eq!(t.rows[0].cells.len(), 2);
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

// ===========================================================================
// 其他块
// ===========================================================================

#[test]
fn thematic_break() {
    let bs = blocks("---");
    assert!(bs.iter().any(|b| matches!(b, DocumentBlock::ThematicBreak)));
}

#[test]
fn image_block() {
    let bs = blocks("![alt text](img.png)");
    match &bs[0] {
        DocumentBlock::Image(img) => {
            assert_eq!(img.alt_text.as_deref(), Some("alt text"));
            assert_eq!(img.extension.as_deref(), Some("png"));
        }
        other => panic!("expected Image, got {other:?}"),
    }
}

#[test]
fn blockquote_textbox() {
    let bs = blocks("> quoted text");
    assert!(bs.iter().any(|b| matches!(b, DocumentBlock::TextBox(_))));
}

#[test]
fn front_matter_metadata() {
    let md = "---\ntitle: My Doc\nauthor: Alice\n---\n\n# Body";
    let content = import_with_comrak(md).expect("import");
    assert_eq!(content.metadata.title.as_deref(), Some("My Doc"));
    assert_eq!(content.metadata.author.as_deref(), Some("Alice"));
    assert!(
        content
            .blocks
            .iter()
            .any(|b| matches!(b, DocumentBlock::Heading { .. }))
    );
}

#[test]
fn empty_document() {
    let bs = blocks("");
    assert!(bs.is_empty());
}

// ===========================================================================
// MD → DOCX（OMML 注入）→ MD 公式往返
// ===========================================================================

#[test]
fn comrak_math_roundtrip_through_docx_omml() {
    let md = "# Formula\n\n$$\\frac{a}{b}$$\n\n$$\\int_0^1 x^2 dx$$";
    let imported = import_with_comrak(md).expect("comrak import");

    // 两个 Math 块
    let math_blocks = imported
        .blocks
        .iter()
        .filter(|b| matches!(b, DocumentBlock::Math { .. }))
        .count();
    assert_eq!(math_blocks, 2, "blocks: {:?}", imported.blocks);

    // 渲染为 DOCX（writer 注入 OMML）——模拟 EasyDoc::write_content 流程
    let dir = tempfile::tempdir().expect("tempdir");
    let docx_path = dir.path().join("math_rt.docx");
    let docx =
        easydoc_writer::content_renderer::render_document_content(&imported).expect("render docx");
    let math = easydoc_writer::content_renderer::take_rendered_math();
    assert_eq!(math.len(), 2, "two math formulas collected: {math:?}");
    let mut xml_docx = docx.build();
    let xml = String::from_utf8_lossy(&xml_docx.document).into_owned();
    xml_docx.document = easydoc_writer::math_omml::postprocess_math_xml(&xml, &math).into_bytes();
    xml_docx
        .pack(std::fs::File::create(&docx_path).expect("create file"))
        .expect("pack docx");

    // 验证 document.xml 含原生 OMML
    let xml = read_document_xml(&docx_path);
    assert!(
        xml.contains("m:oMath"),
        "expected OMML in document.xml, got: {}",
        &xml[..xml.len().min(400)]
    );

    // sax 流式读取还原 Math 块（office_oxide 语义路径不识别 OMML）
    let mut reader = easydoc_reader::DocxSaxReader::from_path(&docx_path).expect("sax reader");
    let readback = reader.read_blocks().expect("sax read");
    let math_count = readback
        .iter()
        .filter(|b| matches!(b, DocumentBlock::Math { .. }))
        .count();
    assert_eq!(
        math_count, 2,
        "sax should restore 2 Math blocks: {readback:?}"
    );

    // Math 块 omml 可转换回 LaTeX（公式往返闭环，精确断言——两方向均为自研转换器）
    let mut roundtrip_latex = Vec::new();
    for b in &readback {
        if let DocumentBlock::Math {
            omml: Some(omml), ..
        } = b
        {
            let latex =
                easydoc_markdown::math::omml_to_latex::convert(omml).expect("omml to latex");
            assert!(!latex.is_empty(), "latex should not be empty");
            roundtrip_latex.push(latex);
        }
    }
    assert_eq!(roundtrip_latex.len(), 2, "two formulas roundtripped");
    assert_eq!(roundtrip_latex[0], r"\frac{a}{b}");
    assert_eq!(roundtrip_latex[1], r"\int_{0}^{1}x^{2}dx");
}

/// 解压 docx 读取 document.xml。
fn read_document_xml(path: &std::path::Path) -> String {
    let file = std::fs::File::open(path).expect("open docx");
    let mut archive = zip::ZipArchive::new(file).expect("open zip");
    let mut entry = archive.by_name("word/document.xml").expect("document.xml");
    let mut buf = String::new();
    std::io::Read::read_to_string(&mut entry, &mut buf).expect("read xml");
    buf
}
