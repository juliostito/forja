# Forja 4×: cómo instalarla en tu iPhone

En total son unos 15 minutos y lo haces una sola vez. Después la app funciona sola, sin Claude.

## 1. Crea tu cuenta de GitHub (2 min)
1. Entra a https://github.com/signup.
2. Elige un nombre de usuario. Aparecerá en la dirección de tu app: `https://TU-USUARIO.github.io/forja/`.
3. Confirma tu correo.

## 2. Sube la app (elige UNA opción)

**Opción A: yo la subo por ti (recomendado)**
1. Conecta GitHub a Claude en https://claude.ai/connect-github.
2. En GitHub, crea un repositorio vacío llamado `forja` (botón **New**, marca **Public** y no agregues archivos).
3. Escríbeme tu nombre de usuario. Yo subo los archivos, activo la página y guardo la clave secreta del servidor de avisos.

**Opción B: la subes tú**
1. En GitHub: **New** → nombre `forja` → **Public** → **Create repository**.
2. Toca **uploading an existing file**, arrastra todo el contenido de `forja-app.zip` (descomprimido; incluye la carpeta `.github`) y luego **Commit changes**.
3. Ve a **Settings → Pages → Branch: main / (root) → Save**. En 1–2 minutos tu app queda en `https://TU-USUARIO.github.io/forja/`.
4. Ve a **Settings → Secrets and variables → Actions → New repository secret**.
   - Nombre: `VAPID_PRIVATE_KEY`.
   - Valor: el contenido del archivo `SECRETO-NO-SUBIR-vapid-private.txt`.
   - Ese archivo **no** se sube al repositorio.

## 3. Instala la app en el iPhone (1 min)
1. Abre tu dirección `https://TU-USUARIO.github.io/forja/` en **Safari** (tiene que ser Safari).
2. Toca **Compartir** (el cuadrado con la flecha) → **Agregar a pantalla de inicio** → **Agregar**.
3. Abre **Forja** desde el ícono nuevo.

## 4. Activa las notificaciones (3 min)
1. En la app, ve a **Plan → Notificaciones → Activar notificaciones** y toca **Permitir**.
2. Toca **Copiar código**.
3. En GitHub, en tu repositorio `forja`, ve a **Settings → Secrets and variables → Actions → New repository secret**.
   - Nombre: `PUSH_SUBSCRIPTIONS`.
   - Valor: pega el código y guarda.
4. Para probar, ve a **Actions → Avisos Forja → Run workflow → Run**. En menos de un minuto te llega una notificación.

## 5. Conecta Apple Salud (opcional, 10 min)
Una app web no puede leer Salud directamente, porque Apple no lo permite. La app **Atajos** sí puede, así que creamos un atajo que copia tus datos del día y Forja los pega.

1. Abre **Atajos** → **+** (nuevo atajo) → ponle de nombre **Forja Salud**.
2. Agrega **Buscar muestras de salud** (Find Health Samples) y configúralo así:
   - Tipo: **Pasos**.
   - Fecha de inicio: **es hoy**.
   - Agrupar por: **Día**.
   - Luego agrega **Calcular estadísticas → Suma**.
   - Toca la variable que resulta y renómbrala **Pasos**.
3. Repite el paso 2 con estos tipos (Fecha de inicio **es hoy**, Suma):
   - **Energía activa** → variable **Kcal**.
   - **Minutos de ejercicio** → variable **Ejercicio**.
4. Agrega **Buscar muestras de salud** con tipo **Frecuencia cardíaca en reposo**, orden **más reciente primero**, límite **1** → variable **FCR**.
5. Agrega **Buscar muestras de salud** con tipo **Peso corporal**, orden **más reciente primero**, límite **1** → variable **Peso**.
6. Opcional: agrega **Análisis del sueño** desde ayer a las 18:00, con **Suma** → variable **Sueño** (en horas).
7. Agrega **Texto** y escribe exactamente esto, insertando cada variable donde corresponde:
   `FORJA;pasos=[Pasos];kcal=[Kcal];ejercicio=[Ejercicio];fcr=[FCR];peso=[Peso];sueno=[Sueño]`
8. Agrega **Copiar al portapapeles**.
9. Ejecuta el atajo una vez. iOS te pedirá permiso para leer Salud: acéptalo. Después abre Forja → **Hoy** → **Pegar datos de Salud**.

**Para que sea automático:** en Atajos ve a **Automatización → + → Hora del día → 21:00 → Diariamente → Ejecutar inmediatamente** y elige **Forja Salud**. Cada noche abres Forja y tocas **Pegar datos de Salud**.

Si tu reloj no es Apple Watch, revisa que su app (Garmin Connect, Samsung Health, Huawei Salud, etc.) tenga activada la sincronización con Apple Salud. Si no la tiene, sus datos no llegarán a Salud ni a Forja.

## Cómo funciona
- **Avisos:** a las 6:45 todos los días te dice qué toca. A las 20:00 los días de entrenamiento te insiste si la sesión sigue pendiente, o te felicita si ya la hiciste. El domingo a las 10:00 te recuerda la revisión. GitHub a veces atrasa los avisos entre 5 y 20 minutos.
- **Tus datos:** quedan solo en tu iPhone. Exporta una copia una vez al mes en **Progreso → Copia de seguridad**.
- **Si borras la app** o cambias de teléfono, vuelve a activar las notificaciones y actualiza el secreto `PUSH_SUBSCRIPTIONS`.
- **En el computador:** puedes instalar la misma dirección desde Edge o Chrome con **Instalar Forja**. Ojo: sus datos van aparte de los del iPhone.
