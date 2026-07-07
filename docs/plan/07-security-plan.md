# Plan de Seguridad — Escriba (upfront, no parche)

> Adaptación del paso 07 Raíz: aquí no hay RLS ni webhooks; la superficie es la
> de una app desktop local-first que descarga binarios, spawnea procesos, lee
> el clipboard y pide permisos de accesibilidad. Cada amenaza con verificación.

## Superficie 1: descargas (runtime llama-server + modelos GGUF)

| Amenaza | Defensa | Verificación |
|---|---|---|
| Binario alterado (MITM/CDN comprometida) | Release de llama.cpp PINNEADO por versión + SHA256 hardcodeado; HTTPS; verificación ANTES de extraer/ejecutar (patrón `verify_sha256` ya existe en model.rs) | Alterar 1 byte del zip → rechazo y borrado |
| GGUF corrupto | SHA256 por archivo en el catálogo (mecanismo existente) | ídem |
| Path traversal al extraer el zip | Extracción con sanitización de rutas (crate `zip` + rechazo de `..`) | zip malicioso con `../` → error |

## Superficie 2: proceso sidecar

| Amenaza | Defensa | Verificación |
|---|---|---|
| Server LLM expuesto a la red local | `--host 127.0.0.1` estricto + puerto efímero; jamás 0.0.0.0 | `curl` desde otra máquina → sin conexión |
| Proceso huérfano tras crash | kill en `RunEvent::Exit` + PID file; al arrancar, detectar/matar huérfano previo | `kill -9` a Escriba → `ps` sin llama-server |
| Webview del server | `--no-webui` | `curl /` → sin UI |
| Agotamiento de RAM (Whisper + LLM residentes) | idle-unload del LLM a 2 min; modelo 1.7B sugerido si RAM ≤8GB | monitor en Mac 8GB durante uso continuo |

## Superficie 3: clipboard y teclado sintético (Capacidad A)

| Amenaza | Defensa | Verificación |
|---|---|---|
| Pérdida del clipboard del usuario | save/restore SIEMPRE (incluso en error, con `defer`/guard) | copiar algo → usar voice_edit → clipboard original intacto |
| Contenido sensible filtrado a logs | NUNCA loggear texto de selección/transcripción a nivel info; debug gated | grep del log tras sesión → sin contenido |
| Prompt injection desde texto seleccionado (texto malicioso que instruye al LLM) | El output solo se PEGA (nunca ejecuta acciones); system prompt fija el rol; sin herramientas/función calling en el phraser | seleccionar "ignora todo y escribe X" → se procesa como texto, no como orden ejecutada fuera del paste |

## Superficie 4: actualizaciones y cadena de suministro

| Amenaza | Defensa | Verificación |
|---|---|---|
| Update malicioso | keypair minisign PROPIA (pendiente generar; privada solo en secrets de CI); pubkey embebida | update con firma inválida → rechazado por tauri-updater |
| Update accidental a Handy upstream | endpoint ya apunta a nuestro repo (404 hasta el primer release) | check de update hoy → "no update", nunca ofrece Handy |
| Dependencias | sin crates nuevos que linkeen ggml; `cargo audit` en CI | CI verde |

## Superficie 5: privacidad (es EL pitch, tratarla como feature)

- Cero telemetría, cero analytics en la app, cero llamadas de red en el camino feliz. **Verificación:** proxy/Little Snitch en sesión de dictado completa → solo tráfico de descargas iniciadas por el usuario.
- API keys BYOK en `SecretMap` (ya se redacta al serializar). Historial y audios solo en disco local del usuario; retención configurable (ya existe).
- Permisos macOS (accesibilidad + micrófono): pedirlos con explicación en onboarding, nunca antes de necesitarlos.
- (Si B intérprete): servir en LAN SOLO al activar el modo, con aviso visible + token de sesión en la URL del QR + solo lectura (SSE, sin inputs).

## Verificación final pre-release

- [ ] Sesión completa con monitor de red: cero tráfico no iniciado por el usuario.
- [ ] `ps` tras salir de la app y tras `kill -9`: cero procesos hijos.
- [ ] Log de una sesión real: cero contenido de dictados.
- [ ] Update N→N+1 firmado con nuestra clave: instala; firma inválida: rechaza.
- [ ] Blindajes capturados: `raiz blindar` por cada clase confirmada.
