# Forja — hoja de mecánicas (hito 1)

Fuente de cada valor: **pedido** = el brief del usuario (sección indicada); **estimado** = decisión nuestra
donde el pedido no da el número. Todo está implementado en `forja-core` y cubierto por tests.

## Perfil y equipo (sección 2)
| Valor | Dato | Fuente |
|---|---|---|
| Sexo / edad / altura / peso inicial | hombre / 30 / 173 cm / 104 kg | pedido §2 |
| Zona horaria | America/Santiago | pedido §1.1 |
| Días y hora de entrenamiento | lun, mar, jue, vie · 07:00 | pedido §2 |
| Mango de mancuerna (fierro) | 1,5 kg | usuario (respuesta) |
| Discos | 4 × 2,5 kg · 6 × 2 kg · 6 × 1,25 kg | usuario (respuesta) |
| Discos que caben en un mango | 8 | **estimado** (configurable) |
| Tope por mancuerna en par | 16,25 kg | calculado del inventario |
| Tope con una sola mancuerna | 19,5 kg (31 kg si cupieran todos los discos) | calculado del inventario |

## Progresión (§3.6)
| Valor | Dato | Fuente |
|---|---|---|
| Regla | todas las series al tope del rango, con buena técnica → sube | pedido |
| Salto de carga | +1,25 kg → carga armable más cercana (empate: la menor) | pedido + **estimado** (redondeo) |
| Carga inicial no armable (p. ej. 7 kg en par) | se ajusta a la más cercana (6,75 kg) | **estimado** |
| En el tope del equipo | tempo 4 s → pausa 2 s → reps 1½ → unilateral | pedido |
| Flexiones | pared → banca → escalón → rodillas → suelo (empieza en banca) | pedido |
| Sesiones que hacen progresar | completas y recuperadas fuera de descarga | **estimado** |

## Plan y fases (§3.1, §3.6)
| Valor | Dato | Fuente |
|---|---|---|
| Semana | Upper A · Lower A · caminar+movilidad · Upper B · Lower B · libre · revisión | pedido |
| Ciclo | 18 semanas: Fase 1 (1–8), descarga (9), Fase 2 (10–17), descarga (18) | pedido |
| Descarga | mitad de las series (redondeo hacia arriba: 3→2, 2→1) | pedido + **estimado** (redondeo) |
| Prueba de fuerza | semanas 4, 8, 12, 16 de cada ciclo | pedido ("cada 4 semanas") |
| Versión mínima | flexiones 2×10, goblet 2×10, remo pecho apoyado 2×10, puente 2×15 | pedido |
| Descanso entre series | 90 s · 45 s en isométricos | pedido |

## Rachas y calor (§5.2)
| Valor | Dato | Fuente |
|---|---|---|
| Calor inicial | 50 | **estimado** |
| Sesión cumplida | +25 (tope 100) | pedido |
| Día planificado fallado | −35 | pedido |
| Día sin actividad | −5 | pedido |
| Multiplicador XP | <30 ×0,8 · 30–69 ×1 · 70–89 ×1,25 · ≥90 ×1,5 | pedido |
| Ventana "Reavivar la forja" | 2 días tras el fallo | pedido ("48 h") |
| Revivir | racha intacta, calor = máx(calor, 50) | pedido |
| Ventana vencida sin cumplir | cuenta como segundo fallo: racha a 0, arma agrietada | **estimado** |
| Recuperación | solo en día de descanso dentro de 2 días; cuenta como hecha | pedido |
| Tramos de llama | 1, 4, 8, 16, 24, 52 sesiones | pedido |
| Motivo para saltar | ≥ 25 caracteres | pedido |

## XP y niveles (§5.1, §5.8)
| Valor | Dato | Fuente |
|---|---|---|
| Sesión completa / mínima / recuperada | 100 / 50 / 80 | pedido |
| Serie / calentamiento / estiramientos | 5 / 15 / 15 | pedido |
| Hábito / día perfecto / pesaje / prueba de fuerza | 5 / +25 / 10 / 60 | pedido |
| Curva | xp_para_nivel(n) = round(120 · n^1,35) | pedido |
| Títulos | Aprendiz 1–4 · Ayudante 5–9 · Oficial de fragua 10–19 · Herrero 20–34 · Maestro herrero 35–49 · Forjador de leyenda 50+ | pedido |
| Tope por sesión | 250 XP base; solo cuentan series planificadas | **estimado** |
| Anti-trampas | < 8 min o sin series → ×0,5 y no cuenta para jefes | pedido + **estimado** (×0,5) |
| Verificado por Apple Watch | ×1,1 | pedido |
| El multiplicador de calor aplica a | todo XP (sesión, hábitos, pesaje, prueba) | **estimado** |

