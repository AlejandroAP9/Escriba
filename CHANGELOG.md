# Changelog

Todas las novedades notables de **Escriba**. El formato sigue el estilo de
[Keep a Changelog](https://keepachangelog.com/es/1.1.0/) y el versionado es
[SemVer](https://semver.org/lang/es/).

Escriba es un _rework_ de [Handy](https://github.com/cjpais/handy) (© CJ Pais,
MIT). Esta historia arranca en el fork del 7 de julio de 2026 y recoge lo que
la dupla **Alejandro & Flor** construyó encima para los Juegos Imperiales:
convertir una app de dictado en un motor de IA **100% local y gratis**.

## [1.9.0] — 2026-07-17

La tanda del QA a dos continentes: lo que encontró la dupla probando en
Windows y en la vida real, más la feature que la comunidad definió.

### Añadido

- **Revisar antes de pegar** (opcional, apagado por defecto): el dictado se
  muestra en pantalla antes de escribirse — pégalo, descártalo o dicta una
  corrección con tu atajo, las veces que quieras. Para correos delicados y
  documentos formales. Nace del benchmark del mercado y del "game over" que
  pidió la comunidad: dictar sin que corregir sea un quilombo.

### Corregido

- **La pluma ya no desaparece en la barra de tareas oscura de Windows** (fix
  de la dupla): Windows permite pintar la barra independiente del modo de las
  apps; ahora se lee el ajuste real del registro y un vigía lo sigue si
  cambia. (QA de Flor)
- **Plumín también recibe en Windows y Linux:** la bienvenida vivía en la
  pantalla de permisos exclusiva de macOS; ahora está en la selección de
  modelo, que ven todos. (QA de Flor)
- **El Estudio rescata los MP4 rebeldes** (videos de cursos y plataformas que
  el decodificador no leía): en macOS entra el motor de QuickTime como
  respaldo. En Windows/Linux el límite se mantiene, dicho de frente.

## [1.8.0] — 2026-07-16

La tanda comunidad: dos features nacidas de los comentarios del hackathon,
con crédito a sus autores.

### Añadido

- **Sugerencias de diccionario** (idea de Benjamín Carreño): Escriba detecta
  en tu historial las palabras inusuales que repites — nombres propios,
  marcas, jerga técnica — y las propone con un clic para Palabras
  personalizadas, donde la corrección con IA las respeta siempre. Heurística
  100% local, como todo.
- **Plumín siente la sesión** (idea de Pedro Sánchez): al crear el documento,
  el motor local ya leyó toda la conversación; ahora también detecta su tono
  general, y Plumín entrega el acta con la carita acorde. Reunión tensa →
  empatía; el resto → fiesta. Reacciona al final, invitado — jamás vigila
  mientras dictas.

## [1.7.0] — 2026-07-15

Escriba tiene alma nueva: nace **Plumín**, el aprendiz de escriba.

### Añadido

- **Plumín** 🪶: la mascota de Escriba (pluma aprendiz + su gotita de tinta),
  diseñada por la dupla y bautizada por el jurado infantil de QA. Debuta
  donde aporta y jamás flota sobre tu trabajo: guía el onboarding, acompaña
  los estados vacíos (Historial, Sesiones), y celebra cuando tu documento se
  escribe solo. El anti-Clippy con el corazón de Clippy.
- **La onda de voz de la marca cobra vida:** respira con desfase por barra en
  la barra lateral y el héroe de Inicio. Con "reducir movimiento" activado
  queda estática.

## [1.6.0] — 2026-07-15

"Solo habla", literal: cero teclas.

### Añadido

- **Dictado libre:** micrófono abierto y cero atajos. El detector de voz corta
  cada frase en los silencios y el texto se escribe donde esté tu cursor,
  frase por frase. Se activa a mano (tray o pill), y como escribe TODO lo que
  se hable, su estado es imposible de ignorar: icono de grabación en la barra
  de menú + pill de modo activo con botón Detener. Tonos por app y los
  interceptores de Sesiones/Traductor aplican igual que con el atajo.
- **Menú de bandeja extendido:** Abrir Escriba, Dictar ahora, activar el
  Dictado libre, Sesión rápida (escuchar reunión) y Abrir historial, sin
  pasar por la ventana.
- **Traductor:** banderas en ambos selectores de idioma (y en el resultado),
  botón Copiar con confirmación, y botón Escuchar para reproducir la
  traducción de nuevo (funciona aunque la lectura automática esté apagada:
  un clic explícito es el permiso).

## [1.5.0] — 2026-07-15

La tanda premium: los momentos que separan una app de hackathon de una app
de verdad, más el design system cerrado de punta a punta.

### Añadido

- **Panel de Permisos** (Ajustes → General): accesibilidad, micrófono y
  grabación de pantalla con su estado real y botón directo a Ajustes del
  Sistema. Se refresca solo al volver a la app. Adiós a descubrir permisos
  a golpes, feature por feature.
- **Estado del sistema real en Inicio:** los checks del panel ya no son
  decorativos; consultan el modelo seleccionado y los permisos de verdad, y
  si algo falta lo marcan en lacre con enlace directo a arreglarlo.
- **Cold start honesto:** el primer dictado del día muestra "Preparando el
  motor…" mientras carga el modelo, en vez de un "Transcribiendo" eterno que
  parecía cuelgue.
- **La ventana recuerda su tamaño y posición** entre lanzamientos.

### Cambiado

- **Onboarding reconstruido con el design system:** botones y tarjetas de
  marca (antes componentes ad-hoc pensados para fondo oscuro), estados
  concedidos en verde consistente, foco de teclado visible.
- **54 títulos y botones en español** corregidos a mayúscula inicial natural
  (adiós al Title Case heredado del inglés).
- Etiquetas de accesibilidad (aria/title) traducidas en toda la app; verdes
  de estado unificados.

## [1.4.0] — 2026-07-15

La tanda de inclusión, nacida de una sugerencia de la comunidad: Escriba para
todos los ojos.

### Añadido

- **Tema Día / Noche / Sistema** en Ajustes → General. El modo nocturno (tinta
  y oro sobre fondo oscuro) ya existía siguiendo al sistema; ahora se puede
  fijar a mano, independiente de macOS.
- **Tamaño del texto** (90% a 130%): agranda toda la interfaz de una vez, para
  ojos cansados o vista reducida. Cuatro escalas curadas, sin perderse en
  porcentajes.

### Cambiado

- Barrido de colores fuera de paleta que desentonaban en el modo oscuro:
  alertas, botones de peligro (ahora en rojo lacre de marca) y esqueletos de
  carga. Foco de teclado visible en todos los botones.

## [1.3.2] — 2026-07-15

### Cambiado

- **La pluma de Escriba llegó a la barra de menú:** los 9 iconos del tray
  (reposo, grabando, transcribiendo × tres temas) ahora son la marca propia,
  en dorado con punto lacre al grabar. Era el último rastro visual de Handy,
  y el más visible de todos (QA de Flor).
- La pantalla de Agentes (MCP) explica cómo reconectar el agente si la
  dirección cambió al actualizar desde una versión sin token.

## [1.3.1] — 2026-07-15

Los dos hallazgos del primer QA real de la dupla (Flor, 15-jul), cerrados el
mismo día.

### Añadido

- **Aviso de motor de IA local faltante** en Traductor, Intérprete, Sesiones y
  Estudio: qué falta, cuánto pesa y botón directo a instalarlo. Antes esas
  pantallas fallaban en silencio para todo usuario nuevo.
- **Indicador global de modo activo:** si hay una sala del Intérprete, un
  Traductor escuchando o una Sesión viva, un pill flotante lo muestra desde
  cualquier pantalla, con botones Ver y Detener. Una sala olvidada ya no se
  traga los dictados sin explicación.

### Cambiado

- "Detener sala" del Intérprete ahora es un botón inconfundible (rojo lacre,
  ancho completo), no un enlace gris.

## [1.3.0] — 2026-07-15

La voz completa el círculo: Escriba ahora también **lee** y **escucha los dos
lados de una reunión**. Y una auditoría operacional de punta a punta dejó la
app más honesta, más robusta y sin rastros del upstream donde no correspondía.

### Añadido

- **Audio del sistema (Sesiones · Escuchar):** un botón y Escriba también oye
  lo que suena en el computador: la otra parte de tu reunión Zoom/Meet, un
  video, un webinar. Cada intervención se corta con el mismo VAD neural del
  micrófono, se transcribe local y entra como turno de "Otros" con marca de
  tiempo. El acta final incluye ambas voces. Escriba se excluye a sí misma de
  la captura (su voz no entra al acta). Solo macOS 13+; captura nativa con
  ScreenCaptureKit compilada dentro del binario.
- **Tu tinta en voz (⌥⇧R):** selecciona texto en cualquier aplicación y
  Escriba lo lee en voz alta con la cascada de motores de voz. El portapapeles
  se restaura solo: leer no deja huellas.
- **Tonos por app:** reglas por aplicación (WhatsApp casual, correo formal,
  IDE → prompt estructurado) que se aplican también al dictado normal, sin
  atajo especial.
- **Manos libres en Sesiones:** el detector de voz corta cada intervención en
  los silencios; hablas sin tocar el atajo. El idioma queda fijado durante la
  sesión (adiós detecciones erráticas en frases cortas).
- El documento final de una sesión **atribuye hablantes por contexto** cuando
  el diálogo lo permite (sin inventar cuando no se distingue).

### Corregido

- **El portapapeles ya no se pierde si el pegado falla** (p. ej. permiso de
  Accesibilidad revocado a mitad de sesión): se restaura siempre.
- Cerrar la ventana con una Sesión activa **apaga el micrófono y la captura
  del sistema**; salir de la app corta cualquier lectura en voz alta en curso.
- Fallos que antes eran silenciosos ahora avisan: resumen del Estudio, sala
  del Intérprete, chequeo e instalación de actualizaciones, extracción de
  modelos. Dictar con el atajo durante una sesión manos libres explica el
  motivo en vez de "error desconocido".
- Cuatro arreglos de raíz heredados del upstream: bajar el límite del
  historial ya no borra grabaciones al teclear, restauración del portapapeles
  con margen para apps lentas, los atajos ya no aceptan combinaciones de solo
  modificadores, y la carga de modelos no queda trabada tras un fallo.

### Cambiado

- Auditoría de marca: la bandeja, el título de la ventana, las novedades y los
  enlaces de actualización y apoyo ahora son 100% de Escriba (la atribución a
  Handy en Acerca de se mantiene, como corresponde).
- Onboarding y Avanzado completamente en español; terminología unificada.
- Limpieza del repositorio: sponsors e infraestructura del upstream fuera.

## [1.2.0] — 2026-07-12

Rework completo de la interfaz y nacimiento del **sistema de diseño de Escriba**.
Se recorrieron las once pantallas una por una y, al terminar, se destiló todo en
un sistema reutilizable para que Escriba se sienta coherente incluso cuando sume
nuevas funciones.

### Diseño

- **Rework de las 11 pantallas:** Inicio, Modelos, Historial, Estudio, Traductor,
  Intérprete en vivo, Agentes (MCP), Escritura Inteligente, General, Avanzado y
  Acerca de. Cada una repensada como experiencia, no como formulario.
- **Escriba Design Guide (sistema de diseño v2):** filosofía y principios, color,
  tipografía, iconografía, espaciado y _layout_, forma, **motion**, estados,
  **accesibilidad**, componentes, patrones y microcopy.
- **Consolidación en código:** tokens únicos de forma (un radio, dos sombras, un
  borde), primitiva `Card` (config/hero/metric), biblioteca de estados
  (vacío/cargando) y comportamiento definido (transiciones 150–220 ms, estado
  _pressed_ en botones, `prefers-reduced-motion` global).
- **Inicio como experiencia:** _"Habla. Escriba hace el resto."_ con estado del
  sistema y estadísticas; barra lateral reorganizada por contexto.

### Añadido

- **Dashboard de Agentes (MCP) en vivo:** tiempo activo, número de llamadas,
  actividad reciente y agentes conectados, con **datos reales** del servidor.
- **Panel de estado del sistema** en Avanzado (modelo, MCP, overlay, inicio) y
  herramientas de usuario avanzado (abrir carpeta de logs y de datos).
- **Escritura Inteligente:** las plantillas de IA como biblioteca visual, con
  ejemplos de entrada → salida.
- **Acerca de:** tarjeta de identidad y _"Escriba en números"_.

### Cambiado

- **Microcopy** con tono más humano y tranquilo (_"No pudimos… Inténtalo de nuevo."_).
- **Lema** ajustado a _"Tu voz en tinta"_ (sin coma), en el _wordmark_ y Acerca de.
- **Firma de macOS:** el `.app` deja de reutilizar un certificado ajeno; en el
  repo va ad-hoc (portable, compatible con CI) y los _builds_ locales usan un
  certificado propio **Escriba Self-Signed**.

## [1.1.0] — 2026-07-11

Auditoría de seguridad completa antes de repartir la app. La promesa de Escriba
—**tu voz nunca sale de tu computador**— ahora es literal y auditable.

### Seguridad

- **Agentes (MCP) autenticados:** el servidor local ahora exige un token secreto
  (en la URL que copias al agente). Sin él, cualquier otro programa de tu equipo
  podía leer tu historial de dictados; ahora responde `401`. El token es estable
  entre reinicios, así que configuras el agente una sola vez.
- **Protección anti-rebinding:** una página web maliciosa ya no puede hablarle al
  servidor local para robar tu historial (se validan `Host`/`Origin` → `403`).
- **Intérprete en vivo protegido:** la sala usa un token largo aleatorio (además
  del código de 4 dígitos) y limita los intentos por IP, para que nadie en el
  mismo WiFi adivine el código y espíe la traducción.
- **Menos permisos:** la interfaz ya no puede leer ni escribir toda tu carpeta de
  usuario, solo los datos de la app; se activó una CSP estricta.
- **Privacidad en los logs:** el texto que dictas ya no se guarda en los archivos
  de registro (los que uno adjunta a un reporte de error).

## [1.0.0] — 2026-07-11

Primera versión mayor: la app queda con marca propia completa y toda la interfaz
en español. De aquí en adelante avanzamos 1.0 → 1.9 → 2.0.

### Cambiado

- **Rebrand visual completo:** la app adopta la paleta pergamino / tinta / oro de
  la landing y el logo oficial de la marca (pluma, onda de voz y firma) en la
  barra lateral, el ícono del Dock y el banner del repo.
- **Todo en español:** los idiomas de transcripción y los dispositivos de audio se
  muestran en español ("Auto Detect" → "Detección automática", "Default" →
  "Predeterminado").

## [0.10.0] — 2026-07-11

### Agregado

- **Agentes (MCP):** servidor local (JSON-RPC sobre HTTP, solo `127.0.0.1`) para
  que Claude Code, Cursor o Cline usen la transcripción, traducción y resumen de
  Escriba como herramientas. 100% local.
- **Re-transcribir con otro modelo** en Historial y Estudio: mismo audio, más
  precisión, sin volver a subir nada.
- **Micrófono en los campos** de la propia app: dicta directo en cualquier campo.

## [0.9.0] — 2026-07-11

### Agregado

- **Traductor cara a cara:** conversación 1-a-1 bidireccional con detección
  automática de idioma, pantalla grande y voz.
- Soporte de **lituano** y conservación del historial de sala al cambiar de
  idioma (feedback de Flor y su señora).
- **Copiar texto** en Estudio e Intérprete y selección de texto en toda la app.
- Nombres de idioma en español entre paréntesis en los selectores.

## [0.8.5] — 2026-07-11

### Agregado

- **Intérprete en vivo:** el equipo levanta una sala en la red local y muestra un
  **QR**; cada asistente lo abre en su teléfono y lee los subtítulos **en su
  propio idioma**, traducidos por el motor local (una vez por idioma). Para guías
  turísticos, clases y charlas con extranjeros.
- Micrófono en vivo del guía (dictado → sala), selector de idioma de origen,
  cambio de idioma en vivo desde el teléfono sin reiniciar y mejor voz TTS.

## [0.8.0] — 2026-07-10

### Agregado

- **Supresión de ruido de fondo (RNNoise):** limpia ventilador, teclado y tráfico
  del micrófono antes de transcribir. 100% local.
- **Buscar y reemplazar:** reglas propias (texto literal o expresión regular) que
  se aplican al texto dictado después de la corrección con IA.
- **Pausar la música al dictar:** pausa Música/Spotify mientras hablas y las
  reanuda al terminar; solo se pausa lo que estaba sonando (macOS por ahora).
- **Reparación de configuración:** si un campo de ajustes se corrompe, se repara
  solo ese campo en vez de perder toda la configuración.

## [0.5.0] — 2026-07-09

### Agregado

- **Estudio de transcripción:** arrastra un audio o video → transcripción con
  marcas de tiempo → exporta **SRT / VTT / TXT / JSON** + **resumen con IA**.
  Incluye soporte de audios **`.opus` de WhatsApp**.
- **Onboarding es-first:** recomendaciones de modelo por idioma y catálogo en
  español (antes recomendaba un modelo solo-inglés a todos).
- **Estadísticas de uso** (palabras, racha, tiempo ahorrado).

## [0.4.0] — 2026-07-08

### Agregado

- **Háblale a cualquier texto:** selecciona texto en cualquier app, mantén el
  atajo y dicta la instrucción (_"hazlo más formal"_, _"resúmelo en 3 líneas"_,
  _"tradúcelo al portugués"_); el texto se reemplaza en el lugar.

## [0.3.0] — 2026-07-08

### Agregado

- **Poderes de dictado:** dictado natural (limpia muletillas y repeticiones),
  presets de comunidad y diccionario personal conectado a la corrección.
- **Traducción al dictar:** hablas en un idioma y el texto se pega en otro.

### Cambiado

- Post-proceso con IA des-enterrado: pasa de ser una función experimental
  escondida a una capacidad principal de la app.

## [0.2.0] — 2026-07-07

### Agregado

- **Motor de IA local:** sidecar de LLM local incluido, con cascada de respaldo
  (local → Ollama → Apple Intelligence → texto crudo). Sin claves de API en el
  camino feliz.

### Cambiado

- **Rebrand completo de Handy a Escriba:** identidad, marca y wordmark propios.

---

La versión 0.1 corresponde a la base original de
[Handy](https://github.com/cjpais/handy); la historia propia de Escriba empieza
en 0.2.0.
