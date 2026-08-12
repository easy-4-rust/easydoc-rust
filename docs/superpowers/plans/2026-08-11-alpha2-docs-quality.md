# v0.1.0-alpha.2 文档与质量增强 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: rust-testing, rust-documentation

**Goal:** 在 alpha.1 基础上新增 MD→DOCX 扩展（front matter / 引用块 / 任务列表）、Writer start_number 支持、OMML→LaTeX `<m:spre>`、proptest fuzzing、fidelity benchmark、SECURITY.md、全 crate 中英文 README、版本号去除。

**Architecture:** `docs/easydoc-rust-Architecture.md`

**Tech Stack:** Rust 1.88+, proptest, criterion, full-stack-doc 标准

## Global Constraints

- MSRV 1.88.0
- `unsafe_code = "forbid"` 全 workspace
- pedantic clippy 全开 + `missing_docs = "warn"`
- rustfmt 100% 合规
- 635 个测试全绿

---

### Task 1: MD→DOCX Front Matter 支持

> Files:
> - Modify: `crates/easydoc-markdown/src/markdown_import.rs`

**Steps:**
- [x] 在 `MarkdownImportParser` 中添加 `in_front_matter` / `front_matter_buffer` 状态
- [x] 检测文件开头 `---` 分隔的 YAML 块
- [x] 解析 `title` / `author` / `subject` / `keywords` 字段
- [x] 自动填入 `DocumentMeta`
- [x] 添加 3 个测试：title+author / 带引号 / front matter + 内容混合

---

### Task 2: MD→DOCX 引用块 `>` 支持

> Files:
> - Modify: `crates/easydoc-markdown/src/markdown_import.rs`

**Steps:**
- [x] 检测 `> text` 语法
- [x] 将引用块识别为斜体段落
- [x] 支持多行引用（连续 `>` 行合并）
- [x] 添加 3 个测试：单行引用 / 多行引用 / 引用后接段落

---

### Task 3: MD→DOCX 任务列表 `- [ ]` / `- [x]` 支持

> Files:
> - Modify: `crates/easydoc-markdown/src/markdown_import.rs`

**Steps:**
- [x] 检测 `- [ ]` / `- [x]` 语法（`is_task_list_item` 辅助函数）
- [x] 输出为带 `☐` / `☑` unicode 前缀的列表项
- [x] 添加 3 个测试：未完成项 / 已完成项 / 混合列表

---

### Task 4: Writer `start_number` 支持

> Files:
> - Modify: `crates/easydoc-writer/src/content_renderer.rs`

**Steps:**
- [x] 实现 `register_custom_start_numbering` 内部函数
- [x] 当 `DocumentList.start_number` 不为 1 时，动态创建独立的 numbering 定义（自定义 abstractNum + numId）
- [x] 避免与默认列表冲突（并发安全的唯一 numId 分配）
- [x] 添加测试：start_number=5 的有序列表

---

### Task 5: OMML→LaTeX `<m:spre>` 支持

> Files:
> - Modify: `crates/easydoc-markdown/src/math/omml_to_latex.rs`

**Steps:**
- [x] 新增 `<m:spre>`（pre-sub-superscript）元素处理
- [x] 转换规则：`<m:spre><m:e>base</m:e><m:sup>top</m:sup><m:sub>bot</m:sub></m:spre>` → `${}^{top}_{bot}base`
- [x] 添加单元测试

---

### Task 6: 列表嵌套 ilvl 跳级测试

> Files:
> - Modify: `crates/easydoc-writer/src/content_renderer.rs`

**Steps:**
- [x] 添加 3 个测试覆盖 ilvl 跳级场景
- [x] 0→2 跳级
- [x] 0→3 跳级
- [x] 跳级 + 兄弟项
- [x] 验证 `attach_to_nested` 的回退行为

---

### Task 7: Writer 写入性能优化快速路径

> Files:
> - Modify: `crates/easydoc-writer/src/executor/table_executor.rs`

**Steps:**
- [x] `apply_xml_extras` 新增快速路径
- [x] 当所有列的 `wrap=true` 且无 `format` 时，跳过整个 XML 后处理
- [x] 避免 O(cells) 次字符串分配

