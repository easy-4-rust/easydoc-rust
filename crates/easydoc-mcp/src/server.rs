//! MCP server 核心：消息路由与协议处理。
//!
//! 解析传入的 JSON-RPC 消息并将其分发到相应的 MCP 方法处理器。
//! 支持 `initialize`、`tools/list`、`tools/call`、`resources/list`、
//! `resources/read`、`prompts/list`、`prompts/get` 方法。

use std::sync::Arc;

use crate::prompts::PromptRenderer;
use crate::protocol::{
    GetPromptParams, INVALID_PARAMS, InitializeResult, JsonRpcRequest, JsonRpcResponse,
    METHOD_NOT_FOUND, PromptsCapability, ReadResourceParams, ResourceCapabilities,
    ServerCapabilities, ServerInfo, ToolCallParams, ToolCallResult, ToolsCapability,
};
use crate::resources::ResourceProvider;
use crate::tools;

/// MCP 协议版本。
const PROTOCOL_VERSION: &str = "2024-11-05";

/// 服务器名称，在 `initialize` 时通告。
const SERVER_NAME: &str = "easydoc-mcp";

/// 服务器版本，在 `initialize` 时通告。
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 服务器配置，持有 resources 和 prompts 的运行时提供者。
///
/// 通过 `ServerConfig` 可以自定义资源和 prompt 的来源，
/// 也可以使用 `default_config()` 获取基于当前目录和内置 prompts 的默认配置。
pub struct ServerConfig {
    /// 资源提供者。
    pub provider: Arc<dyn ResourceProvider>,
    /// Prompt 渲染器。
    pub renderer: Arc<dyn PromptRenderer>,
}

impl ServerConfig {
    /// 创建新的服务器配置。
    pub fn new(provider: Arc<dyn ResourceProvider>, renderer: Arc<dyn PromptRenderer>) -> Self {
        Self { provider, renderer }
    }
}

/// 创建默认服务器配置。
///
/// 使用 `DirectoryResourceProvider`（扫描当前目录的 docx/doc 文件）
/// 和 `BuiltinPrompts`（四个内置 prompt 模板）。
///
/// 扫描根目录可通过环境变量 `EASYDOC_MCP_ROOT` 配置（缺省为当前目录 `.`）；
/// 也可用 [`ServerConfig::new`] 传入自定义的 [`ResourceProvider`]。
#[must_use]
pub fn default_config() -> ServerConfig {
    let root = std::env::var("EASYDOC_MCP_ROOT").unwrap_or_else(|_| ".".to_string());
    default_config_with_root(root)
}

/// 以指定根目录创建默认服务器配置（供 `default_config` 与自定义部署使用）。
#[must_use]
pub fn default_config_with_root(root: impl Into<std::path::PathBuf>) -> ServerConfig {
    ServerConfig::new(
        Arc::new(
            crate::resources::DirectoryResourceProvider::new(root)
                .recursive(true)
                .with_extensions(vec!["docx".into(), "doc".into()]),
        ),
        Arc::new(crate::prompts::BuiltinPrompts::new()),
    )
}

// ---------------------------------------------------------------------------
// 公共入口（带配置）
// ---------------------------------------------------------------------------

/// 使用指定配置处理单条 JSON-RPC 请求。
///
/// 对于通知（无 `id` 的请求）返回 `None`。
#[must_use]
pub fn handle_request_with_config(
    request: &JsonRpcRequest,
    config: &ServerConfig,
) -> Option<JsonRpcResponse> {
    let is_notification = request.id.is_none();
    let id = request.id.clone().unwrap_or(serde_json::Value::Null);

    let response = match request.method.as_str() {
        "initialize" => handle_initialize(&id),
        "notifications/initialized" => {
            return None;
        }
        "ping" => JsonRpcResponse::success(id, serde_json::json!({})),
        "tools/list" => handle_tools_list(&id),
        "tools/call" => handle_tools_call(&id, &request.params),
        "resources/list" => handle_resources_list(&id, config),
        "resources/read" => handle_resources_read(&id, &request.params, config),
        "resources/subscribe" | "resources/unsubscribe" => {
            handle_resources_subscribe(&id, &request.params, config)
        }
        "prompts/list" => handle_prompts_list(&id, config),
        "prompts/get" => handle_prompts_get(&id, &request.params, config),
        other => {
            if is_notification {
                return None;
            }
            JsonRpcResponse::error(id, METHOD_NOT_FOUND, format!("method not found: {other}"))
        }
    };

    if is_notification {
        None
    } else {
        Some(response)
    }
}

