//! Comandos del servidor MCP: arrancar/parar y consultar estado. Permite que
//! agentes de IA (Claude Code, Cursor) usen las capacidades locales de Escriba.

use crate::managers::mcp::global;
use serde::Serialize;
use specta::Type;

#[derive(Serialize, Clone, Type)]
pub struct McpStatus {
    pub running: bool,
    pub port: u16,
    pub url: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn mcp_start(app: tauri::AppHandle) -> Result<McpStatus, String> {
    let info = global().start(app).await?;
    Ok(McpStatus {
        running: true,
        port: info.port,
        url: Some(info.url),
    })
}

#[tauri::command]
#[specta::specta]
pub fn mcp_stop() {
    global().stop();
}

#[tauri::command]
#[specta::specta]
pub fn mcp_status() -> McpStatus {
    let s = global();
    McpStatus {
        running: s.is_running(),
        port: s.port(),
        url: s.info().map(|i| i.url),
    }
}
