# Decisiones técnicas (registro)

## D1 — SQLite del core también en iOS
Una sola fuente de verdad, mismas migraciones y mismos tests en todas las plataformas. SwiftData
duplicaría el modelo y las reglas.

## D2 — Apple Watch Series 3 = armv7k
El pedido mencionaba `arm64_32-apple-watchos`, pero ese target corresponde a Series 4 en adelante.
El Series 3 (chip S3) es **armv7k**. Rust tiene `armv7k-apple-watchos` solo como tier 3 (sin binarios
oficiales; requiere nightly y `-Z build-std`). Decisión: el reloj usa una **versión mínima de la lógica en
Swift** (serie actual, descanso, pulso) y manda los registros al iPhone por WatchConnectivity; el iPhone
aplica las reglas del core. Se documentará el resultado real al llegar al hito 6.

## D3 — Complicaciones con ClockKit
WidgetKit para complicaciones requiere watchOS 9. En watchOS 8 se usa ClockKit (CLKComplicationDataSource).

## D4 — Pesos en gramos enteros
`Grams(u32)`. Los discos de 1,25 kg hacen que los `f32` acumulen error; con enteros la progresión es exacta.

## D5 — Hora como parámetro
Ninguna función del core lee el reloj del sistema. La app pasa `now`. Así el simulador de tiempo del modo
desarrollador y los tests de 12 semanas usan exactamente el mismo código.

## D6 — Interpretaciones del pedido (marcadas como estimadas en MECHANICS.md)
* ~~Inventario de discos: 2 de cada uno + mango de 2 kg~~ — reemplazado por el inventario real (D7).
* "+1,25 kg": se elige la carga armable más cercana a actual + 1,25 kg (empate → la menor).
* Kcal: la fórmula pedida da **2.250 kcal** con tu perfil (no ~2.150). Se usa la fórmula; el factor de
  actividad es configurable (con 1,40 daría 2.150).
* Ventana de "Reavivar la forja": los 2 días siguientes al fallo. Si se vence sin cumplir, cuenta como
  segundo fallo seguido.
* Calor inicial: 50.

## D7 — Respuestas del usuario (27-11-2026)
* Kcal: se queda la fórmula, **2.250 kcal**.
* Equipo real: mangos de fierro de ~1,5 kg; discos 4 × 2,5 kg, 6 × 2 kg, 6 × 1,25 kg; una banca. Se asume que
  caben 8 discos por mango (configurable): tope 16,25 kg por mancuerna en par y 19,5 kg con una sola.
* Apple Watch Series 3: si la toolchain no permite compilar para watchOS 8 / armv7k, **ese hito se salta**.

## D8 — Detector de ejercicio con cámara
El detector vive en el núcleo (`pose.rs`): recibe puntos del cuerpo y reconoce el movimiento, cuenta reps,
revisa la profundidad de la sentadilla y da un aviso orientativo de postura. En el iPhone los puntos vienen de
Apple Vision (`VNDetectHumanBodyPoseRequest`, en el dispositivo, sin grabar nada). En el PC no hay un modelo
de poses integrado, así que la demo lo alimenta con la figura animada; conectar la cámara real llega con la
app de iPhone. Se probó contra las animaciones (9 movimientos, 3 reps cada uno).

## D9 — Comidas por foto con Claude
Una foto → Claude estima plato, porciones, kcal y macros → se guarda como comida y su proteína suma al
contador del día. Se usa la clave propia del usuario (Administrador de credenciales de Windows; en iOS irá
al Llavero). Solo se envía la foto reducida; nada del perfil. La foto queda en el dispositivo: la base de
datos guarda solo su ruta. Tabla `meal` en la migración 3.