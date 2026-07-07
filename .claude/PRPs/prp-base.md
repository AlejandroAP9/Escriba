# Sistema PRP (Product Requirements Proposal) — Edición Escriba

> **Los Blueprints del Bosque**, adaptados a un árbol de otra especie: app de
> escritorio Tauri local-first. Contrato humano-IA antes de escribir código.
> Fuente: framework Raíz de Alejandro (github.com/AlejandroAP9/raiz).

---

## Qué es un PRP

El blueprint de una pieza del software. Define QUÉ construir antes de escribir
una sola línea de código.

| Sección | Propósito | Responsable |
|---------|-----------|-------------|
| **Objetivo** | Qué se construye (estado final) | Humano define |
| **Por Qué** | Valor de negocio | Humano define |
| **Qué** | Comportamiento + criterios de éxito | Humano + IA |
| **Contexto** | Docs, referencias, código existente | IA investiga |
| **Premortem** | Cómo se rompe y cómo el diseño lo mata | IA + blindajes |
| **Blueprint** | Fases de implementación (sin subtareas) | IA genera |
| **Aprendizajes** | Self-Annealing: errores y fixes | IA actualiza |

## Flujo

```
1. Humano: "Necesito [feature]"
2. IA: Investiga contexto y viabilidad (skill `prp`)
3. IA: Genera PRP-XXX-nombre.md con este template
4. Humano (dupla Alejandro + Flor): revisa y aprueba
5. IA: Ejecuta Blueprint fase por fase (skill `bucle-agentico`)
6. IA: Documenta aprendizajes (Self-Annealing) y captura blindajes (`raiz blindar`)
```

Nomenclatura: `PRP-[NUMERO]-[descripcion-kebab].md`. Estados: `PENDIENTE` → `APROBADO` → `EN PROGRESO` → `COMPLETADO`.

---

# TEMPLATE PRP

```markdown
# PRP-XXX: [Título]

> **Estado**: PENDIENTE
> **Fecha**: YYYY-MM-DD
> **Proyecto**: Escriba

## Objetivo

[Estado final deseado en 1-2 oraciones]

## Por Qué

| Problema | Solución |
|----------|----------|
| [Dolor del usuario] | [Cómo lo resuelve esta feature] |

**Valor para el concurso/comunidad**: [impacto medible o demoable]

## Qué

### Criterios de Éxito
- [ ] [Criterio medible 1]
- [ ] [Criterio medible 2]

### Comportamiento Esperado
[Happy path]

## Contexto

### Referencias
- `src-tauri/src/[módulo].rs` — patrón a seguir
- [URL de docs/crate]

### Arquitectura Propuesta
[Backend: manager/comando/settings tocados. Frontend: componente/store/i18n.
Patrón de referencia del repo: managers residentes con Arc<Mutex<Option<T>>> +
watcher de unload; comandos tauri-specta; settings con merge idempotente.]

### Modelo de Datos (si aplica)
[Migración rusqlite (history.db) o campos nuevos en AppSettings con default +
merge para usuarios existentes. El schema debe satisfacer el Premortem.]

## Premortem (matar el proyecto en papel)

> ANTES de congelar el diseño. Entradas: `raiz blindajes <tema>` + superficie
> real de una app desktop local: procesos hijos, descargas, clipboard, permisos.

| Amenaza (cómo se rompe) | Cómo la mata el diseño | Cómo se verifica |
|---|---|---|
| [ej: descarga corrupta/MITM] | [SHA256 pinneado + HTTPS] | [alterar 1 byte → rechazo] |
| [ej: proceso hijo huérfano] | [kill en RunEvent::Exit + PID file] | [kill -9 a la app → ps sin huérfanos] |
| [ej: server local expuesto a la LAN] | [bind 127.0.0.1 estricto] | [curl desde otra máquina → sin conexión] |

## Blueprint (el ciclo de cultivo)

> Solo FASES. Las subtareas se generan al entrar a cada fase (bucle agéntico).

### Fase 1: [Nombre]
**Objetivo**: [qué se logra]
**Validación**: [cómo se verifica]

### Fase N: Validación Final
- [ ] `cargo build` + `tsc` pasan
- [ ] `bun run tauri dev` y ejercitar el flujo real (no solo compilar)
- [ ] Prueba reina si toca IA local: funciona con wifi apagado
- [ ] Criterios de éxito cumplidos
- [ ] Premortem re-verificado con evidencia
- [ ] Blindajes capturados (`raiz blindar`)

## Aprendizajes (Self-Annealing)

### [YYYY-MM-DD]: [Título]
- **Error**: [qué falló]
- **Fix**: [cómo se arregló]
- **Aplicar en**: [dónde más]

## Gotchas

- [ ] [ej: "bindings.ts es autogenerado por tauri-specta, NO editar a mano"]
- [ ] [ej: "strings en JSX prohibidos por ESLint: todo pasa por i18next"]

## Anti-Patrones

- NO agregar crates que linkeen ggml (conflicto de símbolos con transcribe-cpp)
- NO llamadas de red en el camino feliz (principio: 100% local, cero API keys)
- NO editar `src/bindings.ts` a mano (se regenera)
- NO strings hardcodeados en JSX (i18next + 21 locales)
- NO settings nuevos sin default + merge para instalaciones existentes
- NO unwrap() en producción

*PRP pendiente aprobación. No se ha modificado código.*
```

---

## Stack de ESTE árbol (no es el Sistema de Raíces estándar)

| Capa | Tecnología |
|------|------------|
| Shell | Tauri 2 (Rust backend + webview) |
| Frontend | React 18 + TypeScript strict + Tailwind CSS 4 + Zustand |
| i18n | i18next, 21 idiomas (es completo) |
| STT | transcribe-cpp (Whisper/GGUF, Metal/Vulkan) + transcribe-rs (ONNX) |
| LLM local | sidecar llama-server (OpenAI-compat en 127.0.0.1) + cascada Ollama/Apple Intelligence |
| Persistencia | tauri-plugin-store (settings) + SQLite rusqlite (historial) |
| Testing | cargo test + Playwright |
| Distribución | GitHub Releases + tauri-plugin-updater (keypair minisign propia) |

**Principio inviolable:** todo funciona gratis y 100% local por defecto. BYOK es
opción avanzada, jamás requisito ni fallback automático.
