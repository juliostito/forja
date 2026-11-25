/* ============ FASES Y PROGRESIÓN AUTOMÁTICA ============ */
Object.assign(EX,{
  boxsq2:{n:"Sentadilla goblet 1½ a la banca",lo:8,hi:12,sets:3,u:"kg",start:12,tempo:"3-1-1",cue:"Baja a la banca, sube hasta la mitad, vuelve a bajar y recién ahí sube completo: eso es 1 rep.",back:"Pecho alto todo el tiempo.",prog:"3×12 → +1,25 kg hasta el tope; luego pausa de 3 s abajo."},
  thrust1:{n:"Hip thrust a una pierna (hombros en la banca)",lo:8,hi:12,sets:3,u:"kg",side:true,start:4,cue:"Una pierna en el aire, empuja con el talón de apoyo, pausa 2 s arriba.",back:"Costillas abajo: el glúteo hace el trabajo, no la lumbar.",prog:"3×12 → +1,25 kg."},
  press2:{n:"Press en banca 1½ con pausa",lo:8,hi:12,sets:3,u:"kg",start:8,tempo:"3-2-1",cue:"Baja, pausa 2 s, sube a la mitad, baja otra vez y sube completo.",back:"Pies firmes, glúteos en la banca.",prog:"3×12 → +1,25 kg."},
  ohp1:{n:"Press de hombro a un brazo sentado",lo:8,hi:12,sets:3,u:"kg",side:true,start:7,cue:"Espalda contra la pared, la mano libre en el abdomen para sentir que no arqueas.",back:"Costillas abajo; si arqueas, baja el peso.",prog:"3×12 → +1,25 kg."},
  csrow2:{n:"Remo pecho apoyado con pausa de 3 s",lo:8,hi:12,sets:3,u:"kg",start:8,cue:"Tira y sostén 3 s arriba apretando las escápulas.",back:"Pecho apoyado = espalda segura.",prog:"3×12 → +1,25 kg."},
  rdl2:{n:"Peso muerto rumano tempo 4 s",lo:8,hi:12,sets:3,u:"kg",start:9,tempo:"4-1-1",cue:"Baja en 4 s contando, siente el estiramiento atrás del muslo.",back:"Columna neutra; si molesta, cambia ese día por hip thrust.",prog:"3×12 → +1,25 kg."},
  split2:{n:"Sentadilla búlgara (pie de atrás en la banca)",lo:6,hi:10,sets:3,u:"kg",side:true,start:3,cue:"Empeine de atrás sobre la banca, baja vertical. Mano en la pared si hace falta.",back:"Tronco erguido; empieza sin peso si la estabilidad falla.",prog:"3×10 → +1,25 kg."}
});
const PHASE2={boxsq:"boxsq2",thrust:"thrust1",press:"press2",ohp:"ohp1",csrow:"csrow2",rdl:"rdl2",split:"split2"};
const ONE_DB=new Set(["boxsq","boxsq2","thrust","thrust1","pullover","skull","goblet_min"]);
function capKg(id){return ONE_DB.has(id)? +(S.profile.capOne||16.5) : +(S.profile.capPair||9.25)}
function phase(){
  const w=weeksIn();
  if(w<8) return {p:1,deload:false,label:`Fase 1 · Base · semana ${w+1} de 8`};
  const k=(w-8)%9;
  if(k===0) return {p:w<9?1:2,deload:true,label:"Semana de descarga: mitad de series, misma técnica. Tu cuerpo se recupera y vuelve más fuerte."};
  return {p:2,deload:false,label:`Fase 2 · Intensidad · semana ${k} de 8`};
}
function sessEx(type){ if(type==="MIN") return SESSIONS.MIN.ex; const ph=phase(); return SESSIONS[type].ex.map(id=>ph.p===2&&PHASE2[id]?PHASE2[id]:id); }
function nS(e){return phase().deload?Math.max(1,Math.ceil(e.sets/2)):e.sets}

/* ============ PROTEÍNA RÁPIDA ============ */
const FOODS=[["Huevo",6],["Pechuga 100 g",31],["Carne magra 100 g",26],["Lata de atún",25],["½ lata de jurel",22],["Yogur proteico",15],["Scoop de proteína",24],["Quesillo 100 g",13],["Vaso de leche",8],["Taza de legumbres",15]];
function proteinBlock(t){
  const tg=targets(),d=S.days[t]||{},pg=d.pg||0,pct=Math.min(100,Math.round(pg/tg.prot*100));
  return `<div class="prot"><div class="row" style="justify-content:space-between"><span><b class="num">${pg}</b> <span class="muted">/ ${tg.prot} g de proteína hoy</span></span><span class="tag ${pg>=tg.prot?"ok":""} num">${pct}%</span></div>
  <div class="bar" role="progressbar" aria-valuenow="${pct}" aria-valuemin="0" aria-valuemax="100"><i style="width:${pct}%"></i></div>
  <div class="chips">${FOODS.map((f,i)=>`<button class="chip" data-food="${i}">+ ${esc(f[0])} <span class="muted">${f[1]} g</span></button>`).join("")}${(d.pl||[]).length?`<button class="chip" data-food="undo">↶ Deshacer</button>`:""}</div></div>`;
}

