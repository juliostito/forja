# Forja — tareas. Instala `just` con: winget install Casey.Just   (o: cargo install just)
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Tests, clippy y formato del núcleo
core-test:
    cargo fmt --all -- --check
    cargo clippy -p forja-core --all-targets -- -D warnings
    cargo test -p forja-core

# Cobertura de las reglas (requiere: cargo install cargo-llvm-cov)
coverage:
    cargo llvm-cov -p forja-core --summary-only

# Demo del hito 1 (pantalla "Hoy" en memoria con simulador de tiempo)
demo:
    cargo run --release

# XCFramework para iOS/watchOS — hito 4/6, requiere macOS + Xcode
ios-xcframework:
    @echo "Pendiente (hito 4): requiere macOS con Xcode; se ejecuta en GitHub Actions macos-latest."

# App de escritorio Tauri — hito 3
tauri-dev:
    @echo "Pendiente (hito 3): app Tauri 2 sobre forja-core."

# Web/PWA Leptos + WASM — hito final opcional
web:
    @echo "Pendiente (hito final): forja-core a WASM con Leptos."
