#!/usr/bin/env python3
"""Arma index.html de la web app a partir del artefacto Forja + módulos de src/."""
import re, pathlib
ROOT = pathlib.Path(__file__).resolve().parent.parent
src = pathlib.Path("/home/user/forja/forja.html").read_text()
store = (ROOT/"src/store.js").read_text()
extras = (ROOT/"src/extras.js").read_text().replace("__VAPID_PUBLIC__", (ROOT/"vapid-public.txt").read_text().strip())
s = src

def rep(old, new, cnt=1):
    global s
    n = s.count(old)
    assert n == cnt, (old[:70], n)
    s = s.replace(old, new)

def cut(start, end, new, keep_end=True):
    global s
    i = s.index(start); j = s.index(end, i)
    s = s[:i] + new + (s[j:] if keep_end else s[j+len(end):])

# ---------- cabecera de documento ----------
head_end = s.index("<style>")
s = s[head_end:]
HEAD = """<!doctype html>
<html lang="es">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<title>Forja 4×</title>
<meta name="description" content="Tu entrenador personal: plan upper/lower 4×, progresión, hábitos y nutrición.">
<meta name="theme-color" content="#0f1217">
<meta name="apple-mobile-web-app-capable" content="yes">
<meta name="mobile-web-app-capable" content="yes">
<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
<meta name="apple-mobile-web-app-title" content="Forja">
<link rel="manifest" href="manifest.webmanifest">
<link rel="icon" href="icon-192.png">
<link rel="apple-touch-icon" href="apple-touch-icon.png">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Barlow+Condensed:wght@600;700;800&family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@500;700&display=swap">
"""
s = HEAD + s
rep("</style>\n", """html{-webkit-text-size-adjust:100%}
:root{padding-top:env(safe-area-inset-top,0px)}
body{margin:0;min-height:100vh}
body::before{content:"";position:fixed;inset:0 0 auto;height:env(safe-area-inset-top,0px);z-index:30;background:var(--bg)}
.toast{top:calc(12px + env(safe-area-inset-top,0px))}
.prot{display:flex;flex-direction:column;gap:8px;padding:10px;border-radius:8px;background:var(--sunk)}
.bar{height:8px;border-radius:99px;background:var(--line);overflow:hidden}.bar i{display:block;height:100%;background:var(--ok)}
.photos{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px}
.photos figure{margin:0;display:flex;flex-direction:column;gap:4px}.photos img{width:100%;aspect-ratio:3/4;object-fit:cover;border-radius:8px;background:var(--sunk)}
.photos figcaption{display:flex;justify-content:space-between;align-items:center}
.medals{display:grid;grid-template-columns:repeat(auto-fill,minmax(130px,1fr));gap:8px}
.medal{border:1px dashed var(--line);border-radius:8px;padding:10px;display:flex;flex-direction:column;gap:2px;opacity:.55}
.medal.on{opacity:1;border-style:solid;border-color:var(--ok);background:var(--ok-soft)}
.m-ico{font-size:1.3rem;color:var(--warn)}
pre.sum{white-space:pre-wrap;background:var(--sunk);border-radius:8px;padding:10px;margin:0;font-family:var(--f-mono);font-size:.82rem}
details.topic{border-top:1px solid var(--line);padding:10px 0}
details.topic summary{font-size:.95rem}
textarea#subTxt{min-height:90px}
.health{display:flex;flex-direction:column;gap:8px;padding:10px;border-radius:8px;background:var(--sunk)}
.hgrid{display:grid;grid-template-columns:repeat(auto-fill,minmax(92px,1fr));gap:6px}
.hs{display:flex;flex-direction:column;background:var(--surface);border-radius:6px;padding:6px 8px}.hs b{font-family:var(--f-display);font-size:1.4rem;line-height:1.1}
</style>
</head>
<body>
""")
rep('<script type="application/json" id="claude-artifacts-manifest">{"permissions": ["notifications", "menu-bar"]}</script>\n', "", 0) if s.count('claude-artifacts-manifest')==0 else None
s = s.replace('<script type="application/json" id="claude-artifacts-manifest">{"permissions": ["notifications", "menu-bar"]}</script>\n', "")
s = s.replace('<meta name="claude:viewport-fit-sides" content="cover">\n', "")

# ---------- almacenamiento local + módulos nuevos ----------
cut("/* ============ ALMACENAMIENTO", "/* ============ LÓGICA", store + "\n")
rep("/* ============ DEMOSTRACIONES", extras + "\n/* ============ DEMOSTRACIONES")