/* ============ PRUEBA DE FUERZA MENSUAL ============ */
const PU_LVLS=["Pared","Banca","Escalón","Rodillas en el suelo","Suelo"];
function testDue(){const ks=Object.keys(S.tests).sort();if(!ks.length)return "base";const days=(parse(today())-parse(ks[ks.length-1]))/864e5;return days>=28?"due":null}
function testCard(){
  const ks=Object.keys(S.tests).sort().reverse(), due=testDue();
  return `<section class="card" id="testCard"><span class="label">Prueba de fuerza · cada 4 semanas · 10 min</span><h3>${due==="base"?"Haz tu prueba inicial":due==="due"?"Toca tu prueba de este mes":"Próxima prueba en "+Math.ceil(28-(parse(today())-parse(ks[0]))/864e5)+" días"}</h3>
  <p class="small muted">Con calentamiento previo. Cada prueba al máximo con buena técnica; para si duele la espalda. La báscula puede estancarse; estas marcas casi nunca mienten.</p>
  <div class="grid2">
    <label class="stack small"><span class="label">Flexiones: nivel</span><select id="tLvl">${PU_LVLS.map((l,i)=>`<option value="${i}" ${i===(ks.length?S.tests[ks[0]].lvl:1)?"selected":""}>${l}</option>`).join("")}</select></label>
    <label class="stack small"><span class="label">Flexiones: reps máximas</span><input type="number" inputmode="numeric" id="tPu"></label>
    <label class="stack small"><span class="label">Sentadillas a la banca en 60 s</span><input type="number" inputmode="numeric" id="tSq"></label>
    <label class="stack small"><span class="label">Plancha lateral (s), peor lado</span><input type="number" inputmode="numeric" id="tSp"></label>
    <label class="stack small"><span class="label">Cintura (cm, a la altura del ombligo)</span><input type="number" step="0.5" inputmode="decimal" id="tWa"></label>
    <label class="stack small"><span class="label">Peso (kg)</span><input type="number" step="0.1" inputmode="decimal" id="tKg" value="${latestWeight()}"></label>
  </div>
  <button class="btn primary" id="tSave">Guardar prueba de hoy</button>
  ${ks.length?`<div class="tbl-wrap"><table><thead><tr><th>Fecha</th><th>Flexiones</th><th>Sent. 60 s</th><th>Plancha</th><th>Cintura</th></tr></thead><tbody>${ks.map(k=>{const r=S.tests[k];return `<tr><td>${fmtDate(k)}</td><td class="num mono">${r.pu??"–"} <span class="muted small">${esc(PU_LVLS[r.lvl]||"")}</span></td><td class="num mono">${r.sq??"–"}</td><td class="num mono">${r.sp??"–"} s</td><td class="num mono">${r.wa??"–"}</td></tr>`}).join("")}</tbody></table></div>`:""}
  </section>`;
}

/* ============ FOTOS DE PROGRESO (IndexedDB, solo en este teléfono) ============ */
const IDB={_p:null,open(){return this._p||(this._p=new Promise((res,rej)=>{const r=indexedDB.open("forja",1);r.onupgradeneeded=()=>r.result.createObjectStore("photos");r.onsuccess=()=>res(r.result);r.onerror=()=>rej(r.error)}))},
  async put(k,v){const d=await this.open();return new Promise((res,rej)=>{const tx=d.transaction("photos","readwrite");tx.objectStore("photos").put(v,k);tx.oncomplete=()=>res();tx.onerror=()=>rej(tx.error)})},
  async get(k){const d=await this.open();return new Promise((res,rej)=>{const r=d.transaction("photos").objectStore("photos").get(k);r.onsuccess=()=>res(r.result);r.onerror=()=>rej(r.error)})},
  async del(k){const d=await this.open();return new Promise(res=>{const tx=d.transaction("photos","readwrite");tx.objectStore("photos").delete(k);tx.oncomplete=()=>res()})}};