/// 使用指定配置处理原始 JSON 字符串并返回响应。
///
/// 通知返回 `None`。
///
/// # 错误
///
/// 仅在响应序列化失败时返回 `Err`（正常情况下不会发生）。
pub fn handle_raw_with_config(raw: &str, config: &ServerConfig) -> anyhow::Result<Option<String>> {
    let request = match parse_request(raw) {
        Ok(r) => r,
        Err(err_resp) => return Ok(Some(serde_json::to_string(&*err_resp)?)),
    };

    let response = handle_request_with_config(&request, config);
    match response {
        Some(resp) => Ok(Some(serde_json::to_string(&resp)?)),
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// 公共入口（默认配置，向后兼容）
// ---------------------------------------------------------------------------

/// 处理单条 JSON-RPC 请求，使用默认配置。
///
/// 对于通知（无 `id` 的请求）返回 `None`。
#[must_use]
pub fn handle_request(request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let config = default_config();
    handle_request_with_config(request, &config)
}

/// 将原始 JSON 字符串解析为 `JsonRpcRequest`。
///
/// # 错误
///
/// 如果 JSON 格式有误，返回 JSON-RPC 解析错误响应。
pub fn parse_request(raw: &str) -> Result<JsonRpcRequest, Box<JsonRpcResponse>> {
    serde_json::from_str::<JsonRpcRequest>(raw).map_err(|e| {
        Box::new(JsonRpcResponse::error(
            serde_json::Value::Null,
            crate::protocol::PARSE_ERROR,
            format!("parse error: {e}"),
        ))
    })
}

/// 便捷函数：处理原始 JSON 字符串并返回 JSON 字符串响应。
///
/// 通知返回 `None`。
///
/// # 错误
///
/// 仅在响应序列化失败时返回 `Err`（正常情况下不会发生）。
pub fn handle_raw(raw: &str) -> anyhow::Result<Option<String>> {
    let config = default_config();
    handle_raw_with_config(raw, &config)
}

// ---------------------------------------------------------------------------
// 方法处理器
// ---------------------------------------------------------------------------

fn handle_initialize(id: &serde_json::Value) -> JsonRpcResponse {
    let result = InitializeResult {
        protocol_version: PROTOCOL_VERSION.into(),
        server_info: ServerInfo {
            name: SERVER_NAME.into(),
            version: SERVER_VERSION.into(),
        },
        capabilities: ServerCapabilities {
            tools: ToolsCapability {
                list_changed: false,
            },
            resources: Some(ResourceCapabilities { subscribe: true }),
            prompts: Some(PromptsCapability {
                list_changed: false,
            }),
        },
    };
    match serde_json::to_value(result) {
        Ok(v) => JsonRpcResponse::success(id.clone(), v),
        Err(e) => JsonRpcResponse::error(
            id.clone(),
            crate::protocol::INTERNAL_ERROR,
            format!("serialisation error: {e}"),
        ),
    }
}

fn handle_tools_list(id: &serde_json::Value) -> JsonRpcResponse {
    let tool_list = tools::tool_definitions();
    match serde_json::to_value(&tool_list) {
        Ok(v) => JsonRpcResponse::success(id.clone(), serde_json::json!({ "tools": v })),
        Err(e) => JsonRpcResponse::error(
            id.clone(),
            crate::protocol::INTERNAL_ERROR,
            format!("serialisation error: {e}"),
        ),
    }
}

fn handle_tools_call(id: &serde_json::Value, params: &serde_json::Value) -> JsonRpcResponse {
    let params: ToolCallParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return JsonRpcResponse::error(
                id.clone(),
                INVALID_PARAMS,
                format!("invalid tools/call params: {e}"),
            );
        }
    };

    let result: ToolCallResult = tools::call_tool(&params.name, &params.arguments);

    match serde_json::to_value(result) {
        Ok(v) => JsonRpcResponse::success(id.clone(), v),
        Err(e) => JsonRpcResponse::error(
            id.clone(),
            crate::protocol::INTERNAL_ERROR,
            format!("serialisation error: {e}"),
        ),
    }
}

