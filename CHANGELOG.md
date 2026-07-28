# Changelog

Todas las novedades notables de **Escriba**. El formato sigue el estilo de
[Keep a Changelog](https://keepachangelog.com/es/1.1.0/) y el versionado es
[SemVer](https://semver.org/lang/es/).

Escriba es un _rework_ de [Handy](https://github.com/cjpais/handy) (© CJ Pais,
MIT). Esta historia arranca en el fork del 7 de julio de 2026 y recoge lo que
la dupla **Alejandro & Flor** construyó encima para los Juegos Imperiales:
convertir una app de dictado en un motor de IA **100% local y gratis**.

## [2.2.1] — 2026-07-28

**Actualizar ya tiene marcha atrás.** Si abrías una versión anterior después de
haber usado la 2.2.0, la app entraba en un bucle de cierres del que no se salía.
Ya no. Y el Intérprete deja de hacer esperar a toda la sala por la traducción de
uno solo: la frase aparece al instante y cada idioma llega cuando está.

### Corregido

- **Bucle de cierres al volver a una versión anterior** — la 2.2.0 dejaba el
  historial en un formato que las versiones previas no reconocían, y en vez de
  seguir adelante la app se cerraba, se ofrecía reabrir, y vuelta a empezar.
  Ahora una base de datos más nueva no impide arrancar. Sin esto, actualizar era
  un camino de ida.
- **Las estadísticas contaban los días en el huso equivocado** — lo dictado a
  partir de las 20:00 se apuntaba en el día siguiente, así que por la noche lo
  de esa misma mañana aparecía en la barra de ayer, y un mismo día contaba dos
  veces en la racha. Ahora el día es el que viviste tú.
- **El diccionario personal no aceptaba nombres de varias palabras** — "Imperio
  Agéntico" quedaba fuera; ahora entra.
- **El asistente de bienvenida no tenía dónde mostrar un aviso** — si algo
  fallaba durante la instalación, el mensaje no aparecía en ninguna parte.
- **El Estudio ya puede resumir grabaciones largas** — antes, cualquier clase de
  más de veinte minutos no cabía de una vez en el motor local y el resumen
  fallaba culpando al motor, que estaba perfectamente. Ahora se resume por
  partes y luego se juntan, y si algo falla el mensaje dice qué fue.
- **Un archivo a la vez en el Estudio** — soltar varias grabaciones de golpe
  cargaba todas enteras en memoria a la vez. Tardan lo mismo y ocupan mucho
  menos.

### Cambiado

- **El Intérprete ya no hace esperar a nadie** — antes traducía a todos los
  idiomas de la sala antes de mostrar nada, uno detrás de otro: con cinco
  idiomas eran varios segundos de pantalla en blanco, y quien escuchaba en el
  idioma del guía esperaba por traducciones que no necesita. Ahora la frase
  original aparece de inmediato y cada idioma la reemplaza al llegar la suya.
- **Una traducción atascada ya no congela la sala** — se descarta a los 30
  segundos y la sala sigue, en vez de esperar cinco minutos.
- **El Modo Calma se nota** — además del texto más grande, ahora apaga los
  colores de aviso, error y confirmación, que era lo que llenaba la pantalla de
  ruido. Queda un solo color hablando.
- **El asistente de bienvenida trae más sustancia** en cada pantalla, con lo que
  la app hace y lo que cuesta comparada con las alternativas de pago.

## [2.2.0] — 2026-07-28

**Plumín te recibe.** La app ya no te suelta en una pantalla vacía después de
elegir modelo: ahora hay un asistente de primera vez donde Plumín te acompaña
paso a paso y termina pidiéndote que le hables, para que veas tus propias
palabras antes de empezar. Y las notas de Obsidian dejan de ensuciar tu vault.

### Añadido

- **Asistente de bienvenida narrado por Plumín** — seis pasos, y él cambia de
  expresión en cada uno: te recibe, te ayuda a elegir con qué va a escucharte,
  te explica el motor de IA local, tu atajo, tu vault de Obsidian, y al final
  **te pide que digas algo**. Dictas, ves aparecer tus palabras, y puedes
  pedirle ahí mismo que las traduzca. Terminas la instalación habiendo usado la
  app con tu voz, no mirando capturas.
- **Volver a ver la bienvenida** desde Ajustes → Depuración, sin tocar nada de
  tu configuración.
- **Las notas de Obsidian van a su propia carpeta**, creada por la app la
  primera vez. Por omisión `Escriba`, y le puedes cambiar el nombre o dejar el
  campo vacío para que caigan en la raíz como antes.
- **Revisar la nota antes de que toque el vault** — se abre con el título y el
  cuerpo editables, y nada se escribe en disco hasta que confirmas. Tu vault es
  tu segundo cerebro, no una bandeja de salida.
- **Ver el foco también con el ratón**, para quien pierde la referencia de
  dónde está parado en la interfaz. Con esto son cuatro los modos de
  accesibilidad visual, y siguen conviviendo con todo lo demás.

### Corregido

- **Las estadísticas ya son acumuladas** (reporte de Flor). El historial guarda
  cinco entradas y borra el resto, y las estadísticas se calculaban sobre esa
  tabla: "has dictado N veces" y "te has ahorrado X minutos" solo contaban las
  últimas cinco, y lo mismo la racha y la gráfica de la semana. Los datos no
  estaban ocultos, estaban borrados. Ahora hay un contador aparte que no se
  poda nunca. Lo ya perdido no vuelve, pero de aquí en adelante cuenta todo.
- **Instalar el motor local ya lo deja funcionando.** Antes se descargaban
  2,5 GB y no pasaba nada, porque el interruptor que lo enciende vivía en otra
  pantalla.

### Seguridad

- El nombre de la carpeta de notas se sanea y la contención se revalida
  **después** de crear el directorio: el nombre puede ser inocente y aun así
  apuntar fuera del vault si la carpeta ya existía como enlace simbólico.

## [2.1.1] — 2026-07-28

**Lo que salió de probar la app de verdad.** Ocho arreglos, todos nacidos de
una sesión de pruebas sobre la 2.1.0: dos rompían funciones enteras, tres eran
cosas que se veían mal, y uno era una función que decía hacer algo y no lo
hacía.

### Corregido

- **"Terminar y crear documento" moría a los 30 segundos.** El límite de
  espera del motor está pensado para pulir un dictado de dos frases, pero el
  documento de sesión le manda la transcripción entera: en el motor local,
  procesarla puede tardar más que eso antes de escribir la primera palabra. El
  documento se cortaba siempre, y el aviso culpaba al motor, que estaba sano.
  Ahora las peticiones largas tienen su propio margen.
- **El "¿" caía en el sitio equivocado.** "Hola, cómo ¿estás?" en vez de
  "Hola, ¿cómo estás?": el modelo planta el signo donde subes la entonación,
  no donde empieza la pregunta. Ahora se coloca bien, y también se pone cuando
  el modelo se lo come. Solo actúa donde el español no deja lugar a dudas.
- **Alto contraste no contrastaba.** Solo oscurecía la tinta, pero el tema base
  ya daba 16,58:1 — muy por encima de lo que exige la norma más estricta.
  Ahora quita el tinte del papel: fondo blanco puro, barra lateral negra,
  21:1 de texto. Medido en los dos temas.
- **El QR tapaba su propia explicación**, porque se dibujaba más grande que su
  recuadro.
- **La pantalla del móvil del Intérprete salía corrida**: la barra de controles
  no cabía en pantallas angostas y empujaba la página entera de lado. De paso,
  ahora respeta la muesca del teléfono.
- **La duración de los audios del historial se salía de la tarjeta.**
- **Un error escondido al dictar**: si tenías palabras en tu diccionario
  personal y el modelo devolvía una con signo de apertura pegado ("¡ándale!"),
  la transcripción reventaba por dentro.

### Añadido

- **Ajustes → Avanzado → Obsidian**: ya puedes ver qué vault tienes
  configurado, cambiarlo u olvidarlo. Antes solo se podía elegir tropezándose
  con el selector de carpetas al exportar, y después no había vuelta atrás.
  Ahora además se avisa antes de pedírtelo.
- Cuando el documento de sesión falla, el registro dice **por qué**. Antes
  callaba, y con el motor apareciendo sano en el log la causa era
  indistinguible sin volver a reproducirla.

## [2.1.0] — 2026-07-26

**La ronda del blindaje.** Cinco auditorías dirigidas — pipeline de voz,
frontera IPC, estado, cadena de suministro y accesibilidad — aplicadas a
fondo, con protocolo de verificación escrito (`AGENTS.md`). Y la otra mitad
de la accesibilidad: a la operabilidad total con teclado y lector de
pantalla se suman los modos visuales.

### Añadido

- **Enviar a Obsidian** — el acta de Sesiones y las transcripciones del
  Estudio entran a tu vault como notas Markdown. 100% local: la exportación
  no toca la red.
- **Modo Calma** — le baja el volumen visual a toda la app: animaciones y
  transiciones apagadas (no acortadas), +10% de texto sobre la escala que ya
  elijas, +15% de aire, superficies planas sin degradados ni grano. Aplica
  también a la ventana de grabación, que es donde más importa la quietud.
- **Alto contraste** — tinta y bordes reforzados (AA holgado en ambos temas),
  sin cristal difuminado.
- **Asistencia para daltonismo** — éxito y error/grabando eran el único par
  de estados que solo el color distinguía, y es justo el eje rojo/verde: pasan
  a cian/violeta, distinguible en los tres tipos de daltonismo.
- **Traducir por teclado** — el panel "Escribe tu texto" ahora también
  traduce, no solo corrige. Quien no puede usar la voz en ese momento (aula
  compartida, afonía, discapacidad del habla) ya tiene el motor completo.
- **Palabras por día** — gráfica de actividad de la última semana en Inicio,
  derivada del historial local, sin telemetría.
- **Voz neural inglesa incluida** en el Intérprete: ya no depende de las
  voces que traiga el sistema.
- **Tonos por app en Windows** — la detección de la app activa llega a
  Windows; cada app recibe su tono también ahí.
- **Linux/Wayland** — el onboarding explica la limitación de atajos globales
  y sus alternativas en vez de saltarse el paso en silencio.

### Corregido

- **Toda la app con teclado y lector de pantalla** — los desplegables son
  listbox de verdad (una parada de tabulación, flechas, Escape) y cada
  control de Ajustes anuncia su nombre y su valor, no solo "Abajo al centro".
- **Barge-in completo** — hablar encima de la voz del Intérprete también la
  corta; era la tercera fuente de audio y la única donde el eco ocurría de
  verdad.
- **El Estudio en Windows** — la contención de rutas no contenía nada: las
  rutas canonicalizadas (`\\?\C:\…`) nunca igualaban al home (`C:\…`) y todo
  archivo pasaba por "medio externo". Corregido comparando la letra de unidad
  del prefijo parseado.
- **Apple Events en el build firmado** — la 2.0.0 salió sin el entitlement:
  pausar la música, silenciar al grabar e instalar BlackHole fallaban en
  silencio en el binario distribuido (y funcionaban en desarrollo).
  Confirmado con `codesign` sobre el binario firmado de esta versión.
- **El corte de producción estaba roto** — desajuste de versiones entre el
  crate de Tauri (2.10) y su cliente npm (2.11) que `tauri dev` no avisa.
- **Dictado más rápido y honesto** — el turno cierra en la mitad de tiempo,
  hablar interrumpe la respuesta, y los segmentos donde el motor dudó quedan
  marcados con su confianza.

### Seguridad

- **La voz dictada deja de ser prompt de sistema** — lo transcrito viaja
  vallado como datos: que alguien hable cerca durante una edición por voz ya
  no puede convertirse en instrucción para el motor.
- **Lo sensible no llega al disco** — el audio no se guarda por omisión
  (opción explícita para activarlo) y los números de tarjeta se redactan
  antes de persistir el texto.
- **Red con cinturón** — timeout, reintentos con `Retry-After` y
  cortacircuitos para el proveedor remoto de pulido: un proveedor caído ya no
  cuelga el dictado.
- **Estado y locks** — una sola fuente de verdad para "¿está grabando?", el
  grabador se recupera solo si su worker muere, y los mutex de audio toleran
  envenenamiento saneando a Idle.
- **Cadena de suministro** — 7 avisos de dependencias cerrados comparando
  contra el rango real de cada aviso (rustls-webpki 0.103.13, tar 0.4.46 y
  las transitivas de npm), CI con toolchains fijados y overrides acotados a
  la mayor que declaran sus consumidores.

## [2.0.0] — 2026-07-21

**El Intérprete de Reuniones.** Escriba deja de ser solo dictado: ahora
interpreta videollamadas en vivo, de ida y vuelta, 100% local y sin API
keys. Nació de un comentario de **John Walter** en la comunidad hace tres
días; hoy es una categoría de producto que ningún fork de Handy tiene.

### Añadido

- **Intérprete de reuniones (idea de John Walter)** — con una sesión de
  Reunión activa y el Audio del sistema encendido, Escriba traduce en vivo
  entre tu idioma y el de la llamada, en los dos sentidos:
  - **Lo que suena** (la otra parte del Meet/Zoom, un video) llega ya
    traducido a tu idioma en el acta.
  - **Lo que dictas** en español se traduce al idioma de la reunión, se
    anexa al acta como par bilingüe (`original ⇢ traducción`), se copia al
    portapapeles listo para el chat, y **se dice en voz alta** con la mejor
    voz instalada de ese idioma.
  - Funciona con el atajo de dictado **y con Manos libres** (cero teclas:
    hablas, pausas, suena la traducción — interpretación consecutiva real).
- **Micrófono virtual integrado** — para que la otra persona escuche tu voz
  ya traducida _dentro_ de la llamada. Instalación en **un clic sin salir de
  Escriba** (patrón del motor local): descarga BlackHole 2ch desde la fuente
  oficial verificada por SHA256, lo instala con el diálogo nativo de macOS y
  reinicia el servicio de audio. Al terminar, se autoselecciona como salida.
- **Selector de voz ♀/♂** para el Intérprete, con voces de calidad
  (Premium > Enhanced) según el idioma, en un grupo de controles integrado a
  la estética de la barra (idioma · salida · voz).
- **Crédito visible a John Walter** en la línea de estado del Intérprete,
  igual que Pedro en Apariencia.

### Corregido

- **Estudio con modelos rápidos (Canary y familia ONNX)** — transcribir un
  audio o video largo devolvía texto vacío en silencio. Dos causas apiladas:
  la ventana de troceo era un supuesto de Whisper (8 min) y el idioma se
  pasaba como "auto" a modelos que no autodetectan. Ahora el troceo y el
  idioma se deciden por el modelo cargado. Aplica también al tool
  `transcribe` de Agentes (MCP).
- **Detector de idioma del Intérprete** — frases con palabras sueltas del
  otro idioma ("Vamos **a** probar…", saludos cortos) empataban y no se
  traducían. Lista de palabras funcionales del español completada.
- **El Intérprete nunca queda mudo hacia la reunión**: si la frase ya está en
  el idioma del otro sale tal cual; si el motor local falla, sale el
  original. Los nombres propios y marcas ya no se traducen ("Escriba" dejó de
  volverse "Type").

### Seguridad

- Cierre del plan de _hardening_: nivel de log **Info** por defecto en
  release (los reportes de usuario no arrastran detalle de depuración) y tope
  de 64 oyentes simultáneos por sala del Intérprete en vivo.

## [1.9.1] — 2026-07-18

Correcciones del QA a tres continentes: Flor en el Traductor, y el primer QA
de Linux (un Ubuntu Server resucitado como escritorio, GTX 1060 incluida).

### Corregido

- **Traductor, dirección de idioma** (QA de Flor): el modelo local chico
  decidía mal y "traducía" español→español. Ahora la dirección la resuelve
  una heurística local (por escritura y palabras funcionales del par) y el
  prompt tiene un solo trabajo, con reintento reforzado si el modelo
  parafrasea en el idioma de origen.
- **Traductor, voz** (QA de Flor): la lectura elegía cualquier voz del
  sistema y el inglés sonaba "como un español hablando inglés". Ahora elige
  por calidad (Premium > Enhanced > local), igual que Sesiones.
- **Traductor, conversación** (QA de Flor): solo se veía el último
  intercambio. Ahora la sesión completa queda a la vista, con botones de
  escuchar y copiar por intercambio y autoscroll.
- **Motor local de IA ahora funciona en Linux** (QA pionero de Linux): antes
  decía "plataforma no soportada". Se añadió el runtime de llama.cpp para
  Ubuntu x64 (CPU), verificado con SHA256.
- **El paquete .deb declara `libopenblas0`**: la app ya no falla al abrir en
  Linux por una librería faltante; `apt` la instala sola.

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