async function shrink(file){
  const bmp=await createImageBitmap(file);const m=1080/Math.max(bmp.width,bmp.height),sc=Math.min(1,m);
  const c=document.createElement("canvas");c.width=Math.round(bmp.width*sc);c.height=Math.round(bmp.height*sc);c.getContext("2d").drawImage(bmp,0,0,c.width,c.height);
  return new Promise(r=>c.toBlob(r,"image/jpeg",0.82));
}
function photoCard(){
  const ks=Object.keys(S.photos).sort().reverse();
  return `<section class="card"><span class="label">Fotos de progreso · 1 al mes · privadas en este teléfono</span><h3>Tu cambio, en imágenes</h3>
  <p class="small muted">Misma luz, misma ropa, misma pose (frente y perfil), por la mañana. No se suben a ningún lado.</p>
  <label class="btn primary" style="text-align:center">+ Agregar foto<input type="file" accept="image/*" id="phIn" hidden></label>
  ${ks.length?`<div class="photos">${ks.map(k=>`<figure><img data-pid="${esc(k)}" alt="Foto del ${fmtDate(S.photos[k].date)}"><figcaption class="small"><span>${fmtDate(S.photos[k].date)}</span><button class="btn ghost small" data-delph="${esc(k)}">Borrar</button></figcaption></figure>`).join("")}</div>`:`<p class="small muted">Aún no hay fotos. La del día 1 es la que más vas a agradecer en 6 meses.</p>`}
  </section>`;
}
const PH_URLS=[];
async function loadPhotos(root){
  PH_URLS.splice(0).forEach(u=>URL.revokeObjectURL(u));
  for(const img of root.querySelectorAll("img[data-pid]")){try{const b=await IDB.get(img.dataset.pid);if(b){const u=URL.createObjectURL(b);PH_URLS.push(u);img.src=u}}catch(e){}}
}

/* ============ MEDALLAS ============ */
function perfectWeeks(){const res=[];let ws=weekStart(parse(startDate()));const now=weekStart();let g=0;while(ws<=now&&g++<600){let n=0;for(let i=0;i<7;i++){const s=iso(addDays(ws,i));if(isTrainDay(s)&&doneOn(s))n++}res.push(n>=4);ws=addDays(ws,7)}return res}
function medals(){
  const done=Object.values(S.logs).filter(l=>l.done).length, pw=perfectWeeks();let run=0,maxRun=0;pw.forEach(b=>{run=b?run+1:0;maxRun=Math.max(maxRun,run)});
  const best=Math.max(S.profile.bestStreak||0,streak()), lost=S.profile.startWeight-latestWeight();
  const lvl=Object.keys(EX).some(id=>{let prev=null;for(const d of Object.keys(S.logs).sort()){const r=(S.logs[d].sets||{})[id];if(!r)continue;const m=Math.max(0,...r.filter(s=>s&&s.ok).map(s=>+s.kg||0));if(!m)continue;if(prev!==null&&m>prev)return true;prev=Math.max(prev||0,m)}return false});
  const fullHab=Object.values(S.days).filter(d=>d.h&&Object.values(d.h).filter(Boolean).length>=7).length;
  return [
    ["first","Primera sesión","El paso más difícil ya está",done>=1],
    ["s10","10 sesiones","Ya no es casualidad",done>=10],
    ["s25","25 sesiones","Hábito en construcción",done>=25],
    ["s50","50 sesiones","Esto ya es parte de ti",done>=50],
    ["s100","100 sesiones","Nivel leyenda",done>=100],
    ["w4","Semana perfecta","4 de 4 sesiones",pw.some(Boolean)],
    ["m4","Mes perfecto","4 semanas perfectas seguidas",maxRun>=4],
    ["st8","Racha de 8","8 sesiones sin fallar",best>=8],
    ["st24","Racha de 24","6 semanas sin fallar",best>=24],
    ["lvl","Primera subida de carga","Más kilos que la vez anterior",lvl],
    ["test","Primera prueba de fuerza","Ya tienes tu línea base",Object.keys(S.tests).length>=1],
    ["hab7","Hábitos al 100%","7 días con todo marcado",fullHab>=7],
    ["photo","Primera foto","Tu yo del día 1 te lo agradecerá",Object.keys(S.photos).length>=1],
    ["kg5","−5 kg","Primera meta de peso",lost>=5],
    ["kg10","−10 kg","Otra persona",lost>=10],
    ["kg15","−15 kg","Transformación",lost>=15],
    ["ph2","Fase 2 desbloqueada","Ejercicios más difíciles",weeksIn()>=9]
  ];
}
function checkMedals(announce){
  const st=streak(); if(st>(S.profile.bestStreak||0)) S.profile.bestStreak=st;
  const seen=new Set(S.profile.medals||[]); const fresh=medals().filter(m=>m[3]&&!seen.has(m[0]));
  if(!fresh.length) return;
  S.profile.medals=[...seen,...fresh.map(m=>m[0])];
  if(announce) setTimeout(()=>toast("🏅 Nueva medalla: "+fresh.map(m=>m[1]).join(", ")),400);
}
function medalsCard(){
  const ms=medals(),n=ms.filter(m=>m[3]).length;
  return `<section class="card"><div class="sess-head"><div><span class="label">Medallas</span><h3>${n} de ${ms.length}</h3></div></div>
  <div class="medals">${ms.map(m=>`<div class="medal ${m[3]?"on":""}"><span class="m-ico" aria-hidden="true">${m[3]?"★":"☆"}</span><b>${esc(m[1])}</b><span class="small muted">${esc(m[2])}</span></div>`).join("")}</div></section>`;
}

