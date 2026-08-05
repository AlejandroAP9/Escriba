use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, Utc};
use log::{debug, error, info, warn};
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_specta::Event;

/// Database migrations for transcription history.
/// Each migration is applied in order. The library tracks which migrations
/// have been applied using SQLite's user_version pragma.
///
/// Note: For users upgrading from tauri-plugin-sql, migrate_from_tauri_plugin_sql()
/// converts the old _sqlx_migrations table tracking to the user_version pragma,
/// ensuring migrations don't re-run on existing databases.
static MIGRATIONS: &[M] = &[
    M::up(
        "CREATE TABLE IF NOT EXISTS transcription_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_name TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            saved BOOLEAN NOT NULL DEFAULT 0,
            title TEXT NOT NULL,
            transcription_text TEXT NOT NULL
        );",
    ),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_processed_text TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_process_prompt TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_process_requested BOOLEAN NOT NULL DEFAULT 0;"),
    // Agregados de uso, que NO se podan.
    //
    // Las estadísticas se calculaban recorriendo `transcription_history`, pero
    // esa tabla se recorta en cada dictado (`cleanup_by_count`, 5 entradas por
    // omisión). El resultado era que "has dictado N veces" y "te has ahorrado X
    // minutos" solo contaban las últimas cinco: los datos no estaban ocultos,
    // estaban BORRADOS. También arrastraba la racha, los días activos y la
    // gráfica de la semana (reporte de Flor, 28-jul-2026).
    //
    // Un balde por día cuesta 365 filas al año y sobrevive a cualquier política
    // de retención. El día es el LOCAL, no el UTC: ver `local_day`.
    M::up(
        "CREATE TABLE IF NOT EXISTS usage_daily (
            day INTEGER PRIMARY KEY,
            transcriptions INTEGER NOT NULL DEFAULT 0,
            words INTEGER NOT NULL DEFAULT 0
        );",
    ),
    // Siembra con lo que quede en el historial. Lo ya podado se perdió y no hay
    // de dónde recuperarlo; esto al menos evita que quien actualiza empiece de
    // cero. El conteo de palabras aquí se aproxima contando espacios, en vez de
    // `split_whitespace`: es una sola pasada sobre unas pocas filas.
    //
    // `julianday(...) - 2440587.5` son los días desde el 1-ene-1970, y con
    // `localtime` salen los del día LOCAL, igual que hace `local_day` en Rust.
    // Quien ya sembró con la 2.2.0 conserva sus filas en UTC (el sembrado no se
    // repite): como mucho, un día de desfase en dictados nocturnos anteriores a
    // la actualización, dentro del margen que este sembrado ya declara.
    M::up(
        "INSERT OR REPLACE INTO usage_daily (day, transcriptions, words)
         SELECT CAST(julianday(timestamp, 'unixepoch', 'localtime') - 2440587.5 AS INTEGER),
                COUNT(*),
                SUM(
                    CASE WHEN trim(COALESCE(NULLIF(post_processed_text, ''), transcription_text)) = ''
                    THEN 0
                    ELSE length(trim(COALESCE(NULLIF(post_processed_text, ''), transcription_text)))
                       - length(replace(trim(COALESCE(NULLIF(post_processed_text, ''), transcription_text)), ' ', ''))
                       + 1
                    END
                )
         FROM transcription_history
         GROUP BY CAST(julianday(timestamp, 'unixepoch', 'localtime') - 2440587.5 AS INTEGER);",
    ),
];

/// Día calendario **local** de una marca de tiempo, numerado como días desde el
/// 1 de enero de 1970 — el mismo esquema que el `timestamp / 86_400` de antes,
/// para que las filas ya escritas sigan siendo comparables.
///
/// El agregado se cubicaba en UTC, pero el gráfico de Inicio etiqueta cada barra
/// con el día LOCAL (`HomeScreen.tsx`, vía `Intl.DateTimeFormat`). En Chile
/// (UTC−4) eso desplaza todo lo dictado entre las 20:00 y medianoche al balde
/// del día siguiente: por la noche, lo dictado esa misma mañana aparecía en la
/// barra de ayer. Y de paso partía un día local en dos días activos, inflando la
/// racha. Se cubica por el día que la persona vivió, que es el que la app le
/// muestra — y esa franja nocturna es justo la hora en que un profesor prepara
/// clases.
fn local_day(timestamp: i64) -> i64 {
    day_in_zone(&Local, timestamp)
}