# ---------- fases / descarga ----------
rep("const ids=SESSIONS[type].ex;", "const ids=sessEx(type);")
rep('${e.sets}×${e.lo}–${e.hi}${e.u==="s"?" s":""}${e.side?"/lado":""}</span></summary>', '${nS(e)}×${e.lo}–${e.hi}${e.u==="s"?" s":""}${e.side?"/lado":""}</span></summary>')
rep("  for(const id of s.ex){", "  for(const id of sessEx(type)){")
rep("const complete = rows.filter(r=>r&&r.ok).length>=e.sets;", "const complete = rows.filter(r=>r&&r.ok).length>=nS(e);")
rep("${e.sets}×${e.lo}–${e.hi}${e.u===\"s\"?\" s\":\"\"}${e.side?\" /lado\":\"\"}${e.tempo", "${nS(e)}×${e.lo}–${e.hi}${e.u===\"s\"?\" s\":\"\"}${e.side?\" /lado\":\"\"}${e.tempo")
rep("Array.from({length:e.sets}", "Array.from({length:nS(e)}")
rep("const allTop=done.length>=e.sets&&", "const allTop=done.length>=nS(e)&&")
rep("  if(allTop){ return e.u===\"kg\"?", "  if(allTop&&e.u===\"kg\"&&kg+1.25>capKg(exId)) return {kg,txt:\"Llegaste al tope de tus discos. Progresa así: baja en 4 s, pausa 2 s abajo y haz reps 1½. En la Fase 2 la app te pasa a la variante más difícil.\"};\n  if(allTop){ return e.u===\"kg\"?")
rep("const rows=k=>SESSIONS[k].ex.map(", "const rows=k=>sessEx(k).map(")
rep("  let h=contractCard();", """  let h=contractCard();
  const ph=phase(), td=testDue();
  if(ph.deload) h+=`<div class="alert info"><strong>${esc(ph.label)}</strong></div>`;
  else if(weeksIn()===9&&new Date().getDay()===1) h+=`<div class="alert ok"><strong>¡Fase 2 desbloqueada!</strong><span class="small">Algunos ejercicios cambian a versiones más difíciles (1½, una pierna, búlgara). Empieza liviano.</span></div>`;
  if(td) h+=`<div class="alert info"><strong>${td==="base"?"Haz tu prueba de fuerza inicial":"Toca tu prueba de fuerza mensual"}</strong><span class="small">10 minutos, en la pestaña Progreso. Es tu punto de comparación.</span><div class="row"><button class="btn" data-tab="progreso">Ir a la prueba</button></div></div>`;""")
rep("<div>${items.map(([k,l])", "${healthBlock(t)}${proteinBlock(t)}<div>${items.map(([k,l])")

# ---------- pestaña Progreso nueva ----------
cut("function viewProgreso(){", "function weightChart(", r'''function viewProgreso(){
  const ws=Object.keys(S.weights).sort();
  const done=Object.values(S.logs).filter(l=>l.done).length;
  const pts=ws.map(k=>({d:k,kg:+S.weights[k].kg}));
  const prs=Object.keys(EX).filter(id=>!id.endsWith("_min")).map(id=>{let best=null;for(const d of Object.keys(S.logs)){const r=(S.logs[d].sets||{})[id];if(!r)continue;for(const s of r){if(!s||!s.ok)continue;const sc=(+s.kg||0)*100+(+s.r||0);if(!best||sc>best.sc)best={sc,kg:+s.kg||0,r:+s.r||0,d}}}return best?{id,...best}:null}).filter(Boolean);
  return `<div class="stack" style="gap:16px">
  <section class="card"><span class="label">Registrar peso (mañana, en ayunas, después del baño)</span>
    <div class="row"><input type="date" id="wDate" value="${today()}" style="max-width:180px"><input type="number" step="0.1" inputmode="decimal" id="wIn2" placeholder="kg" style="max-width:120px"><button class="btn primary" id="wSave2">Guardar</button></div>
    ${weightChart(pts)}
    ${ws.length?`<div class="tbl-wrap"><table><thead><tr><th>Fecha</th><th>Peso</th><th></th></tr></thead><tbody>${ws.slice().reverse().slice(0,8).map(k=>`<tr><td>${fmtDate(k)}</td><td class="num mono">${S.weights[k].kg} kg</td><td><button class="btn ghost small" data-delw="${k}">Borrar</button></td></tr>`).join("")}</tbody></table></div>`:`<p class="small muted">Pésate 1–3 veces por semana a la misma hora; mira la tendencia, no el número de un día.</p>`}
  </section>
  <section class="card"><span class="label">Totales</span>
    <div class="grid2"><div class="stat"><span class="label">Sesiones hechas</span><b class="num">${done}</b></div><div class="stat"><span class="label">Mejor racha</span><b class="num">${Math.max(S.profile.bestStreak||0,streak())}</b></div><div class="stat"><span class="label">Semanas en el plan</span><b class="num">${weeksIn()}</b></div><div class="stat"><span class="label">Fase</span><b class="num">${phase().p}</b><span class="small muted">${esc(phase().label)}</span></div></div></section>
  ${medalsCard()}
  ${testCard()}
  ${photoCard()}
  <section class="card"><span class="label">Tus mejores marcas</span>
    ${prs.length?`<div class="tbl-wrap"><table><thead><tr><th>Ejercicio</th><th>Mejor serie</th><th>Fecha</th></tr></thead><tbody>${prs.map(p=>`<tr><td>${esc(EX[p.id].n)}</td><td class="num mono">${p.r}${EX[p.id].u==="s"?" s":" reps"}${p.kg?" × "+p.kg+" kg":""}</td><td>${fmtDate(p.d)}</td></tr>`).join("")}</tbody></table></div>`:`<p class="small muted">Tus récords aparecerán aquí después de tu primera sesión.</p>`}
  </section>
  ${partnerCard()}
  <section class="card"><span class="label">Historial</span>
    ${Object.keys(S.logs).length?`<div class="tbl-wrap"><table><thead><tr><th>Fecha</th><th>Sesión</th><th>Lumbar</th><th>Esfuerzo</th></tr></thead><tbody>${Object.keys(S.logs).sort().reverse().slice(0,20).map(d=>{const l=S.logs[d];return `<tr><td>${fmtDate(d)}</td><td>${l.skipped?'<span class="tag bad">Fallada</span>':l.done?esc(SESSIONS[l.type]?.name||l.type):'<span class="tag">A medias</span>'}</td><td class="num mono">${l.pain??"–"}</td><td class="num mono">${l.rpe??"–"}</td></tr>`}).join("")}</tbody></table></div>`:`<p class="small muted">Sin sesiones todavía. La primera es la más importante.</p>`}
  </section>
  ${backupCard()}</div>`;
}
''')