/* ============ COMPAÑERO DE COMPROMISO ============ */
function summaryText(){
  const lw=latestWeight(),diff=+(lw-S.profile.startWeight).toFixed(1);
  const last=Object.keys(S.logs).filter(d=>S.logs[d].done).sort().pop();
  return `Forja 4× · semana ${weeksIn()+1}\nRacha: ${streak()} sesiones seguidas\nEsta semana: ${weekCount()}/4 sesiones\nÚltima sesión: ${last?fmtDate(last):"ninguna aún"}\nPeso: ${String(lw).replace(".",",")} kg (${diff>0?"+":""}${String(diff).replace(".",",")} kg)\nSi el domingo no te mando esto, pregúntame por qué 💪`;
}
function partnerCard(){
  return `<section class="card"><span class="label">Compañero de compromiso</span><h3>Alguien que te pida cuentas</h3>
  <p class="small muted">Elige a una persona. Cada domingo le mandas tu resumen; si no llega, te pregunta. Saber que alguien mira aumenta mucho la constancia.</p>
  <label class="stack small"><span class="label">Nombre de tu compañero</span><input type="text" id="pName" value="${esc(S.profile.partner||"")}" placeholder="Ej: mi hermano, mi pareja, un amigo"></label>
  <pre class="sum">${esc(summaryText())}</pre>
  <div class="row"><button class="btn primary" id="shareSum">Enviar resumen</button><button class="btn" id="copySum">Copiar</button></div></section>`;
}

