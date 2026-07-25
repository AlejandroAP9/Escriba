---
name: bucle-agentico
description: "Ejecutar features complejas por fases con mapeo de contexto real antes de cada fase. Activar cuando la tarea toca multiples archivos, requiere cambios en BD + codigo + UI coordinados, tiene fases que dependen una de otra, o cuando un PRP fue aprobado y hay que implementarlo."
---

# Modo BLUEPRINT del Bucle Agentico

> "No planifiques lo que no entiendes. Mapea contexto, luego planifica."

El modo BLUEPRINT es para sistemas complejos que requieren construccion por fases con mapeo de contexto just-in-time.

---

## Cuando Usar BLUEPRINT

- La tarea requiere multiples componentes coordinados
- Involucra cambios en DB + codigo + UI
- Tiene fases que dependen una de otra
- Requiere entender contexto antes de implementar
- El sistema final tiene multiples partes integradas

### Ejemplos

```
"Sistema de autenticacion con roles y permisos"
"Feature de notificaciones en tiempo real"
"Dashboard con metricas y graficos"
"Sistema de facturacion con Stripe"
"CRUD completo de productos con imagenes"
"Migracion de arquitectura de componentes"
```

---

## La Innovacion Clave: Mapeo de Contexto Just-In-Time

### El Problema del Enfoque Tradicional

```
Recibir problema
    |
Generar TODAS las tareas y subtareas
    |
Ejecutar linealmente
```

**Problema**: Las subtareas se generan basandose en SUPOSICIONES, no en contexto real.

### El Enfoque BLUEPRINT

```
Recibir problema
    |
Generar solo FASES (sin subtareas)
    |
ENTRAR en Fase 1
    |
MAPEAR contexto real de Fase 1
    |
GENERAR subtareas basadas en contexto REAL
    |
Ejecutar Fase 1
    |
ENTRAR en Fase 2
    |
MAPEAR contexto (incluyendo lo construido en Fase 1)
    |
GENERAR subtareas de Fase 2
    |
... repetir ...
```

**Ventaja**: Cada fase se planifica con informacion REAL del estado actual del sistema.

---

## El Flujo BLUEPRINT: 5 Pasos

### PASO 1: DELIMITAR Y DESCOMPONER EN FASES

```
+-------------------------------------------------------------+
|  PASO 1: DELIMITAR Y DESCOMPONER EN FASES                   |
|                                                              |
|  - Entender el problema FINAL completo                       |
|  - Romper en FASES ordenadas cronologicamente                |
|  - Identificar dependencias entre fases                      |
|  - NO generar subtareas todavia                              |
|  - Usar TodoWrite para registrar las fases                   |
+-------------------------------------------------------------+
```

### PASO 2: ENTRAR EN FASE N - MAPEAR CONTEXTO

ANTES de generar subtareas, explorar:

**Codebase:**

- Que archivos/componentes existen relacionados?
- Que patrones usa el proyecto actualmente?
- Hay codigo que puedo reutilizar?

**Base de Datos (Supabase MCP):**

- Que tablas existen?
- Que estructura tienen?
- Hay RLS policies configuradas?

**Dependencias:**

- Que construi en fases anteriores?
- Que puedo asumir que ya existe?
- Que restricciones tengo?

DESPUES de mapear, generar subtareas especificas y actualizar TodoWrite.

### PASO 3: EJECUTAR SUBTAREAS DE LA FASE

```
WHILE subtareas pendientes en fase actual:

  1. Marcar subtarea como in_progress en TodoWrite

  2. Ejecutar la subtarea

  3. [Dinamico] Usar MCPs si el juicio lo indica:
     - Next.js MCP -> Ver errores en tiempo real
     - Playwright -> Validar visualmente
     - Supabase -> Consultar/modificar DB

  4. Validar resultado
     - Si hay error -> AUTO-BLINDAJE (ver paso 3.5)
     - Si esta bien -> Marcar completed

  5. Siguiente subtarea

Fase completada cuando todas las subtareas done.
```

### PASO 3.5: AUTO-BLINDAJE (cuando hay errores)

El sistema se BLINDA con cada error. Cuando algo falla:

1. **ARREGLA** el codigo
2. **TESTEA** que funcione
3. **DOCUMENTA** el aprendizaje:
   - En el PRP actual (seccion "Aprendizajes")
   - O en el skill relevante (`.Codex/skills/*/SKILL.md`)
4. Continua con la subtarea

**Formato de documentacion:**

```markdown
### [YYYY-MM-DD]: [Titulo corto]

- **Error**: [Que fallo exactamente]
- **Fix**: [Como se arreglo]
- **Aplicar en**: [Donde mas aplica este conocimiento]
```

| Tipo de Error               | Donde Documentar                             |
| --------------------------- | -------------------------------------------- |
| Especifico de esta feature  | PRP actual (seccion Aprendizajes)            |
| Aplica a multiples features | Skill relevante (`.Codex/skills/*/SKILL.md`) |
| Aplica a TODO el proyecto   | `AGENTS.md` (seccion No Hacer)               |

