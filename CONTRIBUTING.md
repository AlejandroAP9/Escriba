# Contribuir a Escriba

¡Gracias por tu interés en **Escriba**! Esta guía te ayuda a empezar a contribuir
a esta app de dictado por voz open source, **100% local y gratis**.

> Escriba es un _rework_ de [Handy](https://github.com/cjpais/handy) (© CJ Pais,
> MIT), construido por la dupla **Alejandro & Flor** para los Juegos Imperiales.
> Buena parte de la arquitectura —y de esta misma guía— parte de las sólidas
> bases de Handy. Gracias, CJ 🙏.

## 📖 Filosofía

Escriba cree que las herramientas de accesibilidad pertenecen a todos.
Priorizamos:

- **Local por defecto:** tu voz siempre se transcribe en tu máquina y nunca sale
  de ella. El texto solo sale si tú configuras y eliges un proveedor remoto
  (viene apagado).
- **Gratis e ilimitado:** sin cuentas, sin claves de API, sin cupos.
- **Open source:** código legible y extensible.
- **Simplicidad:** código claro y mantenible por encima de soluciones ingeniosas.

## 🚀 Empezar

### Requisitos

- [Rust](https://rustup.rs/) (última estable)
- [Bun](https://bun.sh/)
- Herramientas de build según tu plataforma (ver [BUILD.md](BUILD.md))

### Configurar el entorno

1. Haz **fork** del repositorio en GitHub.
2. **Clona** tu fork:

   ```bash
   git clone git@github.com:TU_USUARIO/Escriba.git
   cd Escriba
   ```

3. (Opcional) Agrega el remoto de la base original:

   ```bash
   git remote add upstream git@github.com:cjpais/handy.git
   ```

4. **Instala dependencias:**

   ```bash
   bun install
   ```

5. **Descarga el modelo de detección de voz (VAD):**

   ```bash
   mkdir -p src-tauri/resources/models
   curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx
   ```

6. **Corre en desarrollo:**
   ```bash
   bun run tauri dev
   # En macOS, si ves errores de cmake:
   CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev
   ```

Para setup detallado por plataforma, ver [BUILD.md](BUILD.md).

### La estructura del código

**Backend (Rust — `src-tauri/src/`):**

- `lib.rs` — entrada de la app y setup de Tauri
- `managers/` — lógica principal (audio, modelos, transcripción, intérprete, LLM
  local, servidor MCP)
- `audio_toolkit/` — audio de bajo nivel (grabación, VAD, supresión de ruido)
- `studio/` — Estudio: archivo → SRT / VTT / TXT / JSON + resumen
- `commands/` — handlers de comandos Tauri
- `shortcut/` — atajos globales de teclado
- `settings.rs` — configuración de la app

**Frontend (React/TypeScript — `src/`):**

- `App.tsx` — componente principal
- `components/` — UI
- `hooks/`, `stores/` — estado y hooks reutilizables
- `bindings.ts` — tipos autogenerados (tauri-specta)
- `i18n/` — 21 idiomas

Más detalle en la sección de arquitectura del [README.md](README.md) y en
[AGENTS.md](AGENTS.md).

## 🐛 Reportar bugs

Antes de abrir un issue:

1. Busca en los
   [issues existentes](https://github.com/AlejandroAP9/Escriba/issues).
2. Prueba la última versión por si ya está resuelto.
3. Activa el modo debug (`Cmd/Ctrl+Shift+D`) para reunir diagnóstico.

Incluye: versión de la app, sistema operativo, CPU/GPU, pasos para reproducir,
comportamiento esperado vs. real, y capturas o logs si aplica.

## 💡 Proponer mejoras

Abre un issue o una discusión en el
[repo de Escriba](https://github.com/AlejandroAP9/Escriba) describiendo el
problema que resuelve tu idea, tu propuesta, y cómo encaja con la filosofía
(local, gratis, simple).

## 🔧 Contribuir código

### Flujo

1. Rama nueva: `git checkout -b feat/tu-feature` (o `fix/...`).
2. Cambios limpios: sigue el estilo existente, comenta la lógica no obvia y
   mantén commits atómicos.
3. **Corre los gates en local antes de commitear** (durante el hackathon el CI
   está en pausa; se valida en local):

   ```bash
   bunx tsc --noEmit          # tipos del frontend
   bun run lint               # ESLint
   bun run check:translations # las 21 locales completas
   bun run format             # Prettier + cargo fmt
   cd src-tauri && cargo test # tests de Rust
   ```

4. Commit con [conventional commits](https://www.conventionalcommits.org/):
   `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
5. Push a tu fork y abre un Pull Request describiendo qué cambia, cómo lo
   probaste y capturas/video si aplica.

### Divulgación de IA

**Los PRs asistidos con IA son bienvenidos.** Solo sé transparente: indica en la
descripción si usaste IA, qué herramienta (Claude Code, Copilot, ChatGPT…) y qué
tanto la usaste.

### Estilo de código

- **Rust:** `cargo fmt` + `cargo clippy`, nombres descriptivos, doc comments en
  APIs públicas y manejo explícito de errores (evita `unwrap` en producción).
- **TypeScript/React:** TS estricto (sin `any`), componentes funcionales,
  Tailwind, y **todo texto de UI vía i18next** (ESLint lo exige; nada de strings
  literales en JSX).

## 🌍 Traducciones

Escriba está en **21 idiomas** (español primero). Las claves viven en
`src/i18n/locales/<idioma>/translation.json`. Corre `bun run check:translations`
para verificar que todas las locales tengan exactamente las mismas claves.

## 📜 Licencia

Al contribuir, aceptas que tu aporte se licencie bajo **MIT** (ver
[LICENSE](LICENSE)). El código original de Handy es © CJ Pais; las
modificaciones de Escriba mantienen la misma licencia.

---

Hecho con ✒️ por **Alejandro & Flor** para los Juegos Imperiales. Gracias por
ayudar a que el dictado por voz sea local, privado y de todos.