/* ============ COACH SIN CONEXIÓN ============ */
function weeklyReview(){
  const end=new Date(),days=[...Array(7)].map((_,i)=>iso(addDays(end,-i))).reverse();
  const planned=days.filter(isTrainDay).filter(d=>d>=startDate()&&d<=today()), done=planned.filter(doneOn).length;
  const missed=planned.filter(d=>!doneOn(d)&&d<today());
  const reasons=missed.map(d=>S.logs[d]&&S.logs[d].reason).filter(Boolean);
  const pains=days.map(d=>S.logs[d]&&S.logs[d].pain).filter(v=>v!=null), avgPain=pains.length?(pains.reduce((a,b)=>a+b,0)/pains.length):null;
  const hab=days.map(d=>{const h=(S.days[d]||{}).h||{};return Object.values(h).filter(Boolean).length}), habPct=Math.round(hab.reduce((a,b)=>a+b,0)/(7*7)*100);
  const protDays=days.filter(d=>(S.days[d]||{}).pg>=targets().prot).length;
  const ws=Object.keys(S.weights).sort(), avg=(from,to)=>{const v=ws.filter(k=>k>=from&&k<=to).map(k=>+S.weights[k].kg);return v.length?v.reduce((a,b)=>a+b,0)/v.length:null};
  const a1=avg(iso(addDays(end,-6)),today()),a0=avg(iso(addDays(end,-13)),iso(addDays(end,-7)));
  const lines=[];
  lines.push(`Sesiones: ${done} de ${planned.length} planificadas hasta hoy.`+(missed.length?` Faltaste ${missed.length}${reasons.length?" (motivos: "+reasons.map(r=>"“"+r+"”").join(", ")+")":""}.`:" Ninguna falta. Así se hace."));
  if(avgPain!=null) lines.push(`Dolor lumbar promedio: ${avgPain.toFixed(1)}/10.`+(avgPain>3?" Está alto: cambia los ejercicios que molestan (ver “Me molesta la espalda”) y consulta si no baja.":" Bajo control."));
  const st=days.map(d=>((S.days[d]||{}).health||{}).steps).filter(v=>v!=null);
  if(st.length) lines.push(`Pasos (Apple Salud): promedio ${Math.round(st.reduce((a,b)=>a+b,0)/st.length).toLocaleString("es-CL")} al día en ${st.length} días registrados; meta ${stepsGoal().toLocaleString("es-CL")}.`);
  lines.push(`Hábitos diarios: ${habPct}% cumplidos. Proteína completa ${protDays} de 7 días.`);
  if(a1!=null&&a0!=null){const dlt=a1-a0;lines.push(`Peso promedio: ${a1.toFixed(1)} kg (${dlt>0?"+":""}${dlt.toFixed(1)} kg vs semana anterior).`+(dlt>-0.2?" Si se repite otra semana, quita ~150 kcal al día.":" Buen ritmo."))}
  else lines.push("Pésate al menos 2 veces por semana para ver la tendencia.");
  let focus;
  if(planned.length&&done<planned.length) focus="Tu foco: no fallar dos veces. Deja la ropa y las mancuernas listas la noche anterior y, si el día se complica, haz la versión mínima de 15 min.";
  else if(protDays<4) focus=`Tu foco: proteína. Usa los botones de “Hoy” y apunta a ${targets().prot} g; agrega huevos al desayuno y una lata de atún o jurel en la cena.`;
  else if(habPct<60) focus=`Tu foco: pasos. Meta ${stepsGoal().toLocaleString("es-CL")} al día: una caminata de 20 min después de almuerzo te deja casi allí.`;
  else focus="Tu foco: sube un poco la exigencia. Termina cada serie a 1 rep del fallo y respeta los descansos de 90 s.";
  lines.push(focus);
  return lines;
}
const TOPICS=[
  ["Hoy no tengo ganas",()=>`Pacto de 2 minutos: ponte las zapatillas y haz solo el calentamiento. Si después de eso quieres parar, para. Casi nunca pasa.\n\nSi el día está difícil, haz la versión mínima de 15 min: cuenta para la racha.\n\nTu porqué: “${S.profile.why||"escríbelo en el contrato"}”.\n\nNo entrenas porque tengas ganas. Entrenas porque eres alguien que entrena. Las ganas llegan a los 5 minutos.`],
  ["Me molesta la espalda",()=>`Semáforo: 0–3/10 que no empeora al día siguiente → sigue. Más de 3 → cambia el ejercicio hoy:\n• Peso muerto rumano → Hip thrust o puente de glúteo.\n• Sentadilla split / búlgara → Sentadilla isométrica en la pared.\n• Sentadilla goblet → misma, pero a una banca más alta (cojín encima) y sin peso.\n• Press de hombro → Floor press.\n• Remo a una mano → Remo con pecho apoyado.\nHaz el Big 3 de McGill todos los días y camina 20–30 min.\nConsulta a un médico si el dolor baja por la pierna, hay hormigueo, debilidad, o no mejora en 2 semanas.`],
  ["No bajo de peso",()=>`1) Mira el promedio de 2 semanas, no un día (el agua mueve 1–2 kg).\n2) ¿Llegas a ${targets().prot} g de proteína y ${stepsGoal().toLocaleString("es-CL")} pasos?\n3) Calorías líquidas: bebidas, jugos, alcohol, café con azúcar.\n4) Fines de semana: suelen borrar el déficit de la semana.\n5) Si todo eso está bien y 2 semanas no baja: quita ~150 kcal al día (una marraqueta o una cucharada de aceite menos).\n6) Duerme 7+ h: dormir poco sube el hambre.`],
  ["Tengo hambre todo el día",()=>`• Proteína en cada comida (2 palmas).\n• Medio plato de verduras en almuerzo y cena: volumen con pocas calorías.\n• Agua: un vaso antes de cada comida.\n• Fibra: legumbres 2–3 veces por semana, avena, fruta entera (no jugo).\n• Come despacio, 15–20 min por plato.\n• Colaciones con proteína: yogur proteico, huevo duro, quesillo.`],
  ["Voy a comer fuera / carrete",()=>`• Elige primero la proteína (pollo, carne, pescado) y pide ensalada de acompañamiento.\n• Un solo carbohidrato: o pan, o papas, o postre.\n• Alcohol: máximo 2 tragos, intercala con agua, evita bebidas azucaradas como mezcla.\n• Al día siguiente no compenses dejando de comer: vuelve al plan normal y camina.\n• Un mal día no es un mal mes.`],
  ["Estoy de viaje / sin equipo",()=>`Circuito de 20 min, 4 vueltas, 45 s trabajo / 15 s pausa:\n1. Flexiones inclinadas (cama o mesa)\n2. Sentadilla a una silla\n3. Remo con mochila con libros\n4. Puente de glúteo\n5. Bird dog\nY camina todo lo que puedas: el viaje no es excusa para la racha.`],
  ["Me estanqué en un ejercicio",()=>`Con mancuernas livianas, el peso no es la única palanca:\n• Baja en 4 s y haz pausa de 2 s abajo.\n• Reps 1½ (baja, sube media, baja, sube completo).\n• Versión a una pierna o un brazo.\n• Acorta el descanso a 60 s.\n• Si llevas 3 sesiones sin mejorar nada, revisa el sueño y la comida; en la semana de descarga se recupera la fuerza.`],
  ["Me duele todo (agujetas)",()=>`Es normal las primeras 2–3 semanas y baja rápido.\n• Entrena igual, con menos series o menos peso: el movimiento las alivia.\n• Camina 20–30 min y toma agua.\n• Duerme bien y come tu proteína.\n• Agujetas ≠ lesión: un dolor agudo, localizado o en la articulación sí es para parar.`]
];
function viewCoach(){
  const ph=phase();
  return `<div class="stack" style="gap:16px">
  <section class="card"><span class="label">Revisión de los últimos 7 días</span><h2>Tu coach</h2>
  <span class="tag">${esc(ph.label)}</span>
  <ul class="clean small">${weeklyReview().map(l=>`<li>${esc(l)}</li>`).join("")}</ul></section>
  <section class="card"><span class="label">Ayuda rápida</span><h3>¿Qué te pasa hoy?</h3>
  ${TOPICS.map(([q,f])=>`<details class="topic"><summary>${esc(q)}</summary><p class="small" style="white-space:pre-line;margin-top:8px">${esc(f())}</p></details>`).join("")}
  </section></div>`;
}

