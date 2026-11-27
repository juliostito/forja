# Forja — Plan de arquitectura (1 página)

**Objetivo único:** que entrenes 4 veces por semana para siempre y bajes de peso cuidando tu espalda.
Cada pieza de abajo existe para eso; si una mecánica no sube la constancia, se elimina.

## Capas

```
┌──────────────────────────── Apps (solo UI, sin reglas) ────────────────────────────┐
│ iOS 17 SwiftUI  │ watchOS 8 SwiftUI │ Escritorio Tauri 2 (Windows) │ Web Leptos/WASM │
└───────┬─────────┴─────────┬─────────┴──────────────┬───────────────┴───────┬─────────┘
        │ UniFFI (Swift)    │ WatchConnectivity      │ Rust directo          │ wasm-bindgen
┌───────▼─────────┐  ┌──────▼──────────┐   ┌─────────▼────────────────────────▼─────────┐
│ forja-ffi       │  │ Lógica mínima   │   │ forja-core (crate Rust, sin UI, sin unsafe)│
│ (único unsafe)  │  │ en Swift (reloj)│   │ plan · progresión · rachas · seguridad     │
└───────┬─────────┘  └─────────────────┘   │ lumbar · nutrición · XP/calor · revisión   │
        └──────────────────────────────────►│ modelo + SQLite versionado (rusqlite)      │
                                            └────────────────────────────────────────────┘
```

* **forja-core** es la única fuente de verdad de las reglas. Todas las funciones reciben la hora como
  parámetro (`now: DateTime<Utc>`), así que todo es determinista y el "simulador de tiempo" es gratis.
* **Zona horaria:** `America/Santiago` (chrono-tz). Chile cambia la hora a medianoche: la medianoche de
  inicio de horario de verano **no existe**. Los días se manejan como `NaiveDate` local y las horas
  locales se resuelven con `time::resolve_local` (huecos → primer minuto válido; repetidas → la primera).
* **Datos:** SQLite del core (`rusqlite` con `bundled`), esquema versionado con `PRAGMA user_version` y
  migraciones en orden. `xp_event` es un libro contable de solo-agregar (triggers lo impiden). Nivel,
  calor y racha se **recalculan** desde los registros, nunca se guardan como verdad.
* **Privacidad:** todo queda en el dispositivo (o en iCloud del usuario). Cero analítica de terceros.
  Las fotos nunca entran en la base: solo una referencia local.

## Decisiones clave

| Tema | Decisión | Por qué |
|---|---|---|
| Persistencia iOS | SQLite del core (no SwiftData) | Una sola fuente de verdad y las mismas migraciones en iPhone, PC y web. |
| Reloj (Series 3) | **Lógica mínima en Swift** + WatchConnectivity | El Series 3 es **armv7k** (32 bits), no `arm64_32`: el target de Rust `armv7k-apple-watchos` es tier 3 (requiere nightly + build-std). Ver `DECISIONS.md`. |
| Complicación | ClockKit | WidgetKit para complicaciones exige watchOS 9; el Series 3 llega a watchOS 8. |
| Escritorio | Tauri 2 sobre forja-core | Es lo primero que puedes usar sin Mac. |
| Sync | Local-first: iPhone fuente de verdad; PC por JSON exportado (hito 3) y luego servidor Axum cifrado E2E (idea 9). | Funciona sin internet. |
| Unidades | Pesos en gramos enteros (`Grams`) | Progresión exacta con discos de 1,25 kg, sin errores de coma flotante. |

## Módulos de forja-core (hito 1)

`units` (kg con coma chilena) · `time` (Santiago, DST) · `profile` (perfil, equipo, contrato) ·
`catalog` (ejercicios, calentamientos, estiramientos) · `plan` (semana, fases de 18 semanas, descarga,
variantes, versión mínima) · `progression` (doble progresión con cargas que **sí** puedes armar) ·
`streak` (racha, "nunca fallar dos veces", recuperación, calor de la forja) · `safety` (semáforo de dolor,
sustituciones, banderas rojas) · `nutrition` (Mifflin-St Jeor, estancamiento) · `habits` · `xp` (XP,
niveles, títulos, multiplicador por calor, anti-trampas) · `review` (revisión semanal por reglas) ·
`store` (SQLite + migraciones + respaldo JSON) · `forja` (fachada que usan todas las apps).

## Requisitos de compilación (advertencia)

* El **núcleo** y la app de **escritorio** se compilan en tu Windows (Rust + MSVC).
* **iPhone y Apple Watch requieren macOS con Xcode** y una cuenta Apple Developer
  (gratis: la app caduca cada 7 días; USD 99/año: sin caducidad y con TestFlight).
  Sin Mac: (a) GitHub Actions con `macos-latest` + firma con fastlane match; (b) Mac prestado.
* **Riesgo serio con el Series 3:** según lo que sé, Xcode 14 dejó de compilar para armv7k, así que una app
  para el Series 3 necesitaría Xcode 13.x (que solo corre en macOS ≤ 12/13) y ya no puede subirse a la
  App Store (que exige SDK recientes). Instalarla con Xcode 13 en un Mac antiguo debería seguir siendo
  posible. **Hay que verificarlo en una máquina real antes del hito 6** (`xcodebuild -showsdks`).