---

### Task 8: proptest fuzzing

> Files:
> - Create: `crates/easydoc-reader/tests/fuzz_docex.rs`
> - Modify: `crates/easydoc-reader/Cargo.toml`

**Steps:**
- [x] 添加 proptest 依赖（workspace = true）
- [x] 实现 8 个 property-based 测试（2048 adversarial cases）
- [x] 覆盖损坏 DOCX / ZIP / URL 输入
- [x] 验证 reader 和 security guards 不 panic

---

### Task 9: fidelity benchmark

> Files:
> - Modify: `crates/easydoc/benches/read_write.rs`
> - Modify: `crates/easydoc/benches/fixtures/table.rs`

**Steps:**
- [x] 实现 5 个 fixture（simple / table / list / rich / image）
- [x] byte-equal 断言嵌入 bench 热循环
- [x] 验证读→写→读往返保真

---

### Task 10: SECURITY.md 与 GitHub 模板

> Files:
> - Create: `SECURITY.md`
> - Create: `.github/ISSUE_TEMPLATE/bug_report.md`
> - Create: `.github/ISSUE_TEMPLATE/feature_request.md`
> - Create: `.github/ISSUE_TEMPLATE/question.md`
> - Create: `.github/pull_request_template.md`

**Steps:**
- [x] SECURITY.md：漏洞报告流程（GitHub Security Advisories）、响应时间承诺、支持版本策略
- [x] bug_report / feature_request / question Issue 模板
- [x] PR 模板

---

### Task 11: ROADMAP.md

> Files:
> - Create: `docs/roadmap.md`

**Steps:**
- [x] 编写 0.1.0-alpha → 1.0.0 路线图（2026 Q2 - 2027 Q4+）
- [x] 分阶段：Phase 1 Infrastructure / Phase 2 Semantic Model / Phase 3 Event Chain / Phase 3.5 Derive + ViewMode / Phase 4 Advanced / Phase 5 Ecosystem
- [x] 标注已完成项

---

### Task 12: 全 crate 中英文 README

> Files:
> - Create: `crates/easydoc/README.md` + `README_zh.md`
> - Create: `crates/easydoc-core/README.md` + `README_zh.md`
> - Create: `crates/easydoc-derive/README.md` + `README_zh.md`
> - Create: `crates/easydoc-ooxml/README.md` + `README_zh.md`
> - Create: `crates/easydoc-reader/README.md` + `README_zh.md`
> - Create: `crates/easydoc-writer/README.md` + `README_zh.md`
> - Create: `crates/easydoc-template/README.md` + `README_zh.md`
> - Create: `crates/easydoc-markdown/README.md` + `README_zh.md`
> - Create: `crates/easydoc-mcp/README.md` + `README_zh.md`

**Steps:**
- [x] 按 full-stack-doc skill 的 Rust README 标准编写
- [x] 剖面组合：文档与文件格式处理 + 上游兼容与移植（Java EasyExcel 4.0.3）+ 大型工具箱 Workspace + 多语言布局
- [x] 每个 README 的支持矩阵分别声明（读/写/编辑/模板填充/往返保真）
- [x] 去除硬编码版本号（使用 `0.1` 而非 `0.1.0-alpha.2`）

---

### Task 13: 版本号去除与发布

> Files:
> - Modify: `docs/usage-guide.md`
> - Modify: 各 crate README

**Steps:**
- [x] 从 18 个 crate README 中去除硬编码版本号
- [x] bump version to 0.1.0-alpha.2
- [x] 编写发布说明

---

## Acceptance / Verification

```bash
cargo test --workspace                    # 635 tests pass
cargo clippy --workspace -- -D warnings   # 0 warnings
cargo fmt --check                         # 100% compliant
cargo test --package easydoc-reader       # proptest 8 cases pass
cargo bench --package easydoc             # fidelity bench pass
```

## 已知局限

- **MD→DOCX** 不支持：HTML 标签、脚注、删除线、数学公式 `$...$`
- **列表嵌套**：不平衡 ilvl（0→2 跳过 1）仍创建中间容器
- **写吞吐**：XML 后处理中 `insert_after_nth` 仍逐 cell 创建新字符串