/* ============ NOTIFICACIONES (Web Push, sin Claude) ============ */
const VAPID_PUBLIC="__VAPID_PUBLIC__";
const isStandalone=()=>window.navigator.standalone===true||matchMedia("(display-mode: standalone)").matches;
const isIOS=/iPhone|iPad|iPod/.test(navigator.userAgent);
function b64u(s){const p="=".repeat((4-s.length%4)%4);const b=atob((s+p).replace(/-/g,"+").replace(/_/g,"/"));return Uint8Array.from(b,c=>c.charCodeAt(0))}
async function renderPushControls(){
  const box=$("#push"); if(!box) return;
  if(isIOS&&!isStandalone()){box.innerHTML=`<div class="alert warn"><strong>Primero instala Forja.</strong><span class="small">En Safari: botón Compartir (cuadrado con flecha) → “Agregar a pantalla de inicio”. Luego abre Forja desde su ícono y vuelve aquí.</span></div>`;return}
  if(!("serviceWorker" in navigator)||!("PushManager" in window)){box.innerHTML=`<p class="small muted">Este navegador no admite notificaciones push.</p>`;return}
  const reg=await navigator.serviceWorker.ready; const sub=await reg.pushManager.getSubscription();
  const perm=Notification.permission;
  box.innerHTML = sub ? `<span class="tag ok">Notificaciones activas en este dispositivo</span>
    <p class="small">Si aún no lo hiciste, copia este código y pégalo en GitHub (paso 4 de la guía) para recibir los avisos de las 6:45, 20:00 y el domingo.</p>
    <textarea readonly id="subTxt" class="mono small">${esc(JSON.stringify(sub))}</textarea>
    <div class="row"><button class="btn primary" id="subCopy">Copiar código</button><button class="btn" id="pushTest">Probar notificación</button></div>`
    : perm==="denied" ? `<div class="alert bad"><strong>Notificaciones bloqueadas.</strong><span class="small">Actívalas en Ajustes del iPhone → Notificaciones → Forja.</span></div>`
    : `<button class="btn primary big" id="pushOn">Activar notificaciones</button>`;
  const on=$("#pushOn"); if(on) on.onclick=async()=>{try{const p=await Notification.requestPermission();if(p!=="granted"){toast("Sin permiso no puedo avisarte.");return renderPushControls()}
      await reg.pushManager.subscribe({userVisibleOnly:true,applicationServerKey:b64u(VAPID_PUBLIC)});toast("Activadas. Ahora copia el código.");}catch(e){toast("No se pudo activar: "+(e.message||e))}renderPushControls()};
  const cp=$("#subCopy"); if(cp) cp.onclick=()=>{const t=$("#subTxt");navigator.clipboard.writeText(t.value).then(()=>toast("Código copiado")).catch(()=>{t.select();toast("Selecciona y copia el texto")})};
  const ts=$("#pushTest"); if(ts) ts.onclick=()=>reg.showNotification("Forja",{body:"Así se verán tus avisos. 💪",icon:"icon-192.png",tag:"test"}).catch(e=>toast("Error: "+e.message));
}
/* Avisos mientras la app está abierta (sin servidor) */
const notifiedOpen={};
function checkOpenReminders(){
  if(!("Notification" in window)||Notification.permission!=="granted"||!navigator.serviceWorker) return;
  const d=new Date(),t=today(),type=SCHEDULE[d.getDay()],l=S.logs[t]; if(!type||(l&&(l.done||l.skipped))) return;
  const [H,M]=S.profile.trainTime.split(":").map(Number),mins=d.getHours()*60+d.getMinutes(),tm=H*60+M;
  if(mins>=tm&&mins<tm+60&&!notifiedOpen[t]){notifiedOpen[t]=1;navigator.serviceWorker.ready.then(r=>r.showNotification("Forja: ahora",{body:`${SESSIONS[type].name}. Solo empieza el calentamiento.`,icon:"icon-192.png",tag:"now"})).catch(()=>{})}
}