/// El cálculo de `local_day`, con la zona horaria como parámetro.
///
/// Existe separado solo para poder probarlo: `Local` lee la zona del proceso, y
/// un test escrito contra ella pasaría igual con el cubicado en UTC cuando corre
/// en una máquina en UTC — que es exactamente lo que hace el CI. Con la zona
/// explícita, el test falla si alguien vuelve a `timestamp / 86_400`, corra donde
/// corra.
fn day_in_zone<Tz: chrono::TimeZone>(tz: &Tz, timestamp: i64) -> i64 {
    // `TimeZone` no se importa: el método llega por el propio límite genérico.
    use chrono::NaiveDate;

    // Sin zona horaria resoluble no hay nada mejor que UTC. `div_euclid` en vez
    // de `/` porque la división trunca hacia cero, y antes de 1970 eso no es el
    // día anterior sino el siguiente.
    let fallback = timestamp.div_euclid(86_400);
    let (Some(epoch), Some(dt)) = (
        NaiveDate::from_ymd_opt(1970, 1, 1),
        tz.timestamp_opt(timestamp, 0).single(),
    ) else {
        return fallback;
    };
    dt.date_naive().signed_duration_since(epoch).num_days()
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct PaginatedHistory {
    pub entries: Vec<HistoryEntry>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(tag = "action")]
pub enum HistoryUpdatePayload {
    #[serde(rename = "added")]
    Added { entry: HistoryEntry },
    #[serde(rename = "updated")]
    Updated { entry: HistoryEntry },
    #[serde(rename = "deleted")]
    Deleted { id: i64 },
    #[serde(rename = "toggled")]
    Toggled { id: i64 },
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct HistoryEntry {
    pub id: i64,
    pub file_name: String,
    pub timestamp: i64,
    pub saved: bool,
    pub title: String,
    pub transcription_text: String,
    pub post_processed_text: Option<String>,
    pub post_process_prompt: Option<String>,
    pub post_process_requested: bool,
}

pub struct HistoryManager {
    app_handle: AppHandle,
    recordings_dir: PathBuf,
    db_path: PathBuf,
}

/// Restringe una ruta al dueño: 0700 en carpetas, 0600 en archivos.
///
/// Silencioso a propósito: si el sistema de archivos no soporta permisos POSIX
/// (un volumen exFAT, una unidad de red), no hay nada que hacer y tampoco tiene
/// sentido impedir que la app arranque por eso. En Windows no se aplica: ahí el
/// control es por ACL y el directorio de datos del usuario ya lo está.
fn restrict_to_owner(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let Ok(meta) = fs::metadata(path) else { return };
        let mode = if meta.is_dir() { 0o700 } else { 0o600 };
        if meta.permissions().mode() & 0o777 == mode {
            return;
        }
        if let Err(e) = fs::set_permissions(path, fs::Permissions::from_mode(mode)) {
            debug!(
                "No se pudieron restringir los permisos de {:?}: {}",
                path, e
            );
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

impl HistoryManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Create recordings directory in app data dir
        let app_data_dir = crate::portable::app_data_dir(app_handle)?;
        let recordings_dir = app_data_dir.join("recordings");
        let db_path = app_data_dir.join("history.db");

        // Ensure recordings directory exists
        if !recordings_dir.exists() {
            fs::create_dir_all(&recordings_dir)?;
            debug!("Created recordings directory: {:?}", recordings_dir);
        }

        // Grabaciones e historial solo para el dueño de la cuenta.
        //
        // No sustituye al cifrado en reposo, que sigue pendiente: un proceso que
        // corra COMO este usuario los sigue leyendo. Lo que sí cierra es el caso
        // del equipo compartido, donde otra cuenta del mismo Mac o del mismo
        // Linux podía entrar a leer el historial de dictados por defecto.
        restrict_to_owner(&app_data_dir);
        restrict_to_owner(&recordings_dir);

        let manager = Self {
            app_handle: app_handle.clone(),
            recordings_dir,
            db_path,
        };

        // Initialize database and run migrations synchronously
        manager.init_database()?;

        // La base se crea en init_database, así que sus permisos se ajustan
        // después de que exista.
        restrict_to_owner(&manager.db_path);

        // Cifrado en reposo (PRP-006, Fase 7): el historial heredado en claro
        // se cifra fila a fila. Idempotente (el prefijo esc1: es el marcador),
        // así que un kill a mitad de camino se completa al siguiente arranque
        // y nada se cifra dos veces. Si el llavero no está disponible, no se
        // migra nada y se reintenta en el próximo arranque.
        if let Err(e) = manager.migrate_plaintext_to_encrypted() {
            error!("Migración de cifrado del historial incompleta: {}", e);
        }

        Ok(manager)
    }

    /// Cifra en el lugar toda fila cuyo texto siga en claro. Fila a fila y
    /// re-ejecutable: el prefijo `esc1:` marca lo ya migrado.
    fn migrate_plaintext_to_encrypted(&self) -> Result<()> {
        if !crate::history_crypto::cifrado_disponible() {
            return Ok(());
        }
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, transcription_text, post_processed_text FROM transcription_history
             WHERE transcription_text NOT LIKE 'esc1:%'
                OR (post_processed_text IS NOT NULL AND post_processed_text NOT LIKE 'esc1:%')",
        )?;
        let pendientes: Vec<(i64, String, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        drop(stmt);
        let total = pendientes.len();
        for (id, texto, post) in pendientes {
            conn.execute(
                "UPDATE transcription_history
                 SET transcription_text = ?1, post_processed_text = ?2
                 WHERE id = ?3",
                params![
                    crate::history_crypto::cifrar_campo(&texto),
                    post.as_deref().map(crate::history_crypto::cifrar_campo),
                    id
                ],
            )?;
        }
        if total > 0 {
            info!(
                "Historial: {} entrada(s) migradas a cifrado en reposo",
                total
            );
        }
        Ok(())
    }

    fn init_database(&self) -> Result<()> {
        info!("Initializing database at {:?}", self.db_path);

        let mut conn = Connection::open(&self.db_path)?;

        // Handle migration from tauri-plugin-sql to rusqlite_migration
        // tauri-plugin-sql used _sqlx_migrations table, rusqlite_migration uses user_version pragma
        self.migrate_from_tauri_plugin_sql(&conn)?;

        // Create migrations object and run to latest version
        let migrations = Migrations::new(MIGRATIONS.to_vec());

        // Validate migrations in debug builds
        #[cfg(debug_assertions)]
        migrations.validate().expect("Invalid migrations");

        // Get current version before migration
        let version_before: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        debug!("Database version before migration: {}", version_before);

        // Apply any pending migrations.
        //
        // Una base MÁS NUEVA que el código no puede tumbar la app. Pasó de
        // verdad (28-jul-2026): la 2.2.0 subió el esquema a la versión 6 con la
        // tabla de agregados, y al abrir después la 2.1.1 —que solo conoce 4
        // migraciones— `to_latest` devolvía `DatabaseTooFarAhead`, el `?` lo
        // propagaba hasta el `.expect()` de `initialize_core_logic`, y el
        // pánico cruzando la frontera de ObjC durante
        // `applicationDidFinishLaunching` terminaba en SIGABRT. macOS ofrecía
        // reiniciar y volvía a reventar: bucle de crashes, app inutilizable.
        //
        // Seguir adelante es seguro y es lo correcto: las tablas y columnas que
        // el código de esta versión conoce siguen ahí —las migraciones solo
        // añaden— y lo que no conoce simplemente lo ignora. El precio de
        // abortar es un usuario con la app muerta; el de continuar, una tabla
        // de más sin usar.
        //
        // Esto además hace posible volver atrás de versión, que sin ello era
        // un camino sin retorno para cualquiera que actualizara.
        if let Err(e) = migrations.to_latest(&mut conn) {
            use rusqlite_migration::{Error as MigError, MigrationDefinitionError};
            match e {
                MigError::MigrationDefinition(MigrationDefinitionError::DatabaseTooFarAhead) => {
                    warn!(
                        "El historial fue creado por una versión más nueva de Escriba \
                         (esquema {}, esta versión conoce {}). Se continúa: lo que esta \
                         versión necesita sigue estando.",
                        version_before,
                        MIGRATIONS.len()
                    );
                }
                other => return Err(other.into()),
            }
        }

        // Get version after migration
        let version_after: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if version_after > version_before {
            info!(
                "Database migrated from version {} to {}",
                version_before, version_after
            );
        } else {
            debug!("Database already at latest version {}", version_after);
        }

        Ok(())
    }

    /// Migrate from tauri-plugin-sql's migration tracking to rusqlite_migration's.
    /// tauri-plugin-sql used a _sqlx_migrations table, while rusqlite_migration uses
    /// SQLite's user_version pragma. This function checks if the old system was in use
    /// and sets the user_version accordingly so migrations don't re-run.
    fn migrate_from_tauri_plugin_sql(&self, conn: &Connection) -> Result<()> {
        // Check if the old _sqlx_migrations table exists
        let has_sqlx_migrations: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_sqlx_migrations {
            return Ok(());
        }

        // Check current user_version
        let current_version: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if current_version > 0 {
            // Already migrated to rusqlite_migration system
            return Ok(());
        }

        // Get the highest version from the old migrations table
        let old_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if old_version > 0 {
            info!(
                "Migrating from tauri-plugin-sql (version {}) to rusqlite_migration",
                old_version
            );

            // Set user_version to match the old migration state
            conn.pragma_update(None, "user_version", old_version)?;

            // Optionally drop the old migrations table (keeping it doesn't hurt)
            // conn.execute("DROP TABLE IF EXISTS _sqlx_migrations", [])?;

            info!(
                "Migration tracking converted: user_version set to {}",
                old_version
            );
        }

        Ok(())
    }

    fn get_connection(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }

    fn map_history_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
        // Frontera ÚNICA de descifrado (PRP-006, Fase 7): todo consumidor
        // (UI, export Obsidian, MCP, estadísticas) recibe texto legible o el
        // marcador de no descifrable, sin enterarse del cifrado.
        let transcription_text: String = row.get("transcription_text")?;
        let post_processed_text: Option<String> = row.get("post_processed_text")?;
        Ok(HistoryEntry {
            id: row.get("id")?,
            file_name: row.get("file_name")?,
            timestamp: row.get("timestamp")?,
            saved: row.get("saved")?,
            title: row.get("title")?,
            transcription_text: crate::history_crypto::campo_para_ui(&transcription_text),
            post_processed_text: post_processed_text
                .map(|t| crate::history_crypto::campo_para_ui(&t)),
            post_process_prompt: row.get("post_process_prompt")?,
            post_process_requested: row.get("post_process_requested")?,
        })
    }

    pub fn recordings_dir(&self) -> &std::path::Path {
        &self.recordings_dir
    }

    /// Save a new history entry to the database.
    /// The WAV file should already have been written to the recordings directory.
    pub fn save_entry(
        &self,
        file_name: String,
        transcription_text: String,
        post_process_requested: bool,
        post_processed_text: Option<String>,
        post_process_prompt: Option<String>,
    ) -> Result<HistoryEntry> {
        let timestamp = Utc::now().timestamp();
        let title = self.format_timestamp_title(timestamp);

        // Redacción antes de tocar el disco. Solo afecta a lo que se GUARDA:
        // el texto que ya se pegó en la app de destino salió intacto, porque si
        // el usuario dictó su tarjeta para meterla en un formulario tiene que
        // llegar entera. Lo que no tiene por qué quedarse es la copia en el
        // historial. Ver `crate::redaction` para el alcance exacto.
        let transcription_text = crate::redaction::redact_for_storage(&transcription_text);
        let post_processed_text = post_processed_text
            .as_deref()
            .map(crate::redaction::redact_for_storage);

        // Agregado diario ANTES de insertar: es lo único que sobrevive a la
        // poda, así que tiene que quedar contado aunque esta misma entrada sea
        // la que se recorte dentro de un momento.
        {
            let counted = post_processed_text
                .as_deref()
                .filter(|t| !t.trim().is_empty())
                .unwrap_or(&transcription_text);
            let words = counted.split_whitespace().count() as i64;
            let conn = self.get_connection()?;
            if let Err(e) = conn.execute(
                "INSERT INTO usage_daily (day, transcriptions, words) VALUES (?1, 1, ?2)
                 ON CONFLICT(day) DO UPDATE SET
                    transcriptions = transcriptions + 1,
                    words = words + excluded.words",
                params![local_day(timestamp), words],
            ) {
                // Que fallen las estadísticas no puede impedir que se guarde el
                // dictado, que es lo que el usuario vino a hacer.
                error!("No se pudo actualizar el agregado de uso: {}", e);
            }
        }

        let conn = self.get_connection()?;
        // Cifrado en reposo DESPUÉS de redactar y de contar palabras (el
        // conteo necesita el texto claro) y SOLO para lo que va al disco: la
        // entrada que se devuelve y el evento al frontend llevan texto legible.
        let transcription_cifrada = crate::history_crypto::cifrar_campo(&transcription_text);
        let post_processed_cifrado = post_processed_text
            .as_deref()
            .map(crate::history_crypto::cifrar_campo);
        conn.execute(
            "INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &file_name,
                timestamp,
                false,
                &title,
                &transcription_cifrada,
                &post_processed_cifrado,
                &post_process_prompt,
                post_process_requested,
            ],
        )?;

        let entry = HistoryEntry {
            id: conn.last_insert_rowid(),
            file_name,
            timestamp,
            saved: false,
            title,
            transcription_text,
            post_processed_text,
            post_process_prompt,
            post_process_requested,
        };

        debug!("Saved history entry with id {}", entry.id);

        self.cleanup_old_entries()?;

        // Emit typed event for real-time frontend updates
        if let Err(e) = (HistoryUpdatePayload::Added {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
    }

    /// Update an existing history entry with new transcription results (used by retry).
    pub fn update_transcription(
        &self,
        id: i64,
        transcription_text: String,
        post_processed_text: Option<String>,
        post_process_prompt: Option<String>,
    ) -> Result<HistoryEntry> {
        let conn = self.get_connection()?;
        let updated = conn.execute(
            "UPDATE transcription_history
             SET transcription_text = ?1,
                 post_processed_text = ?2,
                 post_process_prompt = ?3
             WHERE id = ?4",
            params![
                crate::history_crypto::cifrar_campo(&transcription_text),
                post_processed_text
                    .as_deref()
                    .map(crate::history_crypto::cifrar_campo),
                post_process_prompt,
                id
            ],
        )?;

        if updated == 0 {
            return Err(anyhow!("History entry {} not found", id));
        }

        let entry = conn
            .query_row(
                "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                 FROM transcription_history WHERE id = ?1",
                params![id],
                Self::map_history_entry,
            )?;

        debug!("Updated transcription for history entry {}", id);

        if let Err(e) = (HistoryUpdatePayload::Updated {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
    }

    pub fn cleanup_old_entries(&self) -> Result<()> {
        let retention_period = crate::settings::get_recording_retention_period(&self.app_handle);

        match retention_period {
            crate::settings::RecordingRetentionPeriod::Never => {
                // Don't delete anything
                Ok(())
            }
            crate::settings::RecordingRetentionPeriod::PreserveLimit => {
                // Use the old count-based logic with history_limit
                let limit = crate::settings::get_history_limit(&self.app_handle);
                self.cleanup_by_count(limit)
            }
            _ => {
                // Use time-based logic
                self.cleanup_by_time(retention_period)
            }
        }
    }

    /// Resta una entrada del agregado diario de uso.
    ///
    /// Se llama SOLO al borrar a mano, nunca al recortar por retención, y esa
    /// distinción es el fondo del asunto. El recorte automático es una política
    /// de la app: que el agregado le sobreviva es exactamente para lo que se
    /// creó, porque si no las estadísticas volverían a contar solo las últimas
    /// cinco entradas. Borrar a mano es otra cosa: es el usuario diciendo
    /// "quita esto". Seguir contando lo que te pidieron quitar es lo que hacía
    /// que Inicio dijera 9 dictados con 4 en el historial.
    fn discount_from_usage(&self, conn: &Connection, entry: &HistoryEntry) {
        // Mismo criterio de conteo que al guardar; si divergen, los números se
        // separan poco a poco y nadie sabe cuál de los dos miente.
        let counted = entry
            .post_processed_text
            .as_deref()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or(&entry.transcription_text);
        let words = counted.split_whitespace().count() as i64;
        // `MAX(0, ...)` en las dos columnas: el agregado de quien actualizó
        // desde una versión anterior se sembró de un historial ya recortado, así
        // que puede no cuadrar con lo que se borra. Un contador corto es feo; uno
        // en negativo es un fallo a la vista.
        if let Err(e) = conn.execute(
            "UPDATE usage_daily
                SET transcriptions = MAX(0, transcriptions - 1),
                    words = MAX(0, words - ?2)
              WHERE day = ?1",
            params![local_day(entry.timestamp), words],
        ) {
            error!("No se pudo descontar del agregado de uso: {}", e);
            return;
        }
        // Un día que se queda en cero no es un día: era una fila con 0 dictados
        // y 0 palabras que sobrevivía para siempre. No afecta a las cuentas
        // (`compute_usage_stats` solo suma días con `count > 0`), pero ensucia
        // la tabla y confunde a quien la mire por fuera.
        if let Err(e) = conn.execute(
            "DELETE FROM usage_daily WHERE day = ?1 AND transcriptions = 0 AND words = 0",
            params![local_day(entry.timestamp)],
        ) {
            debug!("No se pudo limpiar el día vacío del agregado: {}", e);
        }
    }

    fn delete_entries_and_files(&self, entries: &[(i64, String)]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let conn = self.get_connection()?;
        let mut deleted_count = 0;

        for (id, file_name) in entries {
            // Delete database entry
            conn.execute(
                "DELETE FROM transcription_history WHERE id = ?1",
                params![id],
            )?;

            // Delete WAV file
            let file_path = self.recordings_dir.join(file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete WAV file {}: {}", file_name, e);
                } else {
                    debug!("Deleted old WAV file: {}", file_name);
                    deleted_count += 1;
                }
            }
        }

        Ok(deleted_count)
    }

    fn cleanup_by_count(&self, limit: usize) -> Result<()> {
        let conn = self.get_connection()?;

        // Get all entries that are not saved, ordered by timestamp desc
        let mut stmt = conn.prepare(
            "SELECT id, file_name FROM transcription_history WHERE saved = 0 ORDER BY timestamp DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>("id")?, row.get::<_, String>("file_name")?))
        })?;

        let mut entries: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        if entries.len() > limit {
            let entries_to_delete = &entries[limit..];
            let deleted_count = self.delete_entries_and_files(entries_to_delete)?;

            if deleted_count > 0 {
                debug!("Cleaned up {} old history entries by count", deleted_count);
            }
        }

        Ok(())
    }

    fn cleanup_by_time(
        &self,
        retention_period: crate::settings::RecordingRetentionPeriod,
    ) -> Result<()> {
        let conn = self.get_connection()?;

        // Calculate cutoff timestamp (current time minus retention period)
        let now = Utc::now().timestamp();
        let cutoff_timestamp = match retention_period {
            crate::settings::RecordingRetentionPeriod::Days3 => now - (3 * 24 * 60 * 60), // 3 days in seconds
            crate::settings::RecordingRetentionPeriod::Weeks2 => now - (2 * 7 * 24 * 60 * 60), // 2 weeks in seconds
            crate::settings::RecordingRetentionPeriod::Months3 => now - (3 * 30 * 24 * 60 * 60), // 3 months in seconds (approximate)
            _ => unreachable!("Should not reach here"),
        };

        // Get all unsaved entries older than the cutoff timestamp
        let mut stmt = conn.prepare(
            "SELECT id, file_name FROM transcription_history WHERE saved = 0 AND timestamp < ?1",
        )?;

        let rows = stmt.query_map(params![cutoff_timestamp], |row| {
            Ok((row.get::<_, i64>("id")?, row.get::<_, String>("file_name")?))
        })?;

        let mut entries_to_delete: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries_to_delete.push(row?);
        }

        let deleted_count = self.delete_entries_and_files(&entries_to_delete)?;

        if deleted_count > 0 {
            debug!(
                "Cleaned up {} old history entries based on retention period",
                deleted_count
            );
        }

        Ok(())
    }

    pub async fn get_history_entries(
        &self,
        cursor: Option<i64>,
        limit: Option<usize>,
    ) -> Result<PaginatedHistory> {
        let conn = self.get_connection()?;
        let limit = limit.map(|l| l.min(100));

        let mut entries: Vec<HistoryEntry> = match (cursor, limit) {
            (Some(cursor_id), Some(lim)) => {
                let fetch_count = (lim + 1) as i64;
                let mut stmt = conn.prepare(
                    "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                     FROM transcription_history
                     WHERE id < ?1
                     ORDER BY id DESC
                     LIMIT ?2",
                )?;
                let result = stmt
                    .query_map(params![cursor_id, fetch_count], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
            (None, Some(lim)) => {
                let fetch_count = (lim + 1) as i64;
                let mut stmt = conn.prepare(
                    "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                     FROM transcription_history
                     ORDER BY id DESC
                     LIMIT ?1",
                )?;
                let result = stmt
                    .query_map(params![fetch_count], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
            (_, None) => {
                let mut stmt = conn.prepare(
                    "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                     FROM transcription_history
                     ORDER BY id DESC",
                )?;
                let result = stmt
                    .query_map([], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
        };

        let has_more = limit.is_some_and(|lim| entries.len() > lim);
        if has_more {
            entries.pop();
        }

        Ok(PaginatedHistory { entries, has_more })
    }

    #[cfg(test)]
    fn get_latest_entry_with_conn(conn: &Connection) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(
            "SELECT
                id,
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
             FROM transcription_history
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let entry = stmt.query_row([], Self::map_history_entry).optional()?;
        Ok(entry)
    }

    /// Get the latest entry with non-empty transcription text.
    pub fn get_latest_completed_entry(&self) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        Self::get_latest_completed_entry_with_conn(&conn)
    }

    fn get_latest_completed_entry_with_conn(conn: &Connection) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(
            "SELECT
                id,
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
             FROM transcription_history
             WHERE transcription_text != ''
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let entry = stmt.query_row([], Self::map_history_entry).optional()?;
        Ok(entry)
    }

    pub async fn toggle_saved_status(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get current saved status
        let current_saved: bool = conn.query_row(
            "SELECT saved FROM transcription_history WHERE id = ?1",
            params![id],
            |row| row.get("saved"),
        )?;

        let new_saved = !current_saved;

        conn.execute(
            "UPDATE transcription_history SET saved = ?1 WHERE id = ?2",
            params![new_saved, id],
        )?;

        debug!("Toggled saved status for entry {}: {}", id, new_saved);

        // Emit history updated event
        if let Err(e) = (HistoryUpdatePayload::Toggled { id }).emit(&self.app_handle) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    pub fn get_audio_file_path(&self, file_name: &str) -> PathBuf {
        self.recordings_dir.join(file_name)
    }

    pub async fn get_entry_by_id(&self, id: i64) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT
                id,
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
             FROM transcription_history
             WHERE id = ?1",
        )?;

        let entry = stmt.query_row([id], Self::map_history_entry).optional()?;

        Ok(entry)
    }

    pub async fn delete_entry(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get the entry to find the file name
        if let Some(entry) = self.get_entry_by_id(id).await? {
            // Delete the audio file first
            let file_path = self.get_audio_file_path(&entry.file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete audio file {}: {}", entry.file_name, e);
                    // Continue with database deletion even if file deletion fails
                }
            }
            self.discount_from_usage(&conn, &entry);
        }

        // Delete from database
        conn.execute(
            "DELETE FROM transcription_history WHERE id = ?1",
            params![id],
        )?;

        debug!("Deleted history entry with id: {}", id);

        // Emit history updated event
        if let Err(e) = (HistoryUpdatePayload::Deleted { id }).emit(&self.app_handle) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    fn format_timestamp_title(&self, timestamp: i64) -> String {
        if let Some(utc_datetime) = DateTime::from_timestamp(timestamp, 0) {
            // Convert UTC to local timezone
            let local_datetime = utc_datetime.with_timezone(&Local);
            local_datetime.format("%B %e, %Y - %l:%M%p").to_string()
        } else {
            format!("Recording {}", timestamp)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE transcription_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                saved BOOLEAN NOT NULL DEFAULT 0,
                title TEXT NOT NULL,
                transcription_text TEXT NOT NULL,
                post_processed_text TEXT,
                post_process_prompt TEXT,
                post_process_requested BOOLEAN NOT NULL DEFAULT 0
            );",
        )
        .expect("create transcription_history table");
        conn
    }

    fn insert_entry(conn: &Connection, timestamp: i64, text: &str, post_processed: Option<&str>) {
        conn.execute(
            "INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                format!("handy-{}.wav", timestamp),
                timestamp,
                false,
                format!("Recording {}", timestamp),
                text,
                post_processed,
                Option::<String>::None,
                false,
            ],
        )
        .expect("insert history entry");
    }

    #[test]
    fn get_latest_entry_returns_none_when_empty() {
        let conn = setup_conn();
        let entry = HistoryManager::get_latest_entry_with_conn(&conn).expect("fetch latest entry");
        assert!(entry.is_none());
    }

    #[test]
    fn get_latest_entry_returns_newest_entry() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "first", None);
        insert_entry(&conn, 200, "second", Some("processed"));

        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .expect("fetch latest entry")
            .expect("entry exists");

        assert_eq!(entry.timestamp, 200);
        assert_eq!(entry.transcription_text, "second");
        assert_eq!(entry.post_processed_text.as_deref(), Some("processed"));
    }

    #[test]
    fn get_latest_completed_entry_skips_empty_entries() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "completed", None);
        insert_entry(&conn, 200, "", None);

        let entry = HistoryManager::get_latest_completed_entry_with_conn(&conn)
            .expect("fetch latest completed entry")
            .expect("completed entry exists");

        assert_eq!(entry.timestamp, 100);
        assert_eq!(entry.transcription_text, "completed");
    }

    /// Una base creada por una versión MÁS NUEVA no puede impedir el arranque.
    ///
    /// Es el crash en bucle del 28-jul-2026: la 2.2.0 subió el esquema a 6 y al
    /// abrir la 2.1.1, que solo conoce 4 migraciones, `to_latest` devolvía
    /// `DatabaseTooFarAhead` y la app moría antes de pintar nada. El test fija
    /// la regla: ese error concreto se tolera, cualquier otro no.
    #[test]
    fn a_newer_database_is_tolerated_not_fatal() {
        use rusqlite_migration::{Error as MigError, MigrationDefinitionError, Migrations, M};

        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        // Base escrita por una versión futura: más migraciones aplicadas de las
        // que este código conoce.
        conn.pragma_update(None, "user_version", 6)
            .expect("set user_version");

        let known = Migrations::new(vec![
            M::up("CREATE TABLE IF NOT EXISTS a (id INTEGER PRIMARY KEY);"),
            M::up("CREATE TABLE IF NOT EXISTS b (id INTEGER PRIMARY KEY);"),
        ]);

        let err = known.to_latest(&mut conn).expect_err("debería quejarse");
        assert!(
            matches!(
                err,
                MigError::MigrationDefinition(MigrationDefinitionError::DatabaseTooFarAhead)
            ),
            "la librería cambió el error para una base adelantada: {err:?}"
        );
    }

    /// Un "hoy" fijo para que los tests no dependan del reloj: día 1000.
    const TODAY: i64 = 1000;

    /// El desfase que motivó `local_day`, con la zona de Chile fijada para que
    /// el test signifique lo mismo aquí y en el CI (que corre en UTC).
    #[test]
    fn day_buckets_by_the_day_the_person_lived_not_utc() {
        use chrono::{FixedOffset, TimeZone};

        let chile = FixedOffset::west_opt(4 * 3600).expect("UTC−4 es un huso válido");
        let at = |h, m| {
            chile
                .with_ymd_and_hms(2026, 7, 24, h, m, 0)
                .single()
                .expect("24-jul-2026 no tiene saltos horarios en UTC−4")
                .timestamp()
        };

        // 24-jul-2026 son 20658 días desde el 1-ene-1970.
        assert_eq!(day_in_zone(&chile, at(12, 0)), 20658);

        // Las 21:00 del MISMO día local: en UTC ya es el 25, y ahí estaba el
        // fallo. Con `timestamp / 86_400` esto daría 20659 y aparecería en la
        // barra del sábado habiéndose dictado el viernes.
        assert_eq!(
            day_in_zone(&chile, at(21, 0)),
            20658,
            "lo dictado de noche debe quedar en el día local, no en el UTC"
        );

        // Y la frontera por el otro lado: las 00:01 siguen siendo el día 24.
        assert_eq!(day_in_zone(&chile, at(0, 1)), 20658);
    }

    #[test]
    fn stats_accumulate_beyond_the_history_limit() {
        // El fallo que reportó Flor: el historial guarda 5 entradas, pero las
        // estadísticas tienen que contar los 120 dictados de siempre.
        let buckets: Vec<(i64, i64, i64)> = (0..40).map(|i| (1000 - i, 3, 120)).collect();
        let stats = compute_usage_stats(&buckets, TODAY);

        assert_eq!(stats.total_transcriptions, 120);
        assert_eq!(stats.total_words, 4800);
        // 4800 palabras * (1/52 - 1/200) min ahorrados por palabra (52 ppm de
        // tecleo segun Dhakal et al., CHI 2018; ~200 ppm de habla) = 68,3.
        assert_eq!(stats.minutes_saved, 68);
    }

    #[test]
    fn stats_streak_counts_consecutive_days_backwards() {
        // Hoy, ayer y anteayer; luego un hueco.
        let buckets = vec![(1000, 1, 10), (999, 1, 10), (998, 1, 10), (995, 1, 10)];
        assert_eq!(compute_usage_stats(&buckets, TODAY).current_streak_days, 3);
    }

    #[test]
    fn stats_streak_tolerates_not_dictating_yet_today() {
        // Sin dictar hoy, la racha cuenta desde ayer en vez de romperse.
        let buckets = vec![(999, 1, 10), (998, 1, 10)];
        assert_eq!(compute_usage_stats(&buckets, TODAY).current_streak_days, 2);
    }

    #[test]
    fn stats_words_by_day_covers_seven_days_ending_today() {
        let buckets = vec![(1000, 1, 7), (997, 1, 4), (900, 1, 999)];
        let stats = compute_usage_stats(&buckets, TODAY);

        assert_eq!(stats.words_by_day.len(), 7);
        // El índice 6 es hoy; el 3 es hace tres días.
        assert_eq!(stats.words_by_day[6], 7);
        assert_eq!(stats.words_by_day[3], 4);
        // Lo viejo cuenta en el total pero no en la ventana de la semana.
        assert_eq!(stats.words_by_day.iter().sum::<u32>(), 11);
        assert_eq!(stats.total_words, 1010);
    }

    #[test]
    fn stats_thirty_day_window_excludes_older_buckets() {
        let buckets = vec![(1000, 1, 100), (980, 1, 50), (960, 1, 999)];
        let stats = compute_usage_stats(&buckets, TODAY);

        assert_eq!(stats.words_last_30_days, 150);
        assert_eq!(stats.active_days_last_30, 2);
    }

    #[test]
    fn stats_empty_history_is_all_zeros() {
        let stats = compute_usage_stats(&[], TODAY);
        assert_eq!(stats.total_transcriptions, 0);
        assert_eq!(stats.current_streak_days, 0);
        assert_eq!(stats.words_by_day, vec![0; 7]);
    }
}

