# easydoc-rust 技术选型文档

> **文档说明**：以 ddd4j-ddd4r 依赖映射对照表为参考，定义 easydoc-rust 各技术域的选型决策。覆盖运行时、序列化、文件处理、错误处理、测试等关键领域。
>
> **Java 基线**：easy4j-easydoc（Apache POI + docx4j）
> **Rust 基线**：easydoc-rust 0.1.0，Rust 1.88+，Edition 2024，Resolver 3
>
> **版本**：V1.0.0
> **最后更新**：2026-08-10

---

## 1. 选型图例

| 标记 | 含义 |
| :--- | :--- |
| ✅ | 已采用，生产就绪 |
| 🔧 | 已采用，需持续评估 |
| ⏳ | 规划中，尚未集成 |
| ❌ | 明确不采用 |

---

## 2. 核心运行时

| 领域 | Java 方案 | Rust 选型 | crate | 版本 | 状态 | 说明 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 同步运行时 | JVM | **std** | — | — | ✅ | 核心库保持同步、确定性 |
| 异步运行时（可选） | Project Reactor | **tokio** | `tokio` | 1.x | ⏳ | integrations 层按需引入 |
| 线程模型 | JVM 线程池 | **std::thread** + RAII | — | — | ✅ | 无需运行时依赖 |

**决策**：easydoc-rust 核心库保持同步 API，不强制 tokio 依赖。异步适配通过 `easydoc-web` / `easydoc-cli` 等集成 crate 引入。

---

## 3. 序列化与数据格式

| 领域 | Java 方案 | Rust 选型 | crate | 版本 | 状态 | 说明 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| JSON | Jackson | **serde** + **serde_json** | `serde` / `serde_json` | 1.x | ✅ | 编译期零成本 derive；模板数据绑定 |
| XML（SAX 流式） | JAXB / DOM4J | **quick-xml** | `quick-xml` | 0.41 | ✅ | OOXML document.xml 解析 |
| XML（DOM 只读） | DOM4J | **roxmltree** | `roxmltree` | 0.20 | ⏳ | 可选：随机访问 XML 节点 |
| ZIP 容器 | java.util.zip | **zip** | `zip` | 8.6 | ✅ | DOCX 包读写；deflate 压缩 |
| 时间 | java.time | **chrono** | `chrono` | 0.4 | ✅ | 文档元数据时间戳 |

**决策**：
- `serde` + `serde_json` 用于模板填充的数据绑定（`fill_template_list` 的 `T: Serialize`）
- `quick-xml` 用于 OOXML XML 解析（规划中，当前通过 `office_oxide` 间接使用）
- `zip` 用于 DOCX ZIP 包操作（`PackageRewriter`、模板填充）

---

## 4. 文件与文档处理

| 领域 | Java 方案 | Rust 选型 | crate | 版本 | 状态 | 说明 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| DOCX 写入 | Apache POI OOXML | **docx-rs** | `docx-rs` | 0.4 | ✅ | DOCX 创建；Builder 模式 |
| DOCX/DOC 读取 | Apache POI HWPF/XWPF | **office_oxide** | `office_oxide` | 0.1 | ✅ | 统一 DOC/DOCX 读取；IR 模型 |
| ZIP 操作 | java.util.zip | **zip** | `zip` | 8.6 | ✅ | 安全重写、资源限制 |
| 临时文件 | File.createTempFile | **tempfile** | `tempfile` | 3.27 | ✅ | 原子写入的临时文件 |
| 图片处理 | ImageIO | **image** | `image` | 0.25 | ⏳ | 图片尺寸计算（规划中） |

**对应关系**（参考 ddd4r 映射表 §9 文件与文档处理）：

| easy-4-j | Rust 对应 | crate | 状态 |
| :--- | :--- | :--- | :--- |
| easy4j-easydoc (POI) | **easydoc-rust** | — | 🔧 |
| easy4j-easypdf | **easypdf-rust** | — | ⏳ |

**决策**：
- `docx-rs` 负责 DOCX 创建（写入路径）
- `office_oxide` 负责 DOC/DOCX 读取（读取路径），提供统一 IR
- `zip` + `tempfile` 负责安全包操作和原子输出
- 三个后端通过 `easydoc-core` 的语义模型解耦

---

## 5. 错误处理

| 领域 | Java 方案 | Rust 选型 | crate | 版本 | 状态 | 说明 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 结构化错误 | Checked Exception | **thiserror** | `thiserror` | 2.0 | ✅ | `#[derive(Error)]` + `DocError` 枚举 |
| 通用错误 | RuntimeException | **anyhow** | `anyhow` | 1.x | ❌ | 库不用 anyhow，留给应用层 |
| 错误传播 | throws / catch | **?** 运算符 | — | — | ✅ | `Result<T, DocError>` |

**决策**：单一 `DocError` 枚举（7 个变体），覆盖 I/O、ZIP、格式、模板、转换、不支持、文档级错误。不使用 `anyhow`，保持库的错误契约清晰。

---

## 6. 并发与线程安全

| 领域 | Java 方案 | Rust 选型 | crate | 版本 | 状态 | 说明 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 共享状态 | ConcurrentHashMap | **Arc + Mutex** | — | — | ✅ | 核心库无需并发 |
| 只读共享 | final / immutable | **Arc\<T\>** | — | — | ✅ | 语义模型不可变共享 |
| 互斥锁 | synchronized | **std::sync::Mutex** | — | — | ✅ | 仅在需要时使用 |