## Seguridad lumbar (§3.5)
| Valor | Dato | Fuente |
|---|---|---|
| Semáforo | 0–3 y no peor → verde; 0–3 pero peor → ámbar (ofrece cambios); > 3 → rojo (cambia solo) | pedido + **estimado** (ámbar) |
| Sustituciones | rumano → hip thrust; rumano 1 pierna → puente 1 pierna; split/búlgara → sentadilla en pared; goblet → banca alta sin peso; press hombro → floor press; remo una mano → remo pecho apoyado | pedido |
| Banderas rojas | dolor a la pierna, hormigueo, debilidad (vigentes 14 días) o dolor que empeora 2 semanas → mensaje médico + pausa de carga axial | pedido + **estimado** (14 días; umbral "empeora": ≥4 reportes, +1 punto, > 3) |
| Flexión lumbar con carga | prohibida en el catálogo (test) | pedido |

## Nutrición (§3.7)
| Valor | Dato | Fuente |
|---|---|---|
| Fórmula | Mifflin-St Jeor con el peso más reciente | pedido |
| Factor de actividad / déficit / redondeo | 1,45 / 600 kcal / 50 kcal | pedido |
| Kcal con el perfil inicial | **2.250** | usuario confirmó 2.250 |
| Proteína | 2 g × (altura − 100) → 145 g | pedido |
| Grasas / agua / fibra | 28 % kcal (70 g) / 0,033 L/kg (3,4 L) / 30 g | pedido |
| Piso de kcal | nunca bajo el metabolismo basal | **estimado** |
| Estancamiento | promedio 14 días no baja vs. 14 anteriores (≥2 pesajes c/u) → sugerir −150 kcal | pedido + **estimado** (mín. pesajes) |
| Meta de pasos | 6.000 + 1.000 cada 2 semanas, tope 10.000 | pedido |

## Detector de ejercicio (idea 3, adelantada)
| Valor | Dato | Fuente |
|---|---|---|
| Entrada | puntos del cuerpo (cabeza, cuello, hombros, codos, muñecas, cadera, rodillas, tobillos) | Vision en iPhone; en la demo, la figura animada |
| Movimientos | sentadilla, bisagra, puente, flexión, press, remo, curl, core | **estimado** |
| Acostado si | inclinación media del tronco > 55° | **estimado** |
| Conteo de reps | filtro paso bajo (α 0,35) + histéresis 30 % / 70 % | **estimado** |
| Profundidad ok | cadera a ≤ 0,15 torsos sobre la rodilla en el punto más bajo | **estimado** |
| Aviso de postura | cabeza/espalda alta > 35° fuera de línea con el tronco (orientativo) | **estimado** |
| Confianza mínima para registrar solo | 60 % (debajo: sugiere y confirmas) | pedido §6 + **estimado** |

## Comidas por foto (Claude)
| Valor | Dato | Fuente |
|---|---|---|
| Modelo | claude-opus-5-5 (cambiable con FORJA_CLAUDE_MODEL) | **estimado** |
| Foto enviada | JPEG ≤ 1280 px, calidad 85; solo la foto | **estimado** |
| Calorías creíbles por comida | 1–5.000 kcal | **estimado** |
| Proteína de la comida | se suma al contador de proteína del día | **estimado** |
| Clave | Administrador de credenciales de Windows o ANTHROPIC_API_KEY | **estimado** |

## Controles de la demo
| Control | Acción |
|---|---|
| Clic en un círculo de serie | registra la serie al tope del rango con la carga sugerida (clic de nuevo la borra) |
| Clic derecho en una serie | −1 rep |
| Espacio | marca la siguiente serie |
| N | mañana a las 06:45 (simulador) |
| Clic en el nombre del ejercicio | abre técnica, nota de espalda, alternativa, demostración animada y el detector |
| Pasar el mouse sobre un calentamiento o estiramiento | muestra su animación |
| Foto de comida | eliges la foto, Claude la analiza y queda registrada |
