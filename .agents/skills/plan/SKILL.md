---
name: plan
description: "Pipeline de planificación de producto completo ANTES de escribir código. Produce un set de documentos (modelo de negocio, PRD, arquitectura, UX/UI, seguridad) que culminan en un Blueprint maestro ejecutable. Activar cuando el usuario dice: planifica el producto, quiero construir un SaaS, /plan, arranquemos el proyecto, plan completo, blueprint, antes de construir, plan maestro."
allowed-tools: Read, Write, Grep, Glob, Bash
---

# Plan: planificación de producto antes de construir

Planifica lo más agudo posible y construye desde ahí. Construir sobre la marcha
genera el castillo de naipes (cada cambio rompe tres cosas). Este pipeline produce
un set de documentos en `docs/plan/` que terminan en un **Blueprint maestro** que
`bucle-agentico` ejecuta fase por fase.

> No es burocracia por burocracia: cada documento existe para que la construcción
> sea determinista. Strip Back aplica DENTRO de cada doc (lo justo, no relleno),
> no se salta documentos.

## Modos

Primero pregunta qué se construye y ajusta la profundidad:

- **SaaS completo** → pipeline completo (todos los docs).
- **MVP para validar** → BMC + PRD + tech-spec + blueprint (salta UX research extenso).
- **Herramienta interna** → PRD + tech-spec + blueprint.
- **Landing** → usa el skill `website-3d`, no este pipeline.
- **Feature con IA** → usa `prp` (este pipeline es para producto nuevo, no features sueltas).

## Pipeline (escribe cada doc en `docs/plan/`)

Hazlo en orden; cada paso se apoya en el anterior. Entrevista al usuario donde
falte información, no inventes (si no sabes un dato, pregúntalo).

1. **00-business-model.md** — Business Model Canvas: problema, segmento/avatar,
   propuesta de valor, canales, modelo de ingresos, costos, métrica North Star.
   (Reusa la entrevista de `new-app` si conviene.)
2. **01-prd.md** — Product Requirements: qué se construye y para quién, alcance,
   **fuera de alcance** (explícito), criterios de éxito, supuestos y riesgos.
3. **02-tech-spec.md** — Arquitectura sobre el Sistema de Raíces: modelo de datos
   (tablas + relaciones), **plan de RLS por tabla**, rutas y API, integraciones,
   variables de entorno, estructura Feature-First.
4. **03-ux-research.md** — usuario objetivo, jobs-to-be-done, flujos críticos,
   competidores y qué hacen mejor/peor.
5. **04-user-stories.md** — historias priorizadas (MoSCoW: must/should/could/won't)
   con criterios de aceptación verificables.
6. **05-ux-design.md** — arquitectura de información, navegación, wireframes en
   texto (layout por pantalla), estados (loading/empty/error).
7. **06-ui-design.md** — design-system elegido de `.Codex/design-systems/`,
   tokens (color, tipografía, espaciado), componentes clave. Evita el AI-slop.
8. **07-security-plan.md** — plan de seguridad **upfront** usando la checklist del
   skill `security-audit`: RLS por tabla, auth de rutas, secrets, rate limiting,
   validación Zod, webhooks. Decidir la seguridad antes, no parchar después.
9. **08-blueprint.md** — el **Blueprint maestro**: fases de construcción ordenadas
   con dependencias explícitas, cada fase con su objetivo, archivos, criterios de
   validación. Es lo que `bucle-agentico` ejecuta. **Cada fase nombra el skill que la
   construye** (ver mapa abajo): así no se reinventa lo que ya es una skill.

### Mapa de capacidades → skills (úsalo al armar las fases del Blueprint)

Si el producto necesita esto, la fase correspondiente usa este skill, no código a mano:

| Capacidad detectada en el PRD/tech-spec                | Skill                           |
| ------------------------------------------------------ | ------------------------------- |
| Login / registro / cuentas                             | `add-login`                     |
| Cobro / suscripción / checkout                         | `add-payments`                  |
| Emails transaccionales                                 | `add-emails`                    |
| PWA / push / instalable                                | `add-mobile`                    |
| **Subida o entrega de archivos** (avatar, docs, fotos) | `add-storage`                   |
| **Trabajo programado** (digests, limpieza, métricas)   | `add-cron`                      |
| **Avisos al usuario** (campana, badge no leídos)       | `add-notifications`             |
| Chat / RAG / visión / IA                               | `ai`                            |
| Landing de marketing                                   | `raiz-landing` (o `website-3d`) |
| Paleta desde la marca del cliente                      | `brand-palette`                 |

Lo que no calza en una skill se construye a mano en su fase. Lo que sí calza, NO se
reinventa.

## Gate de validación (LA única parada del Flujo 1)

Cuando el Blueprint esté listo, **párate y revísalo con el usuario**. Resume las
decisiones clave (stack, modelo de datos, fases) y pide aprobación explícita. El
código NO empieza hasta que el usuario diga "go". Si algo no convence, se ajusta el
doc correspondiente y se regenera el Blueprint. (Equivale al "crisol" de validación.)

Este es el **único punto** del Flujo 1 donde te detienes a pedir permiso. Todo lo
anterior (entrevista, docs) y todo lo posterior (construcción, revisores) avanza de
corrido sin que el usuario tenga que invocar comandos.

## Después del plan (handoff automático, NO esperes comandos)

Apenas el usuario aprueba el Blueprint ("go"), **continúa automáticamente** sin pedir
que invoque nada:

1. **`bucle-agentico`** → construye fase por fase desde el Blueprint. Cada fase nombra
   su skill (`add-login`, `add-payments`, etc.) y se ejecuta sola en orden.
2. Al terminar la construcción, corre **los 4 revisores** de corrido:
   `security-audit`, `design-review`, `architecture-review`, `performance-review`
   (y `adversarial-review` sobre código sensible: pagos/auth/webhooks). Arregla lo
   que salga.
3. Propón el **deploy** y, si el usuario confirma en lenguaje natural, despliega.

El usuario solo interviene contestando o aprobando en lenguaje natural; nunca tiene
que escribir el nombre de un skill para avanzar.

Captura con `raiz blindar` cualquier decisión de arquitectura que sirva a futuros
proyectos.
