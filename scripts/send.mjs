// Envía las notificaciones push de Forja. Lo ejecuta GitHub Actions a la hora programada.
// Secretos necesarios (Settings → Secrets and variables → Actions):
//   VAPID_PRIVATE_KEY   la clave privada (archivo SECRETO-NO-SUBIR-vapid-private.txt)
//   PUSH_SUBSCRIPTIONS  el código que muestra la app al activar notificaciones (uno, o varios en una lista [ ... ])
import webpush from "web-push";
import { readFileSync } from "node:fs";

const TZ = "America/Santiago";
const PUBLIC = readFileSync(new URL("../vapid-public.txt", import.meta.url), "utf8").trim();
const PRIVATE = (process.env.VAPID_PRIVATE_KEY || "").trim();
const RAW = (process.env.PUSH_SUBSCRIPTIONS || "").trim();
if (!PRIVATE || !RAW) { console.log("Faltan los secretos VAPID_PRIVATE_KEY o PUSH_SUBSCRIPTIONS. Nada que enviar."); process.exit(0); }

let kind = (process.argv[2] || "auto").trim();
const p = Object.fromEntries(new Intl.DateTimeFormat("en-US", { timeZone: TZ, hour12: false, weekday: "short", hour: "2-digit", minute: "2-digit" }).formatToParts(new Date()).map(x => [x.type, x.value]));
const mins = (+p.hour % 24) * 60 + +p.minute, dow = p.weekday;
console.log(`Hora en Chile: ${dow} ${p.hour}:${p.minute}`);
if (kind === "auto") {
  // GitHub puede retrasar los cron; cada aviso tiene una ventana y solo una de las dos ejecuciones (horario de verano/invierno) cae dentro.
  if (mins >= 6 * 60 && mins < 7 * 60 + 20) kind = "morning";
  else if (mins >= 19 * 60 + 30 && mins < 21 * 60 && ["Mon", "Tue", "Thu", "Fri"].includes(dow)) kind = "evening";
  else if (mins >= 9 * 60 + 30 && mins < 11 * 60 && dow === "Sun") kind = "sunday";
  else { console.log("Fuera de las ventanas de aviso; no se envía nada."); process.exit(0); }
}

webpush.setVapidDetails("mailto:forja-app@example.com", PUBLIC, PRIVATE);
let subs = JSON.parse(RAW); if (!Array.isArray(subs)) subs = [subs];
let ok = 0;
for (const s of subs) {
  try { await webpush.sendNotification(s, JSON.stringify({ kind }), { TTL: 3 * 3600, urgency: "high" }); ok++; }
  catch (e) { console.error(`Error enviando a un dispositivo: ${e.statusCode || ""} ${e.body || e.message}`); if (e.statusCode === 404 || e.statusCode === 410) console.error("Esa suscripción ya no es válida: activa de nuevo las notificaciones en la app y actualiza el secreto."); }
}
console.log(`Aviso "${kind}" enviado a ${ok} de ${subs.length} dispositivo(s).`);
if (!ok) process.exit(1);