/* ============ APPLE SALUD (vía Atajos de iOS) ============ */
// El atajo "Forja Salud" copia un texto como: FORJA;pasos=8123;kcal=512;ejercicio=34;fcr=62;peso=103,2;sueno=7,1
function hnum(v,int){if(v==null)return null;let x=String(v).replace(/[^0-9.,]/g,"");if(!x)return null;
  if(x.includes(",")&&x.includes(".")) x=x.replace(/\./g,"").replace(",",".");
  else if(x.includes(",")) x=x.replace(",",".");
  else if(int&&/^\d{1,3}(\.\d{3})+$/.test(x)) x=x.replace(/\./g,"");
  const n=parseFloat(x);return isFinite(n)?(int?Math.round(n):Math.round(n*10)/10):null}
function parseHealth(txt){
  if(!/FORJA/i.test(txt||"")) return null;
  const kv={};String(txt).split(/[;\n|]/).forEach(p=>{const m=p.match(/^\s*([a-záéíóúñ]+)\s*[=:]\s*(.+)$/i);if(m)kv[m[1].toLowerCase()]=m[2]});
  const h={steps:hnum(kv.pasos,1),kcal:hnum(kv.kcal,1),exMin:hnum(kv.ejercicio,1),rhr:hnum(kv.fcr,1),weight:hnum(kv.peso),sleep:hnum(kv.sueno??kv["sueño"])};
  return Object.values(h).some(v=>v!=null)?h:null;
}
function applyHealth(h){
  const t=today(),d=JSON.parse(JSON.stringify(S.days[t]||{h:{}}));d.h=d.h||{};d.health={...(d.health||{}),...Object.fromEntries(Object.entries(h).filter(([,v])=>v!=null)),at:Date.now()};
  if(h.steps!=null&&h.steps>=stepsGoal()) d.h.steps=true;
  if(h.sleep!=null&&h.sleep>=7) d.h.sleep=true;
  if(h.weight&&h.weight>30&&h.weight<300) S.weights[t]={kg:h.weight,ts:Date.now(),src:"salud"};
  put("days",t,d); toast("Datos de Salud importados.");
}
function healthBlock(t){
  const hd=(S.days[t]||{}).health;
  const cell=(l,v,u)=>`<div class="hs"><span class="label">${l}</span><b class="num">${v==null?"–":v.toLocaleString("es-CL")}</b><span class="small muted">${u}</span></div>`;
  return `<div class="health"><div class="row" style="justify-content:space-between"><span class="label">Apple Salud · hoy</span>${hd?`<span class="small muted">actualizado ${new Date(hd.at).toLocaleTimeString("es-CL",{hour:"2-digit",minute:"2-digit"})}</span>`:""}</div>
  ${hd?`<div class="hgrid">${cell("Pasos",hd.steps,"meta "+stepsGoal().toLocaleString("es-CL"))}${cell("Energía activa",hd.kcal,"kcal")}${cell("Ejercicio",hd.exMin,"min")}${cell("FC reposo",hd.rhr,"lpm")}${cell("Sueño",hd.sleep,"h")}</div>`:`<p class="small muted">Ejecuta tu atajo “Forja Salud” y luego toca el botón para traer pasos, calorías, minutos de ejercicio, pulso en reposo, sueño y peso.</p>`}
  <div class="row"><button class="btn" id="hPaste">Pegar datos de Salud</button></div>
  <div id="hManual" hidden class="stack"><textarea id="hTxt" placeholder="Pega aquí el texto que copió el atajo (empieza con FORJA)"></textarea><button class="btn primary" id="hApply">Importar</button></div></div>`;
}

