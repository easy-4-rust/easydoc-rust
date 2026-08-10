//! Transport layer for MCP message exchange.
//!
//! Currently provides a stdio transport (newline-delimited JSON over
//! stdin/stdout), which is the standard MCP transport for local servers.

pub mod stdio;