# ---------- quitar coach con Claude y escritorio ----------
cut("/* ============ COACH (Claude) ============ */", "/* ============ EVENTOS", "")
rep("  renderDeskControls();\n}", "  bindExtras(P);\n}")
cut("/* ============ APP DE ESCRITORIO", "</script>", r'''function updateDesktop(){}
/* ============ ARRANQUE ============ */
try{const t=localStorage.getItem("forja.tab");if(t&&TABS.some(x=>x[0]===t))TAB=t}catch(e){}
initStore();
if("serviceWorker" in navigator){navigator.serviceWorker.register("sw.js").then(()=>{if(TAB==="plan")renderPushControls()}).catch(()=>{})}
document.addEventListener("visibilitychange",()=>{if(!document.hidden){syncSW();if(!ACTIVE)render(true)}});
setInterval(()=>{checkOpenReminders(); if(!ACTIVE && !(document.activeElement&&document.activeElement.matches("input,textarea"))) render(true); },60000);
''')
s = s.replace("</script>\n", "</script>\n</body>\n</html>\n") if s.rstrip().endswith("</script>") else s + "\n</body>\n</html>\n"

# ---------- Plan: recordatorios y ajustes ----------
s = re.sub(r'<section class="card"><span class="label">Recordatorios</span>.*?<div id="desk" class="row"></div></section>',
 '''<section class="card"><span class="label">Notificaciones</span><h3>Avisos automáticos</h3>
  <ul class="clean small">
    <li><b>6:45 todos los días:</b> qué toca hoy. Si fallaste la sesión anterior, te lo recuerda.</li>
    <li><b>20:00 lunes, martes, jueves y viernes:</b> si no marcaste la sesión, te insiste con la versión mínima. Si la hiciste, te felicita.</li>
    <li><b>Domingo 10:00:</b> pesarte, revisar tu semana en Coach y enviar el resumen a tu compañero.</li>
  </ul>
  <div id="push" class="stack"></div></section>''', s, flags=re.S)
rep('''<label class="stack small"><span class="label">Hora de entrenamiento</span><input type="text" id="pT" value="${esc(S.profile.trainTime)}" placeholder="07:00"></label>''',
'''<label class="stack small"><span class="label">Hora de entrenamiento</span><input type="text" id="pT" value="${esc(S.profile.trainTime)}" placeholder="07:00"></label>
      <label class="stack small"><span class="label">Máx. por mancuerna (par, kg)</span><input type="number" step="0.25" id="pCapP" value="${S.profile.capPair||9.25}"></label>
      <label class="stack small"><span class="label">Máx. una mancuerna con todos los discos (kg)</span><input type="number" step="0.25" id="pCap1" value="${S.profile.capOne||16.5}"></label>''')
rep('trainTime:t});toast("Ajustes guardados.")', 'trainTime:t,capPair:+$("#pCapP").value||9.25,capOne:+$("#pCap1").value||16.5});toast("Ajustes guardados.")')
rep('<span class="small muted" id="syncState">Cargando tus datos…</span>', '<span class="small muted" id="syncState">Cargando…</span>')

assert "claude.use" not in s, "quedó una llamada a claude.use"
(ROOT/"index.html").write_text(s)
print("index.html", len(s), "bytes")