/* ============ COPIA DE SEGURIDAD ============ */
function backupCard(){
  return `<section class="card"><span class="label">Copia de seguridad</span><h3>Tus datos viven en este teléfono</h3>
  <p class="small muted">Exporta una copia una vez al mes y guárdala en iCloud Drive o envíatela por correo. Incluye registros, peso, pruebas y hábitos (las fotos no).</p>
  <div class="row"><button class="btn" id="bkOut">Exportar copia</button><label class="btn">Importar copia<input type="file" accept="application/json,.json" id="bkIn" hidden></label></div>
  <div id="bkConfirm" hidden class="alert warn"><strong>Esto reemplaza todos tus datos actuales.</strong><div class="row"><button class="btn danger" id="bkYes">Sí, reemplazar</button><button class="btn" id="bkNo">Cancelar</button></div></div></section>`;
}
let pendingImport=null;
function bindExtras(P){
  P.querySelectorAll("[data-food]").forEach(b=>b.onclick=()=>{const t=today(),tg=targets();const d=JSON.parse(JSON.stringify(S.days[t]||{h:{}}));d.h=d.h||{};d.pl=d.pl||[];
    if(b.dataset.food==="undo")d.pl.pop();else d.pl.push(FOODS[+b.dataset.food][1]);d.pg=d.pl.reduce((a,c)=>a+c,0);if(d.pg>=tg.prot)d.h.prot=true;put("days",t,d)});
  const ts=$("#tSave",P); if(ts) ts.onclick=()=>{const v=id=>{const x=$(id).value;return x===""?null:+String(x).replace(",",".")};const r={lvl:+$("#tLvl").value,pu:v("#tPu"),sq:v("#tSq"),sp:v("#tSp"),wa:v("#tWa"),kg:v("#tKg")};
    if(r.pu==null&&r.sq==null&&r.sp==null){toast("Completa al menos una prueba.");return} if(r.kg)S.weights[today()]={kg:r.kg,ts:Date.now()}; put("tests",today(),r); toast("Prueba guardada. Nos vemos en 4 semanas.")};
  const ph=$("#phIn",P); if(ph) ph.onchange=async()=>{const f=ph.files[0];if(!f)return;try{const blob=await shrink(f);const k=today()+"-"+Date.now();await IDB.put(k,blob);put("photos",k,{date:today()});toast("Foto guardada.")}catch(e){toast("No se pudo guardar la foto.")}};
  P.querySelectorAll("[data-delph]").forEach(b=>b.onclick=async()=>{if(!b.dataset.armed){b.dataset.armed=1;b.textContent="¿Seguro?";return}await IDB.del(b.dataset.delph).catch(()=>{});del("photos",b.dataset.delph)});
  if(P.querySelector("img[data-pid]")) loadPhotos(P);
  const pn=$("#pName",P); if(pn) pn.onchange=()=>put("app","profile",{partner:pn.value.trim()});
  const sh=$("#shareSum",P); if(sh) sh.onclick=async()=>{const text=summaryText();try{if(navigator.share){await navigator.share({text});return}}catch(e){if(e.name==="AbortError")return}navigator.clipboard.writeText(text).then(()=>toast("Copiado: pégalo en WhatsApp")).catch(()=>toast("No se pudo copiar"))};
  const cs=$("#copySum",P); if(cs) cs.onclick=()=>navigator.clipboard.writeText(summaryText()).then(()=>toast("Copiado")).catch(()=>toast("No se pudo copiar"));
  const bo=$("#bkOut",P); if(bo) bo.onclick=()=>{const blob=new Blob([JSON.stringify({app:"forja4x",v:1,exported:new Date().toISOString(),data:S},null,1)],{type:"application/json"});const a=document.createElement("a");a.href=URL.createObjectURL(blob);a.download=`forja-copia-${today()}.json`;document.body.appendChild(a);a.click();setTimeout(()=>{URL.revokeObjectURL(a.href);a.remove()},1000)};
  const bi=$("#bkIn",P); if(bi) bi.onchange=async()=>{try{const j=JSON.parse(await bi.files[0].text());if(j.app!=="forja4x"||!j.data)throw 0;pendingImport=j.data;$("#bkConfirm").hidden=false}catch(e){toast("Ese archivo no es una copia de Forja.")}};
  const by=$("#bkYes",P); if(by) by.onclick=()=>{for(const k of Object.keys(S)) S[k]=k==="profile"?{...PROFILE_DEFAULT,...(pendingImport.profile||{})}:(pendingImport[k]||{});pendingImport=null;lsSave();render(true);toast("Copia restaurada.")};
  const bn=$("#bkNo",P); if(bn) bn.onclick=()=>{pendingImport=null;$("#bkConfirm").hidden=true};
  const cap=$("#pCap",P); if(cap) cap.onchange=()=>{};
  const hp=$("#hPaste",P); if(hp) hp.onclick=async()=>{let txt="";try{txt=await navigator.clipboard.readText()}catch(e){}
    const h=parseHealth(txt); if(h) return applyHealth(h); $("#hManual").hidden=false; toast(txt?"El portapapeles no tiene datos de Forja: ejecuta el atajo primero.":"Pega el texto del atajo aquí abajo.")};
  const ha=$("#hApply",P); if(ha) ha.onclick=()=>{const h=parseHealth($("#hTxt").value);if(!h){toast("Ese texto no viene del atajo Forja Salud.");return}applyHealth(h)};
  renderPushControls();
}