**决策**：核心库保持单线程同步模型，无需 `dashmap` 或 `parking_lot`。并发需求由集成层（easydoc-web）处理。

---

## 7. 构建与宏

| 领域 | Java 方案 | Rust 选型 | crate | 版本 | 状态 | 说明 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 编译期代码生成 | Lombok / MapStruct | **proc-macro2** + **quote** + **syn** | `proc-macro2` / `quote` / `syn` | 1.x / 1.x / 3.x | ✅ | `#[derive(DocxRow)]` 派生宏 |
| 属性解析 | Annotation | **proc-macro-crate** | `proc-macro-crate` | 3.5 | ✅ | 派生宏属性解析 |

**决策**：`#[derive(DocxRow)]` 在编译期生成 `DocxRow` trait 实现，替代 Java 的运行时反射。

---

## 8. 测试

| 领域 | Java 方案 | Rust 选型 | crate | 版本 | 状态 | 说明 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 单元测试 | JUnit 5 | **#[test]** | — | — | ✅ | 内置测试框架 |
| 集成测试 | SpringBootTest | **tests/** 目录 | — | — | ✅ | 端到端测试 |
| 覆盖率 | JaCoCo | **cargo-llvm-cov** | `cargo-llvm-cov` | latest | ✅ | LLVM 源码覆盖率 |
| 属性测试 | jqwik | **proptest** | `proptest` | 1.x | ⏳ | 属性基测试（规划中） |
| Mock | Mockito | **mockall** | `mockall` | 0.13 | ⏳ | trait mock（规划中） |
| 基准测试 | JMH | **criterion** | `criterion` | 0.5 | ⏳ | 性能基准（规划中） |

---

## 9. 可观测性

| 领域 | Java 方案 | Rust 选型 | crate | 版本 | 状态 | 说明 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 日志 | SLF4J + Logback | **tracing** | `tracing` | 0.1 | ⏳ | 结构化日志（集成层引入） |
| 指标 | Micrometer | **opentelemetry_sdk** | `opentelemetry_sdk` | 0.32 | ⏳ | 指标采集（集成层引入） |

**决策**：核心库不引入 tracing 依赖，保持零日志开销。可观测性由集成层按需引入。

---

## 10. 与 ddd4r 映射表的对应关系

| ddd4r 技术域 | easydoc-rust 使用 | 说明 |
| :--- | :--- | :--- |
| §2 核心运行时 | std 同步（无 tokio） | 核心库保持同步 |
| §3 数据库 | ❌ 不涉及 | 纯文件操作库 |
| §5 缓存 | ❌ 不涉及 | — |
| §7 HTTP 客户端 | ❌ 不涉及 | — |
| §8 序列化 | serde + serde_json + quick-xml + zip | 模板数据 + OOXML + ZIP |
| §9 文件处理 | docx-rs + office_oxide + zip + tempfile | 核心文档处理 |
| §19 错误处理 | thiserror | DocError 枚举 |
| §20 并发 | std::sync（最小化） | 无需 tokio/dashmap |
| §21 easy-4-rust 生态 | easydoc-rust 对应 easy4j-easydoc | 文档操作库 |

---

## 11. Cargo.toml 依赖清单

```toml
[workspace.dependencies]
# ═══ 序列化 ═══
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# ═══ 错误处理 ═══
thiserror = "2"

# ═══ 时间 ═══
chrono = { version = "0.4", default-features = false, features = ["std"] }

# ═══ 文件处理 ═══
docx-rs = "0.4"
office_oxide = "0.1"
zip = { version = "8.6", default-features = false, features = ["deflate"] }
tempfile = "3.27"
image = "0.25"

# ═══ 派生宏 ═══
proc-macro2 = "1"
quote = "1"
syn = { version = "3", features = ["full"] }
proc-macro-crate = "3.5"

# ═══ 测试 ═══
# cargo-llvm-cov (外部工具，非 crate 依赖)
```

---

## 12. 决策记录

| ID | 决策 | 理由 | 替代方案 | 状态 |
| :--- | :--- | :--- | :--- | :--- |
| ADR-001 | 核心库保持同步 API | 零运行时依赖，确定性行为 | tokio async | ✅ 已确认 |
| ADR-002 | 使用 docx-rs 而非 docx4j 移植 | 纯 Rust，MIT 许可，社区活跃 | 自研 OOXML 写入 | ✅ 已确认 |
| ADR-003 | 使用 office_oxide 统一读取 | 支持 DOC + DOCX，统一 IR | 分别用 calamine + docx-rs | ✅ 已确认 |
| ADR-004 | 单一 DocError 枚举 | 与 easyexcel-rust 一致的错误模式 | anyhow + thiserror 混用 | ✅ 已确认 |
| ADR-005 | easydoc-core 零引擎依赖 | 解耦语义模型与后端实现 | core 依赖 docx-rs | ✅ 已确认 |
| ADR-006 | atomic 输出策略 | 失败安全，不损坏原文件 | 直接覆盖 | ✅ 已确认 |
| ADR-007 | 模板 XML 转义 | 防止动态值破坏 XML 结构 | 手动转义 | ✅ 已确认 |
| ADR-008 | Markdown 使用独立 crate | 不污染 reader/writer | 内嵌到 reader | ✅ 已确认 |

---

**文档版本**：V1.0.0
**创建日期**：2026-08-10
**最后更新**：2026-08-10
**文档状态**：✅ 已批准
