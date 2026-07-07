# Blueprint Maestro — Escriba ✒️

> **Estado**: APROBADO con 2 decisiones de dupla abiertas 🤝 (capacidad D vs B; reparto de frentes)
> El documento que `bucle-agentico` ejecuta fase por fase. Cada fase nombra su
> PRP o skill. Solo FASES; las subtareas se generan al entrar a cada una.
> Deadline: entrega 30-jul (buffer 1 día sobre el 31-jul 20:00 Chile).

## Reglas del cultivo

- Principio inviolable: gratis y 100% local por defecto; cero API keys en camino feliz.
- Cada fase termina con su validación ejecutada de verdad (no "compila").
- Hito publicable cada viernes en la comunidad (feedback de jueces = ventaja).
- Errores → sección Aprendizajes del PRP correspondiente + `raiz blindar`.
- Sin merges de upstream (`upstream-base` = dad37ba congelado).

---

### Fase 0: Identidad y fundación pública — ✅ PARCIALMENTE HECHA
**Objetivo:** dupla declarada, nombre, rebrand técnico base, upstream congelado.
**Estado:** Escriba ✒️ confirmado por Flor · tag `upstream-base` · R1 aplicado (productName, identifier com.escriba.app, Cargo/package/título/logs/CLI/headers, updater desconectado de Handy) · avance 1 redactado.
**Pendiente:** registrar escriba.la · publicar avance 1 · primer commit del fork · repo GitHub propio (escriba-app/escriba) · 🤝 reparto de frentes.
**Validación:** `cargo build` OK (✅ binario `escriba` verificado) · avance 1 publicado con @Flor.

### Fase 1: Distribución mínima (semana 7-11 jul)
**Objetivo:** cualquiera puede descargar e instalar Escriba hoy, aunque sin features nuevas.
**Construye:** keypair minisign propia (`tauri signer generate` + secrets CI) · workflows CI para macOS ad-hoc + Windows NSIS sin firma · icono provisional (pluma) en app + tray.
**Archivos:** `.github/workflows/build.yml`, `tauri.conf.json:77` (pubkey), `src-tauri/icons/*`, `resources/tray_*.png`.
**Validación:** instalador de CI instala en máquina virgen por OS; la app abre como "Escriba".
**Hito viernes:** avance 2 con link de descarga.

### Fase 2: Spike del motor (paralela a F1, GATE viernes 11-jul)
**Objetivo:** decisión go/no-go del sidecar con evidencia.
**Construye:** descarga manual de llama-server + Qwen3-4B → spawn desde test Rust → health → chat completion vía `llm_client` apuntando a localhost.
**Validación:** GO = completion correcta en macOS y Windows. NO-GO = activar Plan B (Ollama guiado: detección + botón instalar + `POST /api/pull` con progreso) y ajustar PRP-001.
**PRP:** PRP-001-motor-local.md (sección Plan B incluida).

### Fase 3: Motor local completo (semana 14-18 jul)
**Objetivo:** corrección con IA sin API key, de fábrica.
**Construye:** `LocalLlmManager` (spawn/health/kill/idle-unload/PID) · descarga runtime pinneado SHA256 · `ModelKind{Stt|Llm}` + filtro picker + `get_available_llm_models` · provider `local_llm` + `resolve_post_process_route` (cascada, BYOK jamás automático) · UI "Descargar y activar (~2.6GB)".
**Validación:** dictado corregido **con wifi apagado** · unload a 2 min · `kill -9` sin huérfanos · Premortem PRP-001 verificado.
**PRP:** PRP-001.

### Fase 4: Poderes del phraser (semana 14-18, frente paralelo)
**Objetivo:** los 6 poderes con calidad validada en español.
**Construye:** presets seed + merge (Dictado natural, Prompt Maestro con plantillas por destino, WhatsApp, Email, Lista) · `PostProcessMode{None,Prompt,Translate,Edit}` + binding `transcribe_translate` · tonos por app (`app_context_rules` + frontmost) · diccionario→prompt · U1 des-enterrar post-proceso · batería de casos es-CL/es-ES.
**Validación:** batería completa contra Qwen3-4B local (no contra modelos grandes) · U1: sección visible, toggle interno.
**PRP:** PRP-002-poderes-phraser.md.
**Hito viernes 18:** avance 3, demo Prompt Maestro en Cursor sin API key.