#[derive(serde::Serialize, Clone, specta::Type)]
pub struct UsageStats {
    pub total_transcriptions: u32,
    pub total_words: u32,
    pub words_last_30_days: u32,
    pub active_days_last_30: u32,
    pub current_streak_days: u32,
    /// Minutos ahorrados vs teclear a 40 palabras/minuto (supuesto explícito
    /// mostrado en la UI), descontando ~200 wpm de dictado efectivo.
    pub minutes_saved: u32,
    /// Palabras dictadas por día en los últimos 7 días, la más antigua primero
    /// (el índice 6 es hoy). Mismo balde de día UTC que usa la racha: un
    /// dictado nocturno puede caer en el día siguiente, y se prefiere esa
    /// imprecisión conocida a tener dos nociones de "día" en la misma tarjeta.
    pub words_by_day: Vec<u32>,
}

impl HistoryManager {
    /// Estadísticas de uso acumuladas.
    ///
    /// Salen de `usage_daily`, NO del historial. El historial se recorta en cada
    /// dictado (5 entradas por omisión), así que calcular sobre él hacía que
    /// "has dictado N veces" contara solo las últimas cinco — y lo mismo la
    /// racha, los días activos y la gráfica de la semana. Los agregados no se
    /// podan nunca.
    pub fn get_usage_stats(&self) -> Result<UsageStats> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT day, transcriptions, words FROM usage_daily")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;

