# Forja

App de entrenamiento y hábitos gamificada como un RPG, con un único objetivo: entrenar 4 veces por semana
para siempre y bajar de peso cuidando la espalda.

* `docs/ARCHITECTURE.md` — plan de arquitectura (1 página) y advertencias de compilación.
* `docs/MILESTONES.md` — hitos y estado.
* `docs/DECISIONS.md` — decisiones e interpretaciones del pedido.
* `docs/BUILD-WINDOWS.md` — cómo compilar en Windows.
* `MECHANICS.md` / `mechanics.json` — todos los números de las reglas y de dónde salen.

## Estructura
* `forja-core/` — núcleo en Rust (hito 1, listo): reglas, SQLite, tests.
* `src/` — demo ejecutable del hito 1 (macroquad).
* Próximos: `forja-ffi/`, `apps/desktop/` (Tauri), `apps/ios/`, `apps/watchos/`, `apps/web/`, `server/`.

Privacidad: los datos quedan en el dispositivo. Cero analítica de terceros.