### Fase 5: Capacidad A "Háblale a cualquier texto" (semana 14-18/21)
**Objetivo:** editar selección por voz en cualquier app + preguntar en overlay.
**Construye:** binding `voice_edit` · captura de selección (Cmd/Ctrl+C sintético + save/restore clipboard con guard) · modo editar (paste) y preguntar (overlay `streamTextEvent`) · fallback sin selección.
**Validación:** Notas + WhatsApp Web + VS Code · clipboard intacto tras error inducido · Premortem superficie 3 del security-plan.
**PRP:** PRP-003-hablale-a-cualquier-texto.md.

### Fase 6: Capacidad mayor 🤝 D o B (semana 21-25)
**Objetivo (D):** archivo → transcripción con timestamps → SRT/VTT/TXT/JSON + resumen IA. **Objetivo (B):** QR → subtítulos traducidos en vivo por teléfono.
**Validación (D):** mp3 + .m4a WhatsApp + mp4 → SRT válido en CapCut/YouTube; resumen con wifi apagado. **(B):** 2 teléfonos, 2 idiomas, <5s/segmento, hotspot.
**PRP:** PRP-004 (redactar tras la decisión de dupla).
**Corte interno si aprieta (D):** editor ligero → resumen → el core archivo→SRT se queda.

### Fase 7: Onboarding es-first + empaque UX (semana 21-25, frente paralelo)
**Objetivo:** experiencia hispana de punta a punta.
**Construye:** `rankModelsForLocale` (top picks por idioma, Parakeet con badge "solo inglés") · catálogo localizado (`onboarding.models.<id>.*` + fix buscador :177) · onboarding con paso de phraser · stats + búsqueda historial (recortables).
**Validación:** UI en es → recomendado multilingüe; buscador encuentra por texto traducido.

### Fase 8: Landing + release de prueba (semana 21-25)
**Objetivo:** escriba.la viva + updater probado ANTES de la semana final.
**Construye:** landing con `raiz init` (skills `raiz-landing`, `brand-palette`, `add-seo`, `design-review`, `pre-launch`; design system a elegir con Flor 🤝) · comparativa vs Typeless/Wispr · GIFs instalación honesta (Gatekeeper "Abrir de todos modos" / SmartScreen "Ejecutar de todas formas") · release de prueba tag+latest.json+update N→N+1.
**Validación:** Lighthouse >90 · responsive móvil · update end-to-end OK.
**Hito viernes 25:** avance 4, beta pública + landing (pedir feedback).

### Fase 9: R2 barrido cosmético (28-jul, tras feature freeze)
**Objetivo:** cero rastros visuales de Handy.
**Construye:** "Handy"→"Escriba" en 21 translation.json (script + revisión manual es/en) · SVGs logo pluma (`HandyTextLogo.tsx`, `HandyHand.tsx`) · purga `#FAA2CA`/`#F9C5E8` → CSS vars/currentColor · sincronizar vars del overlay · paleta definitiva en `theme.css` 🤝.
**Validación:** grep "Handy" en src/ y locales → solo créditos deliberados a upstream.

### Fase 10: QA final + video + ENTREGA (28-30 jul)
**Objetivo:** entregar el 30-jul.
**Construye:** QA matrix (macOS ARM 8/16GB · Win 10/11 con/sin Vulkan · instalación virgen) · verificación final del security-plan (red, huérfanos, logs) · video 2 min (guion: problema → Prompt Maestro/Capacidad A en vivo → clímax D o B → wifi apagado → "gratis, open source, tu voz nunca sale" → Escriba ✒️; subtítulos siempre) · post de ENTREGA FINAL con formato oficial.
**Validación:** checklist completa del PRD "Criterios de éxito" · regresión de flujos originales.

---

## Corte mínimo viable (si todo se atrasa)

Instaladores con rebrand (F0-F1) · motor demostrable aunque sea Plan B (F2-F3) · Prompt Maestro + Dictado natural (F4 parcial) · Capacidad A (F5) · landing + video (F8, F10). Orden de sacrificio: búsqueda → stats → tonos por app → traducción → F6 completa.

## Mapa fase → skill/herramienta (regla Raíz: lo que es skill no se reinventa)

| Fase | Skill/herramienta |
|---|---|
| F4/F5/F6 diseño previo | `prp` (este repo, adaptado) |
| Construcción por fase | `bucle-agentico` |
| F8 landing | `raiz init` + `raiz-landing` + `brand-palette` + `add-seo` |
| Revisiones pre-entrega | `security-audit` (adaptado desktop) + `design-review` + `adversarial-review` |
| Assets (pluma, iconos, video) | `image-generation` + `video-visuals` |
| Lecciones | `raiz blindar` (ya candidatos: fix build.rs CLT, conflicto ggml, updater apuntando a upstream) |
