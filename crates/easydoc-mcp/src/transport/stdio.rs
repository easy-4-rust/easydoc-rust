//! Stdio transport: newline-delimited JSON over stdin/stdout.
//!
//! This is the standard MCP transport for locally spawned servers.
//! The server reads one JSON-RPC message per line from stdin and writes
//! responses (one per line) to stdout.
//!
//! Uses synchronous I/O — no async runtime required.  This is acceptable
//! because each `EasyDoc` operation is CPU-bound and the stdio pipe is the
//! only I/O surface.

use std::io::{self, BufRead, Write};

use crate::server;

/// Run the MCP server loop on stdin/stdout.
///
/// Blocks until stdin reaches EOF or a fatal I/O error occurs.
///
/// # Errors
///
/// Returns on I/O errors reading from stdin or writing to stdout.
/// Protocol-level errors are sent back as JSON-RPC error responses
/// and do **not** terminate the loop.
pub fn run_stdio_server() -> anyhow::Result<()> {
    let stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    let mut buf = String::new();
    let mut reader = io::BufReader::new(stdin);

    loop {
        buf.clear();
        let bytes_read = reader.read_line(&mut buf)?;
        if bytes_read == 0 {
            // EOF — client closed the pipe.
            break;
        }

        let line = buf.trim();
        if line.is_empty() {
            continue;
        }

        match server::handle_raw(line) {
            Ok(Some(response)) => {
                stdout.write_all(response.as_bytes())?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
            }
            Ok(None) => {
                // Notification — no response.
            }
            Err(e) => {
                // Serialisation failure — send a generic error.
                let err_json = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32603,
                        "message": format!("internal error: {e}"),
                    }
                });
                let err_str = serde_json::to_string(&err_json)?;
                stdout.write_all(err_str.as_bytes())?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}
