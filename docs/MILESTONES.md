# Hitos

Cada hito termina compilando, con tests pasando y una demo ejecutable.

| # | Hito | Estado | Demo |
|---|---|---|---|
| 1 | **forja-core**: dominio, plan, progresión doble, rachas + "nunca fallar dos veces", seguridad lumbar, nutrición, SQLite + migraciones, zona America/Santiago, tests + proptest + simulación de 12 semanas. Incluye XP, niveles y calor de la forja (los necesita la simulación). | **Hecho** | `cargo run` abre la pantalla "Hoy" en memoria con simulador de tiempo |
| 1b | Agregado al hito 1: equipo real del usuario, animaciones de los 61 ejercicios/calentamientos/estiramientos, detector de ejercicio (núcleo; cámara real en iPhone), comidas por foto con Claude. | **Hecho** | Demo: botón «Detector de ejercicio» y «Foto de comida» |
| 2 | Motor de juego completo en el core: misiones, jefes semanales, logros, árbol de habilidades, botín (tablas ya creadas por la migración 2). | Pendiente | Simulación por consola de jefes y misiones |
| 3 | App de escritorio Tauri 2 (Windows): Hoy, Plan, Progreso, juego completo, importación del JSON de la PWA (`forja4x` v1) con XP retroactivo. | Pendiente | Instalador .msi |
| 4 | App iOS: paridad de funciones + animaciones (120 Hz), hápticas (Core Haptics) y sonido. | Pendiente | Build en GitHub Actions macOS |
| 5 | Integraciones iOS: HealthKit, notificaciones locales, widgets, Live Activity, App Intents. | Pendiente | |
| 6 | Apple Watch (watchOS 8): entrenamiento, descanso háptico, complicación ClockKit, sincronización. | Pendiente; se salta si Xcode ya no permite watchOS 8 / armv7k (decisión del usuario) | |
| 7+ | Ideas 1, 2, 9, 3, 6, 5, 8, 10, 7, 4 (en ese orden). | Pendiente | |
| final | Web/PWA con Leptos + WASM en GitHub Pages (`juliostito/forja`). | Opcional | |