El conocimiento persiste. El mismo error NUNCA ocurre dos veces en este proyecto ni en proyectos futuros.

### PASO 4: TRANSICIONAR A SIGUIENTE FASE

- Confirmar que fase actual esta REALMENTE completa
- NO asumir que todo salio como se planeo
- Volver a PASO 2 con la siguiente fase
- El contexto ahora INCLUYE lo construido

Repetir hasta completar todas las fases.

### PASO 5: VALIDACION FINAL

- Testing end-to-end del sistema completo
- Validacion visual con Playwright si aplica
- Confirmar que el problema ORIGINAL esta resuelto
- Reportar al usuario que se construyo

---

## Uso de MCPs en BLUEPRINT

Los MCPs se usan **durante la ejecucion**, no como pasos del plan.

### Durante Mapeo de Contexto

```
Supabase MCP:
  - list_tables -> Ver que tablas existen
  - execute_sql -> Verificar estructura actual

Codebase (Grep/Glob/Read):
  - Buscar patrones existentes
  - Entender estructura actual
```

### Durante Ejecucion de Subtareas

```
Next.js MCP:
  - get_errors -> Despues de escribir codigo
  - get_logs -> Si algo no funciona como esperado

Playwright MCP:
  - screenshot -> Validar UI despues de cambios visuales
  - click/fill -> Probar flujos completos

Supabase MCP:
  - apply_migration -> Crear/modificar tablas
  - execute_sql -> Verificar que datos se guardan
```

---

## Errores Comunes a Evitar

### Error 1: Generar todas las subtareas al inicio

```
MAL:
Fase 1: Auth base
   -> 10 subtareas detalladas
Fase 2: Roles
   -> 8 subtareas detalladas (basadas en SUPOSICIONES)
Fase 3: Permisos
   -> 12 subtareas detalladas (basadas en SUPOSICIONES)
```

```
BIEN:
Fase 1: Auth base (sin subtareas)
Fase 2: Roles (sin subtareas)
Fase 3: Permisos (sin subtareas)

-> Entrar en Fase 1
-> MAPEAR contexto
-> GENERAR subtareas de Fase 1
-> Ejecutar
-> Entrar en Fase 2
-> MAPEAR contexto (ahora incluye lo que REALMENTE construi)
-> GENERAR subtareas de Fase 2
...
```

### Error 2: MCPs como pasos obligatorios

```
MAL:
1. Tomar screenshot
2. Escribir codigo
3. Tomar screenshot
4. Verificar errores
5. Tomar screenshot
```

```
BIEN:
1. Implementar componente LoginForm
2. Implementar validacion
3. Conectar con auth service

(Durante ejecucion, usar MCPs cuando el JUICIO lo indique)
```

### Error 3: No re-mapear contexto entre fases

```
MAL:
Fase 1 completada -> Pasar directo a ejecutar Fase 2

BIEN:
Fase 1 completada -> MAPEAR contexto de Fase 2 -> Generar subtareas -> Ejecutar
```

---

## Checklist de Calidad

Antes de marcar una fase como completada:

- [ ] Todas las subtareas estan realmente terminadas?
- [ ] Verifique errores con Next.js MCP?
- [ ] La funcionalidad hace lo que se esperaba?
- [ ] Hay algo que deberia ajustar antes de avanzar?

Antes de transicionar a siguiente fase:

- [ ] Mapee el contexto actualizado?
- [ ] Las subtareas de la nueva fase consideran lo que YA existe?
- [ ] Hay dependencias que debo tener en cuenta?

---

## Al terminar la construcción (handoff automático)

Cuando todas las fases estén completas, **no te quedes esperando un comando**:

- **Si vienes del Flujo 1 (proyecto nuevo)**: continúa de corrido con los 4 revisores
  (`security-audit`, `design-review`, `architecture-review`, `performance-review`,
  - `adversarial-review` sobre pagos/auth/webhooks), arregla lo que salga, y luego
    propón el deploy. El usuario solo aprueba en lenguaje natural; no invoca skills.
- **Si es una feature suelta (Flujo 2)**: valida con `playwright-cli` y reporta el
  resultado. No dispares revisores/deploy salvo que el usuario lo pida.

---

## Principios BLUEPRINT

1. **Fases primero, subtareas despues**: Solo generar subtareas cuando entras a la fase
2. **Mapeo obligatorio**: Siempre mapear contexto antes de generar subtareas
3. **MCPs como herramientas**: Usar cuando el juicio lo indique, no como pasos fijos
4. **TodoWrite activo**: Mantener actualizado el progreso para visibilidad
5. **Validacion por fase**: Confirmar que cada fase esta completa antes de avanzar
6. **Contexto acumulativo**: Cada fase hereda el contexto de las anteriores

---

_"La precision viene de mapear la realidad, no de imaginar el futuro."_
_"El sistema que se blinda solo es invencible."_
