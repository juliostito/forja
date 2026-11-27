# Compilar Forja en Windows (sin experiencia previa)

1. Instala **Rust**: descarga `rustup-init.exe` desde https://rustup.rs y acepta lo que propone.
   Si te pide las "Build Tools de Visual Studio", instálalas con la carga "Desarrollo para el escritorio con C++"
   (SQLite viene incluido y se compila con ellas).
2. (Opcional) Instala **just**: `winget install Casey.Just`.
3. Abre PowerShell en la carpeta del proyecto y ejecuta:
   * `cargo test -p forja-core` — corre las 57 pruebas (incluye la simulación de 12 semanas).
   * `cargo run --release` — abre la demo del hito 1.
   * o con just: `just core-test` y `just demo`.

## La demo del hito 1
* Arranca con **tu perfil** y **sin registros**. Todo vive en memoria: al cerrar la ventana no queda nada guardado.
* Abajo está el **simulador de tiempo** (modo desarrollador): "+30 min", "Mañana 06:45", "+1 semana".
  Sirve para ver la descarga, la Fase 2, el "Fallaste ayer" y la recuperación sin esperar días reales.
* `forja-demo --screenshot captura.png` guarda una imagen y sale; `forja-demo --self-test resultado.json` recorre
  el ciclo básico solo y guarda el resultado.

## iPhone y Apple Watch
Requieren macOS con Xcode (ver `ARCHITECTURE.md`, sección "Requisitos de compilación"). Se preparan en los
hitos 4–6 con GitHub Actions (`macos-latest`) y fastlane match.
