# docs/plan — Pipeline Raíz aplicado a Escriba

Generado con la metodología `/plan` del framework Raíz (github.com/AlejandroAP9/raiz),
adaptada a un árbol fuera del Sistema de Raíces (app desktop Tauri local-first).

| Doc | Estado | Nota |
|---|---|---|
| 00-business-model.md | OMITIDO deliberado | El "modelo de negocio" es el concurso + la comunidad; está en el PRD §contexto y propuesta de valor |
| [01-prd.md](01-prd.md) | ✅ APROBADO | Qué, para quién, alcance, fuera de alcance, criterios de éxito |
| [02-tech-spec.md](02-tech-spec.md) | ✅ | Arquitectura heredada + componentes nuevos (sin tablas/RLS: managers/providers) |
| 03-ux-research.md | PENDIENTE con Flor 🤝 | Avatares (builder + no técnico), jobs-to-be-done; insumos ya en 04 |
| [04-user-stories.md](04-user-stories.md) | ✅ | MoSCoW con criterios de aceptación verificables |
| 05-ux-design.md | PENDIENTE con Flor 🤝 | Wireframes de onboarding nuevo, sección phraser, overlay |
| 06-ui-design.md | PENDIENTE con Flor 🤝 | Paleta/design system (theming = 8 CSS vars en `src/styles/theme.css`) |
| [07-security-plan.md](07-security-plan.md) | ✅ | Superficies reales de desktop: descargas, sidecar, clipboard, updater, privacidad |
| [08-blueprint.md](08-blueprint.md) | ✅ APROBADO (2 🤝 abiertas) | **El Blueprint maestro**: 10 fases con validación, mapa fase→skill/PRP |

PRPs en `.claude/PRPs/`: template adaptado (`prp-base.md`) + `PRP-001-motor-local.md`
(APROBADO, con premortem y Plan B). PRP-002 (poderes), PRP-003 (capacidad A) y
PRP-004 (D o B 🤝) se redactan al entrar a sus fases con el skill `prp`.

Skills injertadas en `.claude/skills/`: `plan`, `prp`, `bucle-agentico`.
Blindajes: usar `raiz blindar` (CLI ya instalada). Candidatos pendientes de capturar:
fix build.rs CLT/FoundationModels, conflicto de símbolos ggml, updater apuntando a upstream.
