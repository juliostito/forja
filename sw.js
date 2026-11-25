/* Forja 4× · service worker: funciona sin internet + notificaciones push */
const V = "forja-v1";
const ASSETS = ["./", "./index.html", "./manifest.webmanifest", "./icon-192.png", "./icon-512.png", "./apple-touch-icon.png"];
self.addEventListener("install", e => { e.waitUntil(caches.open(V).then(c => c.addAll(ASSETS)).then(() => self.skipWaiting())); });
self.addEventListener("activate", e => {
  e.waitUntil(caches.keys().then(ks => Promise.all(ks.filter(k => ![V, "forja-state", "forja-fonts"].includes(k)).map(k => caches.delete(k)))).then(() => self.clients.claim()));
});
self.addEventListener("fetch", e => {
  const r = e.request; if (r.method !== "GET") return;
  const u = new URL(r.url);
  if (u.hostname.endsWith("fonts.googleapis.com") || u.hostname.endsWith("fonts.gstatic.com")) {
    e.respondWith(caches.open("forja-fonts").then(async c => (await c.match(r)) || fetch(r).then(res => { c.put(r, res.clone()); return res; })));
    return;
  }
  if (u.origin !== location.origin || u.pathname.endsWith("state.json")) return;
  if (r.mode === "navigate") { // red primero (para recibir actualizaciones), caché si no hay conexión
    e.respondWith(fetch(r).then(res => { caches.open(V).then(c => c.put("./index.html", res.clone())); return res; }).catch(() => caches.match("./index.html")));
    return;
  }
  e.respondWith(caches.match(r).then(hit => hit || fetch(r)));
});

const DOW = ["Dom", "Lun", "Mar", "Mié", "Jue", "Vie", "Sáb"];
const PLAN = { 1: "Upper A", 2: "Lower A", 4: "Upper B", 5: "Lower B" };
const pad = n => String(n).padStart(2, "0");
const iso = d => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
async function readState() {
  try { const c = await caches.open("forja-state"); const r = await c.match(new URL("state.json", self.registration.scope).href); return r ? await r.json() : null; } catch (e) { return null; }
}
function compose(kind, st) {
  const now = new Date(), dow = now.getDay(), today = iso(now), fresh = st && st.date === today;
  const sess = PLAN[dow], time = (st && st.trainTime) || "07:00", streak = st ? st.streak : 0;
  const why = st && st.why ? ` Recuerda: “${st.why.slice(0, 90)}”` : "";
  if (kind === "evening") {
    if (fresh && st.done) return ["Sesión cumplida ✔", `Racha: ${streak}. Esta noche: proteína completa y 7+ horas de sueño.`];
    return [`${sess || "Tu sesión"} sigue pendiente`, `Todavía puedes: 15 minutos de versión mínima ahora. Nunca fallar dos veces seguidas.${why}`];
  }
  if (kind === "sunday") return ["Domingo de revisión", "Pésate en ayunas, mira tu semana en Coach, cocina proteína para 3 días y envía tu resumen a tu compañero."];
  if (kind === "test") return ["Forja funciona 💪", "Así te llegarán los avisos."];
  // morning
  const missed = st && st.missed;
  if (sess) return [`Hoy: ${sess} a las ${time}`, `${missed ? "Fallaste la anterior: hoy no se falla dos veces. " : ""}Zapatillas, agua y calentamiento. Solo empieza.${streak ? " Racha: " + streak + "." : ""}`];
  if (dow === 0) return ["Domingo: pesaje y preparación", "Pésate en ayunas y prepara tu semana. A las 10:00 te mando la revisión."];
  return ["Día de caminar", "30–45 min a ritmo de conversación + 10 min de movilidad para la espalda."];
}
self.addEventListener("push", e => {
  let data = {}; try { data = e.data ? e.data.json() : {}; } catch (_) { data = { kind: (e.data && e.data.text()) || "morning" }; }
  e.waitUntil(readState().then(st => {
    const [title, body] = data.title ? [data.title, data.body || ""] : compose(data.kind || "morning", st);
    return self.registration.showNotification(title, { body, icon: "icon-192.png", badge: "icon-192.png", tag: data.kind || "forja", data: { url: "./" } });
  }));
});
self.addEventListener("notificationclick", e => {
  e.notification.close();
  e.waitUntil(self.clients.matchAll({ type: "window", includeUncontrolled: true }).then(cs => {
    for (const c of cs) if ("focus" in c) return c.focus();
    return self.clients.openWindow(e.notification.data && e.notification.data.url || "./");
  }));
});
