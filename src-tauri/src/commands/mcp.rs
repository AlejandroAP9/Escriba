//! Comandos del servidor MCP: arrancar/parar y consultar estado. Permite que
//! agentes de IA (Claude Code, Cursor) usen las capacidades locales de Escriba.

use crate::managers::mcp::global;
use serde::Serialize;
use specta::Type;

/// Una llamada a tool en el feed de actividad reciente.
#[derive(Serialize, Clone, Type)]
pub struct McpActivity {
    pub tool: String,
    pub ms: u64,
    pub seconds_ago: u64,
}

/// Un agente que se ha conectado (Claude Code, Cursor, ...).
#[derive(Serialize, Clone, Type)]
pub struct McpClient {
    pub name: String,
    pub version: String,
    pub seconds_ago: u64,
}

/// Conteo de llamadas por tool.
#[derive(Serialize, Clone, Type)]
pub struct McpToolCount {
    pub name: String,
    pub count: u64,
}

#[derive(Serialize, Clone, Type)]
pub struct McpStatus {
    pub running: bool,
    pub port: u16,
    pub url: Option<String>,
    pub uptime_seconds: u64,
    pub total_calls: u64,
    pub tool_counts: Vec<McpToolCount>,
    pub activity: Vec<McpActivity>,
    pub clients: Vec<McpClient>,
}

/// Construye el estado enriquecido (con telemetría) desde el servidor global.
fn build_status() -> McpStatus {
    let s = global();
    McpStatus {
        running: s.is_running(),
        port: s.port(),
        url: s.info().map(|i| i.url),
        uptime_seconds: s.uptime_seconds(),
        total_calls: s.total_calls(),
        tool_counts: s
            .tool_counts()
            .into_iter()
            .map(|c| McpToolCount {
                name: c.name,
                count: c.count,
            })
            .collect(),
        activity: s
            .activity()
            .into_iter()
            .map(|a| McpActivity {
                tool: a.tool,
                ms: a.ms,
                seconds_ago: a.seconds_ago,
            })
            .collect(),
        clients: s
            .clients()
            .into_iter()
            .map(|c| McpClient {
                name: c.name,
                version: c.version,
                seconds_ago: c.seconds_ago,
            })
            .collect(),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn mcp_start(app: tauri::AppHandle) -> Result<McpStatus, String> {
    global().start(app).await?;
    Ok(build_status())
}

#[tauri::command]
#[specta::specta]
pub fn mcp_stop() {
    global().stop();
}

#[tauri::command]
#[specta::specta]
pub fn mcp_status() -> McpStatus {
    build_status()
}
