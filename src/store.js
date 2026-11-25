/* ============ ALMACENAMIENTO LOCAL (este dispositivo) ============ */
const S = { profile:{...PROFILE_DEFAULT}, logs:{}, days:{}, weights:{}, tests:{}, photos:{} };
const LSK = "forja4x.v1";
function lsLoad(){try{const j=JSON.parse(localStorage.getItem(LSK)||"null");if(j){for(const k of Object.keys(S)) if(j[k]&&k!=="profile") S[k]=j[k]; S.profile={...PROFILE_DEFAULT,...(j.profile||{})}}}catch(e){}}
function lsSave(){try{localStorage.setItem(LSK,JSON.stringify(S))}catch(e){toast("No se pudo guardar: el almacenamiento está lleno.")} syncSW();}
function put(col,id,data,quiet){
  if(col==="app") S.profile={...S.profile,...data}; else S[col][id]=data;
  checkMedals(true); lsSave(); if(quiet) renderTop(); else render(false);
}
function del(col,id){delete S[col][id];lsSave();render(false)}
function initStore(){
  lsLoad();
  if(!S.profile.startDate){S.profile.startDate=today();}
  checkMedals(false); lsSave();
  try{navigator.storage&&navigator.storage.persist&&navigator.storage.persist()}catch(e){}
  $("#syncState").textContent="Guardado en este dispositivo · funciona sin internet";
  render(true);
}
/* El service worker lee este estado para personalizar las notificaciones push */
function syncSW(){
  try{
    const t=today(),type=SCHEDULE[new Date().getDay()];
    const st={date:t,trainDay:!!type,session:type?SESSIONS[type].name:null,done:doneOn(t),skipped:!!(S.logs[t]&&S.logs[t].skipped),streak:streak(),why:S.profile.why||"",trainTime:S.profile.trainTime,missed:lastMissed()};
    if("caches" in window) caches.open("forja-state").then(c=>c.put(new URL("state.json",location.href).href,new Response(JSON.stringify(st),{headers:{"content-type":"application/json"}}))).catch(()=>{});
    if(navigator.setAppBadge){ (type&&!st.done&&!st.skipped)?navigator.setAppBadge(1).catch(()=>{}):navigator.clearAppBadge().catch(()=>{}); }
  }catch(e){}
}