        let buckets: Vec<(i64, i64, i64)> = rows.flatten().collect();
        Ok(compute_usage_stats(
            &buckets,
            local_day(Utc::now().timestamp()),
        ))
    }
}

/// Cálculo puro sobre los baldes diarios, aparte para poder probarlo: el fallo
/// que lo motivó (estadísticas que solo contaban las últimas cinco entradas)
/// era justo de esta lógica, y no había forma de cubrirlo con un test.
///
/// `today` llega ya resuelto como número de día (ver `local_day`) en vez de como
/// marca de tiempo: así la función razona solo en días —igual que los baldes que
/// recibe— y sigue sin depender del reloj ni de la zona horaria, que es lo que
/// la hace comprobable.
fn compute_usage_stats(buckets: &[(i64, i64, i64)], today: i64) -> UsageStats {
    {
        let cutoff_30 = today - 30;

        let mut total_transcriptions = 0u32;
        let mut total_words = 0u32;
        let mut words_30 = 0u32;
        let mut active_days: std::collections::HashSet<i64> = std::collections::HashSet::new();
        let mut words_per_day: std::collections::HashMap<i64, u32> =
            std::collections::HashMap::new();

        for &(bucket, count, words) in buckets {
            let count = count.max(0) as u32;
            let words = words.max(0) as u32;
            total_transcriptions += count;
            total_words += words;
            if bucket >= cutoff_30 {
                words_30 += words;
            }
            if count > 0 {
                active_days.insert(bucket);
            }
            *words_per_day.entry(bucket).or_insert(0) += words;
        }

        let mut streak = 0u32;
        let mut cursor = today;
        // La racha admite que hoy aún no dictes (cuenta desde ayer también).
        if !active_days.contains(&cursor) {
            cursor -= 1;
        }
        while active_days.contains(&cursor) {
            streak += 1;
            cursor -= 1;
        }

        let active_days_30 = active_days.iter().filter(|d| **d >= cutoff_30).count() as u32;
        // Tecleo promedio real: 52 palabras/min (Dhakal, Feit, Kristensson y
        // Oulasvirta, "Observations on Typing from 136 Million Keystrokes",
        // CHI 2018). Habla: ~200 ppm. Ahorro por palabra: 1/52 - 1/200 ≈
        // 0,0142 min. El 40 ppm anterior no tenia fuente e INFLABA el ahorro
        // (~40%): la cifra que presume la app es justo la que mas barato sale
        // poder defender con una cita.
        let minutes_saved = ((total_words as f64) * (1.0 / 52.0 - 1.0 / 200.0)).round() as u32;

        let words_by_day = ((today - 6)..=today)
            .map(|d| words_per_day.get(&d).copied().unwrap_or(0))
            .collect();

        UsageStats {
            total_transcriptions,
            total_words,
            words_last_30_days: words_30,
            active_days_last_30: active_days_30,
            current_streak_days: streak,
            minutes_saved,
            words_by_day,
        }
    }
}
