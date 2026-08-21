//! JSON-RPC 2.0 and MCP protocol types.
//!
//! Defines the wire format for the Model Context Protocol, which uses
//! JSON-RPC 2.0 messages over stdio.  Types here are serialised directly
//! to/from `serde_json::Value` so that the transport layer stays generic.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 message envelope
// ---------------------------------------------------------------------------

/// A JSON-RPC 2.0 request or notification.
///
/// Notifications are requests without an `id` field — the server must not
/// send a response for them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version — always `"2.0"`.
    pub jsonrpc: String,
    /// Request identifier (integer or string).  Absent for notifications.
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    /// Method name.
    pub method: String,
    /// Optional parameters.
    #[serde(default)]
    pub params: serde_json::Value,
}

/// A JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Protocol version — always `"2.0"`.
    pub jsonrpc: String,
    /// Identifier matching the originating request.
    pub id: serde_json::Value,
    /// Successful result (mutually exclusive with `error`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error result (mutually exclusive with `result`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// A JSON-RPC 2.0 notification (no `id` field, no response expected).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// Protocol version — always `"2.0"`.
    pub jsonrpc: String,
    /// Method name.
    pub method: String,
    /// Optional parameters.
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i32,
    /// Human-readable message.
    pub message: String,
    /// Optional additional data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Well-known error codes
// ---------------------------------------------------------------------------

/// Invalid JSON.
pub const PARSE_ERROR: i32 = -32700;
/// JSON is valid but not a valid JSON-RPC object.
pub const INVALID_REQUEST: i32 = -32600;
/// Method does not exist.
pub const METHOD_NOT_FOUND: i32 = -32601;
/// Invalid method parameters.
pub const INVALID_PARAMS: i32 = -32602;
/// Internal JSON-RPC error.
pub const INTERNAL_ERROR: i32 = -32603;

// ---------------------------------------------------------------------------
// MCP-specific types
// ---------------------------------------------------------------------------

/// Information about this MCP server, returned during `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Human-readable server name.
    pub name: String,
    /// Semver version.
    pub version: String,
}

/// Capabilities advertised by this server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// The server provides tools.
    pub tools: ToolsCapability,
    /// The server provides resources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapabilities>,
    /// The server provides prompt templates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
}

/// Marker that the server supports the `tools` feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    /// Whether the server supports `tools/list_changed` notifications.
    /// We do not support dynamic changes, so this is `false`.
    #[serde(rename = "listChanged", default)]
    pub list_changed: bool,
}

/// 资源能力声明。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapabilities {
    /// 是否支持 `resources/subscribe` 通知。
    ///
    /// 服务器接受 `resources/subscribe` / `resources/unsubscribe` 请求并
    /// 校验资源 URI 存在性；同步 stdio 模型下不主动推送
    /// `notifications/resources/updated`。
    #[serde(default, skip_serializing_if = "is_false")]
    pub subscribe: bool,
}

/// 提示模板能力声明。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    /// 是否支持 `prompts/list_changed` 通知（当前不支持）。
    #[serde(rename = "listChanged", default, skip_serializing_if = "is_false")]
    pub list_changed: bool,
}

/// 用于 JSON 序列化时跳过 `false` 值。
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}

/// The `initialize` request parameters (we ignore most fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    /// Client-reported protocol version.
    #[serde(rename = "protocolVersion")]
    pub protocol_version: Option<String>,
}

/// The `initialize` response result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    /// Protocol version the server supports.
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Server metadata.
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
    /// Advertised capabilities.
    pub capabilities: ServerCapabilities,
}

/// Parameters for `tools/call`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    /// Tool name.
    pub name: String,
    /// Tool arguments (schema depends on the tool).
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// A single content item in a `tools/call` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContent {
    /// Content type — `"text"`, `"image"`, or `"resource"`.
    #[serde(rename = "type")]
    pub content_type: String,
    /// Text payload (when `type` is `"text"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Base64 image data (when `type` is `"image"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// MIME type (when `type` is `"image"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Result of a `tools/call` invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// Content items returned by the tool.
    pub content: Vec<ToolContent>,
    /// Whether this result represents an error.
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

/// `resources/read` 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceParams {
    /// 要读取的资源 URI。
    pub uri: String,
}

/// `prompts/get` 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptParams {
    /// Prompt 模板名称。
    pub name: String,
    /// Prompt 参数（JSON 对象，键为参数名）。
    #[serde(default)]
    pub arguments: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl JsonRpcResponse {
    /// Build a successful response for the given request id.
    #[must_use]
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Build an error response for the given request id.
    #[must_use]
    pub fn error(id: serde_json::Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

impl ToolContent {
    /// Convenience: create a text content block.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content_type: "text".into(),
            text: Some(text.into()),
            data: None,
            mime_type: None,
        }
    }
}

impl ToolCallResult {
    /// Convenience: a successful result with one text block.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(text)],
            is_error: false,
        }
    }

    /// Convenience: an error result with one text block describing the failure.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(message)],
            is_error: true,
        }
    }
}
