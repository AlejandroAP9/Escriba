# PRD — Escriba ✒️

> **Estado**: APROBADO (plan validado por Alejandro 7-jul; nombre confirmado por Flor 7-jul)
> Pipeline Raíz `/plan`, modo adaptado (producto sobre fork existente).

## Qué se construye y para quién

**Escriba**: app de escritorio (macOS + Windows) de dictado por voz con IA, 100%
local y gratuita. Fork con rebrand de Handy (MIT). Eslogan: **"Tu Escriba
personal: hablas, él escribe"**. Logo: una pluma.

**Para quién:** la comunidad hispanohablante de IA "Imperio" (+2.500 builders,
creadores de contenido y profesionales) y, tras el hackathon, cualquier
hispanohablante que hoy paga Typeless/Wispr Flow o no dicta por fricción.

**Contexto de negocio:** entrada al Hackathon Imperial (deadline 31-jul 20:00
Chile; entrega objetivo 30-jul). Se evalúa branding, funcionamiento, usabilidad,
video 2 min (elige finalistas) y landing. Premio: 1 Mac Mini por integrante.
La versión ganadora queda como app oficial de dictado de la comunidad.

## Propuesta de valor (la tesis)

**Tres categorías de producto pago en una sola app gratis, ilimitada, open
source, donde la voz nunca sale del computador:**

| Categoría | Quién cobra | Nuestro precio |
|---|---|---|
| Dictado con IA (limpieza, tonos) | Typeless $12-30/mes (free: 8.000 palabras/semana, nube) · Wispr Flow $15/mes | $0, ilimitado, local |
| Edición/consulta por voz sobre texto seleccionado | Typeless (parte de Pro) | $0 |
| Estudio de transcripción de archivos + subtítulos + resumen | WhisperAI $99-750/año | $0 |

**Diferenciador estructural:** el post-procesado con LLM corre LOCAL (sidecar
llama-server + Qwen3-4B descargados por la app). Cero API keys en el camino
feliz. Las demás duplas seguirán la receta BYOK del enunciado.

## Alcance (qué entra)

1. **Motor**: LLM local zero-install con cascada de respaldo (sidecar → Ollama → Apple Intelligence → texto crudo con aviso).
2. **Poderes del phraser**: Prompt Maestro (vibecoding), Dictado natural (muletillas/autocorrecciones/formato), Traducción al dictar, Tonos por app, Diccionario personal conectado, Presets en español.
3. **Capacidad A**: "Háblale a cualquier texto" (editar selección por voz + preguntar sobre texto de solo lectura).
4. **Capacidad mayor 🤝 (D o B)**: Estudio de transcripción (archivos → SRT/VTT/TXT/JSON + resumen) o Intérprete en vivo con QR.
5. **Empaque**: rebrand completo, post-proceso des-enterrado, onboarding es-first, stats + búsqueda de historial (recortables), landing (escriba.la, nace de `raiz init`), video 2 min, CI + updater propio.

## Fuera de alcance (explícito)

- Diarización / speaker labels (modelos pesados; post-hackathon).
- Apps móviles iOS/Android; extensión Chrome; equipos/seats/cuentas.
- Firma/notarización Apple y firma Windows (se documenta la instalación honesta).
- Chat conversacional con transcripciones; aprendizaje de estilo automático.
- Merges de upstream Handy durante el hackathon (tag `upstream-base` congelado).

## Criterios de éxito

- [ ] Instalador descargable macOS + Windows; instalación en máquina virgen sin login.
- [ ] Dictado + corrección IA funcionan **con wifi apagado** (prueba reina).
- [ ] Los 6 poderes pasan su batería de casos en español contra Qwen3-4B local.
- [ ] Capacidad A funciona en 3 apps distintas (Notas, WhatsApp Web, VS Code).
- [ ] Landing viva en escriba.la con conversión en frío (hero + demo + descarga por OS + comparativa + instalación con GIFs).
- [ ] Video ≤2 min con subtítulos, clímax demoable en vivo.
- [ ] Updater end-to-end probado (release N → N+1) antes de la semana final.
- [ ] Flujos originales de Handy sin regresión (dictado, historial, modelos).

## Supuestos y riesgos

- **Supuesto:** Qwen3-4B Q4 corrige/traduce/estructura bien en español (validar en spike; fallback 1.7B en 8GB).
- **Riesgo mayor:** sidecar en Windows (Vulkan/DLLs/antivirus) → gate go/no-go viernes etapa 1; Plan B Ollama guiado (~2 días) listo.
- **Riesgo:** Cmd/Ctrl+C sintético bloqueado en algunas apps → fallback "actúa sobre el último dictado".
- **Riesgo calendario:** dupla remota Chile-España (~6h) → handoffs asíncronos diarios; hitos publicables cada viernes (feedback de jueces).
- Decisiones de dupla pendientes 🤝: capacidad D vs B, reparto de frentes, tono de marca/video.
