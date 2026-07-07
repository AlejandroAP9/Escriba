---
name: prp
description: "Planificar una feature compleja antes de implementarla. Genera un PRP (Product Requirements Proposal) con objetivo, comportamiento, modelo de datos, y fases. Activar SIEMPRE antes de bucle-agentico, o cuando el usuario dice: planea esto, necesito un sistema de X, quiero agregar algo grande."
context: fork
allowed-tools: Read, Write, Edit, Grep, Glob
---

# Skill: Generar PRP (Product Requirements Proposal)

> Generar un PRP para: $ARGUMENTS

---

## Que es un PRP

Un PRP (Product Requirements Proposal) es el **blueprint de una pieza de tu software**. Define QUE construir antes de escribir una sola linea de codigo.

Es el contrato humano-IA. El humano define el objetivo y el por que. La IA investiga contexto, propone la arquitectura, y genera el plan de fases. Juntos validan antes de ejecutar.

**Sin PRP**: vibe coding al aire, codigo espagueti, features que no encajan.
**Con PRP**: arquitectura clara, fases definidas, aprendizajes que persisten.

---

## Proceso

### Paso 1: Leer el template

Lee el template base de PRP:

```
.claude/PRPs/prp-base.md
```

Este archivo contiene la estructura completa, las secciones obligatorias, y ejemplos de cada campo. Usalo como referencia para generar el PRP.

### Paso 2: Entrevistar al usuario

Si `$ARGUMENTS` no tiene suficiente detalle, haz preguntas cortas y directas para clarificar:

1. **Objetivo**: Que quieres construir? (estado final en 1-2 oraciones)
2. **Por que**: Que problema resuelve? Cual es el valor de negocio?
3. **Criterios de exito**: Como sabes que esta terminado? (3-5 checkboxes medibles)
4. **Restricciones**: Hay algo que NO deba hacer o alguna dependencia critica?

NO hagas las 4 preguntas si el usuario ya dio suficiente contexto. Usa el juicio. Si el objetivo es claro, pregunta solo lo que falta.

### Paso 3: Investigar contexto

Antes de escribir el PRP, investiga el codebase:

- **Grep/Glob**: Buscar codigo existente relacionado con la feature
- **Read**: Leer archivos relevantes para entender patrones actuales
- **Supabase**: Si involucra BD, verificar tablas y estructura existente

Esto alimenta las secciones de Contexto, Referencias, y Arquitectura del PRP.

### Paso 3.5: Premortem (matar el proyecto en papel)

> Lo mejor de Forge, con lo que a Forge le falta. El premortem corre **ANTES**
> de congelar el modelo de datos, no despues de construir. Objetivo: que el
> schema **nazca blindado, no parchado**.

Antes de escribir el modelo de datos y el Blueprint, imagina como este sistema
podria fallar o ser vulnerado, y deja la defensa escrita en el PRP.

1. **Trae la memoria del bosque** (esto Forge no lo tiene: re-piensa de cero cada
   vez). Busca blindajes previos del tema antes de imaginar nada:
   ```
   raiz blindajes "<tema: pagos, RLS, webhook, uploads, cron, multi-tenant...>"
   ```
   Cada blindaje relevante es una amenaza **ya confirmada** en otro proyecto:
   entra al premortem como hallazgo con su guard listo.

2. **Pasa el plan por la checklist del skill `security-audit`** como lente de
   amenaza (clases reales de vuln en Next.js+Supabase, no teoria generica del
   modelo). Para CADA area que toque la feature pregunta "¿como lo rompo?", con
   foco en lo que un premortem ingenuo SIEMPRE se salta:
   - **Concurrencia**: ¿que pasa si llega 2 veces / 3 a la vez? Idempotencia,
     locks, limites **atomicos**. Un threat-model de una sola pasada no ve races.
   - **Guard debil**: no basta "valida la URL" o "cifra el secreto". ¿El guard
     resuelve DNS? ¿el WITH CHECK congela el campo critico? ¿el limite es atomico
     o se pasa por concurrencia? ¿falla **abierto** o **cerrado** ante error?
   - **Bypass por service-role**: ¿alguna ruta firma/lee datos con service-role
     confiando en que "la RLS valida"? La RLS NO aplica a service-role.

3. **Escribe cada hallazgo** en la seccion `## Premortem` del PRP con tres
   columnas: amenaza, como la mata el diseño, y como se **verifica** que quedo
   cerrada (comando/query/test, no "confia en mi").

4. **Cada hallazgo se vuelve dos cosas** (asi no es teatro como en Forge):
   - una **restriccion del Modelo de Datos** (secreto cifrado, RLS con WITH CHECK
     que congela `role`/`plan` desde el dia uno, indice unico para la race), y
   - un **Criterio de Exito verificable**.

### Paso 4: Generar el PRP

Crea el archivo PRP siguiendo el template de `prp-base.md`:

**Nombre del archivo**: `.claude/PRPs/prp-{feature-name}.md`

Donde `{feature-name}` es el nombre de la feature en kebab-case.

**Contenido obligatorio**:
- Objetivo (1-2 oraciones)
- Por Que (tabla problema/solucion + valor de negocio)
- Criterios de Exito (checkboxes medibles)
- Comportamiento Esperado (happy path)
- Contexto (referencias, arquitectura propuesta, modelo de datos)
- Premortem (amenazas + como el diseño las mata + como se verifica) — del Paso 3.5
- Blueprint (SOLO fases, sin subtareas)
- Secciones vacias de Aprendizajes, Gotchas, y Anti-Patrones

### Paso 5: Presentar al usuario

Muestra un resumen del PRP generado:
- Objetivo
- Numero de fases
- Decisiones de arquitectura clave
- Pregunta si quiere ajustar algo antes de aprobar

**NO implementes nada todavia.** El PRP debe ser aprobado antes de ejecutar.

---

## Despues del PRP

Una vez aprobado, la implementacion se hace con el skill `/bucle-agentico`, que usa el PRP como guia para ejecutar fase por fase con mapeo de contexto just-in-time.

Los aprendizajes descubiertos durante la implementacion se documentan de vuelta en el PRP (seccion Aprendizajes) para que el conocimiento persista.

---

## Reglas

- SIEMPRE leer `prp-base.md` antes de generar
- NUNCA generar subtareas dentro de las fases (eso lo hace Blueprint)
- NUNCA implementar codigo en este skill (solo generar el documento)
- SIEMPRE investigar el codebase antes de proponer arquitectura
- SIEMPRE correr el Premortem (Paso 3.5) antes de congelar el modelo de datos. El
  schema nace blindado, no parchado
- El premortem NO es teatro: cada amenaza es un Criterio de Exito verificable, y
  se re-verifica sobre el codigo construido (skill `security-audit` + evidencia),
  no basta "funciona". Las clases confirmadas se capturan con `raiz blindar`
- El PRP se crea en estado `PENDIENTE` hasta que el usuario apruebe