fn handle_resources_list(id: &serde_json::Value, config: &ServerConfig) -> JsonRpcResponse {
    let resources = config.provider.list();
    match serde_json::to_value(&resources) {
        Ok(v) => JsonRpcResponse::success(id.clone(), serde_json::json!({ "resources": v })),
        Err(e) => JsonRpcResponse::error(
            id.clone(),
            crate::protocol::INTERNAL_ERROR,
            format!("serialisation error: {e}"),
        ),
    }
}

fn handle_resources_read(
    id: &serde_json::Value,
    params: &serde_json::Value,
    config: &ServerConfig,
) -> JsonRpcResponse {
    let params: ReadResourceParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return JsonRpcResponse::error(
                id.clone(),
                INVALID_PARAMS,
                format!("invalid resources/read params: {e}"),
            );
        }
    };

    match config.provider.read(&params.uri) {
        Ok(Some(contents)) => match serde_json::to_value(&contents) {
            Ok(v) => JsonRpcResponse::success(id.clone(), serde_json::json!({ "contents": v })),
            Err(e) => JsonRpcResponse::error(
                id.clone(),
                crate::protocol::INTERNAL_ERROR,
                format!("serialisation error: {e}"),
            ),
        },
        Ok(None) => JsonRpcResponse::error(
            id.clone(),
            -32002,
            format!("resource not found: {}", params.uri),
        ),
        Err(e) => JsonRpcResponse::error(
            id.clone(),
            crate::protocol::INTERNAL_ERROR,
            format!("resource read error: {e:#}"),
        ),
    }
}

/// 处理 `resources/subscribe` 与 `resources/unsubscribe` 请求。
///
/// 校验请求的 URI 是否为服务器已知资源，返回空结果（MCP 规范要求）。
/// 服务器使用同步 stdio 模型，不主动推送 `notifications/resources/updated`，
/// 客户端需自行在 subscribe 后重新 `resources/read` 获取最新内容。
fn handle_resources_subscribe(
    id: &serde_json::Value,
    params: &serde_json::Value,
    config: &ServerConfig,
) -> JsonRpcResponse {
    let params: ReadResourceParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return JsonRpcResponse::error(
                id.clone(),
                INVALID_PARAMS,
                format!("invalid resources/subscribe params: {e}"),
            );
        }
    };

    // 仅确认 URI 存在；不维护订阅集合（无变化推送）
    match config.provider.read(&params.uri) {
        Ok(Some(_)) => JsonRpcResponse::success(id.clone(), serde_json::json!({})),
        Ok(None) => JsonRpcResponse::error(
            id.clone(),
            -32002,
            format!("resource not found: {}", params.uri),
        ),
        Err(e) => JsonRpcResponse::error(
            id.clone(),
            crate::protocol::INTERNAL_ERROR,
            format!("resource subscribe error: {e:#}"),
        ),
    }
}

fn handle_prompts_list(id: &serde_json::Value, config: &ServerConfig) -> JsonRpcResponse {
    let prompts = config.renderer.list();
    match serde_json::to_value(&prompts) {
        Ok(v) => JsonRpcResponse::success(id.clone(), serde_json::json!({ "prompts": v })),
        Err(e) => JsonRpcResponse::error(
            id.clone(),
            crate::protocol::INTERNAL_ERROR,
            format!("serialisation error: {e}"),
        ),
    }
}

fn handle_prompts_get(
    id: &serde_json::Value,
    params: &serde_json::Value,
    config: &ServerConfig,
) -> JsonRpcResponse {
    let params: GetPromptParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return JsonRpcResponse::error(
                id.clone(),
                INVALID_PARAMS,
                format!("invalid prompts/get params: {e}"),
            );
        }
    };

    match config.renderer.render(&params.name, params.arguments) {
        Ok(messages) => match serde_json::to_value(&messages) {
            Ok(v) => JsonRpcResponse::success(id.clone(), serde_json::json!({ "messages": v })),
            Err(e) => JsonRpcResponse::error(
                id.clone(),
                crate::protocol::INTERNAL_ERROR,
                format!("serialisation error: {e}"),
            ),
        },
        Err(e) => JsonRpcResponse::error(
            id.clone(),
            crate::protocol::INTERNAL_ERROR,
            format!("prompt render error: {e:#}"),
        ),
    }
}
