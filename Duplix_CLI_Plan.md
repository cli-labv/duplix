# Duplix — Plan de app CLI para detectar y mover archivos duplicados

> **Versión:** 1.0 · **Nombre de trabajo:** *Duplix* (renombrable)
> **Qué pediste:** escanear una carpeta, comparar por peso y tipo, mover todos los duplicados menos uno (el original) a una carpeta `duplicate`, dejar en paz los archivos sin duplicado, para cualquier tipo de archivo, con la mejor librería posible, el mejor stack para velocidad, e interfaz de menú interactivo navegable con flechas (no "escribe un número").

---

## 0. Una corrección importante antes de todo: peso + tipo no alcanza

Comparar solo por **tamaño** y **tipo** (extensión) va a darte falsos positivos: dos videos distintos de la misma duración suelen pesar casi lo mismo por pura coincidencia, y dos PDFs de igual tamaño pueden tener contenido completamente distinto. Si Duplix moviera archivos basándose solo en eso, correrías el riesgo real de mover (o, peor, perder de vista) un archivo que **no** era duplicado.

La forma correcta — y es literalmente así como funcionan las herramientas de este tipo que investigué (Czkawka, Fclones, y un proyecto más pequeño llamado `rustdupe` que hace casi exactamente lo que pediste) — es usar el tamaño y el tipo como **filtro rápido**, y confirmar la duplicidad real con un **hash de contenido**. Esto no contradice lo que pediste, lo completa: tamaño y tipo siguen siendo el primer paso (y el más barato), pero la decisión final de "esto es un duplicado" la toma el contenido, no la coincidencia de peso.

### El algoritmo en 4 fases (así evitás falsos positivos sin perder velocidad)

```
Fase 1 — Agrupar por TAMAÑO exacto en bytes
          (dos archivos de distinto tamaño NUNCA pueden ser duplicados: descarte gratis,
          sin leer ni un byte del contenido)
             │
Fase 2 — Dentro de cada grupo, agrupar por TIPO real
          (firma de bytes del archivo — "magic bytes" — no solo la extensión,
          porque un archivo puede estar mal nombrado)
             │
Fase 3 — Pre-hash rápido: hash de solo los primeros 4–8 KB de cada archivo
          (descarta la mayoría de los "casi iguales mas no iguales" sin leer el archivo entero)
             │
Fase 4 — Hash completo (BLAKE3) SOLO de los que sobrevivieron hasta acá
          → esto es lo que confirma de verdad "son el mismo contenido, byte a byte"
```

Esto es exactamente lo que hace que estas herramientas sean rápidas en discos con cientos de miles de archivos: el 95% de los archivos se descarta en la Fase 1 sin tocar el disco para leer contenido, y el hash completo (lo más caro) solo se calcula sobre un puñado de candidatos reales.

---

## 1. Objetivo y alcance

- Recorrer una carpeta (y sus subcarpetas, opcionalmente) de forma **recursiva**.
- Detectar grupos de archivos con contenido idéntico, **de cualquier tipo** (PDF, video, imagen, documento, lo que sea — no hay lógica especial por tipo de archivo, el contenido es el contenido).
- Por cada grupo de duplicados: dejar **uno** en su lugar (el "original", según una regla configurable) y mover **todos los demás** a una carpeta `duplicate/`.
- Los archivos **sin** duplicado no se tocan, no se mueven, no aparecen como acción pendiente — se ignoran, tal como pediste.
- Interfaz: nada de "escribí 1 para continuar". Un menú de verdad, navegable con flechas, con un proceso de pantallas **secuencial** (escaneo → comparación → revisión → ejecución → resumen).

---

## 2. ¿Cuál archivo se queda como "el original"?

Esto no lo pediste explícitamente, pero hay que decidirlo — moverse "todos menos uno" implica elegir cuál es ese uno. Se resuelve con una **regla configurable por línea de comandos**, con una por defecto razonable, y la posibilidad de **cambiarla a mano, grupo por grupo**, en la pantalla de revisión (ver §6):

| Regla (`--keep=`) | Qué hace | Cuándo conviene |
|---|---|---|
| `oldest` (por defecto) | Conserva el archivo con fecha de modificación más antigua | Asume que el primero que existió es "el original" |
| `newest` | Conserva el más reciente | Si solés quedarte con la última versión guardada |
| `shortest-path` | Conserva el que está en la ruta más corta (menos subcarpetas) | Útil si tus duplicados reales suelen quedar enterrados en subcarpetas de descargas/respaldos |
| `first-alpha` | Conserva el primero por orden alfabético de ruta | Determinista y simple |

En la pantalla de revisión, sea cual sea la regla, **siempre podés anular la elección a mano** en un grupo puntual con la tecla indicada — la regla es el punto de partida, no una imposición.

---

## 3. Requisitos funcionales

### 3.1 Escaneo y comparación
| ID | Requisito |
|---|---|
| RF-01 | Escanear una carpeta de forma recursiva, con exclusión opcional de subcarpetas por patrón (ej. no tocar `node_modules`, `.git`) |
| RF-02 | Filtro por tamaño mínimo (ignorar archivos de 0 bytes o menores a un umbral, para no perder tiempo con archivos triviales) |
| RF-03 | Agrupar por tamaño exacto, luego por tipo real (firma de bytes), luego por pre-hash, luego por hash completo — el algoritmo del §0 |
| RF-04 | Detectar y **excluir** hardlinks (mismo archivo físico en disco con dos nombres) — eso no es un duplicado real, es el mismo dato |
| RF-05 | Manejar symlinks de forma explícita y segura: por defecto no seguirlos (evita bucles infinitos y mover algo fuera de la carpeta escaneada por accidente), con flag para incluirlos a propósito |
| RF-06 | Archivos sin duplicado: no aparecen en ninguna acción, no se tocan |

### 3.2 Acción (mover) y seguridad
| ID | Requisito |
|---|---|
| RF-10 | Mover los duplicados (todos menos el elegido como original) a una carpeta `duplicate/` dentro de la carpeta escaneada (ruta configurable) |
| RF-11 | Si dos duplicados tienen el mismo nombre de archivo al moverlos (vienen de subcarpetas distintas), evitar que se pisen entre sí (sufijo automático, o conservar la subestructura de carpetas dentro de `duplicate/`) |
| RF-12 | Modo `--dry-run`: mostrar exactamente qué se movería, sin mover nada todavía |
| RF-13 | **Registro de deshacer**: cada ejecución real genera un archivo de log (JSON) con el origen y destino exacto de cada archivo movido, para poder revertir la operación completa con un comando `duplix undo <log>` |
| RF-14 | Mover entre distintas unidades/discos si hace falta (si un `rename` directo falla por cruzar sistemas de archivos, copiar y luego borrar el original, verificando el hash después de copiar antes de borrar) |
| RF-15 | Nunca borrar nada de forma permanente — la operación es siempre "mover a una carpeta", consistente con lo que pediste |

### 3.3 Interfaz interactiva
| ID | Requisito |
|---|---|
| RF-20 | Proceso en pantallas secuenciales: **Escaneo → Comparación/Hash → Revisión → Confirmación → Ejecución → Resumen** |
| RF-21 | Pantallas de Escaneo y Comparación: barra de progreso en vivo (archivos procesados, velocidad, tiempo estimado) — sin pedir ninguna acción al usuario, son automáticas |
| RF-22 | Pantalla de Revisión: lista de grupos de duplicados navegable con **flechas arriba/abajo**; dentro de cada grupo, lista de archivos con cuál quedaría como original marcado, cambiable con **Enter/Espacio** |
| RF-23 | Posibilidad de **omitir un grupo completo** (dejarlo tal cual, no mover nada de ese grupo) con una tecla dedicada, visible en la ayuda de la pantalla |
| RF-24 | Pantalla de Confirmación: resumen de cuántos archivos se moverán y cuánto espacio se liberará, antes de ejecutar nada |
| RF-25 | Pantalla de Ejecución: progreso del movimiento real de archivos |
| RF-26 | Pantalla de Resumen final: cuántos grupos, cuántos archivos movidos, espacio liberado, ruta del log de deshacer, y cualquier error puntual (permisos, archivo en uso) |
| RF-27 | Todas las pantallas muestran, en todo momento, un pie con las teclas disponibles (flechas, Enter, Espacio, `s` omitir, `q` salir) — nunca "escribí un número" |

### 3.4 No funcionales
| ID | Requisito |
|---|---|
| RNF-01 | Rendimiento: aprovechar todos los núcleos disponibles para el hashing (la fase más cara), sin bloquear la interfaz |
| RNF-02 | No modificar ni mover absolutamente nada hasta que el usuario confirme explícitamente en la pantalla de Confirmación |
| RNF-03 | Funcionar sobre Unicode sin romperse con nombres de archivo en distintas normalizaciones (relevante si alguna vez se usa sobre una carpeta sincronizada desde macOS) |
| RNF-04 | Un solo binario, sin dependencias externas que instalar aparte, para poder llevarlo y usarlo en cualquier máquina |

---

## 4. El stack para máxima velocidad: Rust puro, binario único

A diferencia de tus otros proyectos de esta conversación (que combinaban Python para orquestar y Rust para el cómputo pesado), acá **no hace falta Python en absoluto**: no hay una interfaz web, no hay que orquestar servicios — es una sola herramienta que corre en la terminal. Meter Python encima solo agregaría una capa de arranque más lenta y una dependencia de entorno (versión de Python, entorno virtual) que un binario de Rust no necesita: **se compila una vez y el resultado es un único ejecutable** que corrés directo, sin instalar nada más. Esa es la razón concreta de la elección, no solo "Rust es rápido".

### 4.1 Librerías (investigadas, con el motivo de cada una)

| Tarea | Crate | Por qué |
|---|---|---|
| Recorrer carpetas | **`jwalk`** | Recorre directorios en paralelo (usa varios hilos para leer el árbol de carpetas a la vez), más rápido que el clásico `walkdir` en discos con muchísimos archivos; se puede combinar con `walkdir` como respaldo más simple si en algún caso `jwalk` da problemas de orden |
| Paralelismo de cómputo (hashear muchos archivos a la vez) | **`rayon`** | El estándar de facto en Rust para paralelismo de datos; convierte un `for` secuencial de hashing en uno repartido entre núcleos con un cambio mínimo de código |
| Hash de confirmación (Fase 4) | **`blake3`** | Hash criptográfico moderno, varias veces más rápido que SHA-256 en los mismos benchmarks que consulté, y ya viene con paralelismo interno para archivos grandes — es la elección que usan las herramientas de referencia que investigué (Czkawka, el proyecto `rustdupe`) |
| Pre-hash rápido (Fase 3) | **`xxhash-rust`** (variante XXH3) | Más liviano que BLAKE3 para leer solo los primeros KB de miles de archivos — se usa solo como filtro barato, no como confirmación final |
| Identificar el tipo real del archivo | **`infer`** | Detecta el tipo por la firma de bytes del archivo (los primeros bytes que identifican si es un PDF, un MP4, un PNG, etc.), no por la extensión del nombre — así no te engaña un archivo mal renombrado |
| Detectar hardlinks (mismo archivo físico) | **`same-file`** | Compara identificadores de archivo del sistema operativo para saber si dos rutas son, en realidad, el mismo dato en disco |
| Interfaz de terminal con flechas | **`ratatui`** + **`crossterm`** | `ratatui` es el framework estándar actual para construir interfaces de texto completas (listas navegables, paneles, barras de progreso) en Rust; `crossterm` es la capa que lee las teclas y dibuja en cualquier terminal (Linux/macOS/Windows) |
| Barras de progreso (si se usan fuera de `ratatui`, p. ej. en modo no interactivo) | **`indicatif`** | Barras de progreso prolijas para el modo `--dry-run`/script, sin pantalla completa |
| Argumentos de línea de comandos | **`clap`** (con derive) | El estándar para parsear flags (`--keep=oldest`, `--dry-run`, carpeta de destino, etc.) con ayuda automática |
| Guardar el log de deshacer y configuración | **`serde`** + **`serde_json`** | Serializar el manifiesto de movimientos a JSON de forma simple y confiable |

### 4.2 Proyectos que investigué como referencia directa
- **`rustdupe`**: hace casi exactamente lo que pediste — TUI interactiva, BLAKE3, las mismas 4 fases de filtrado, detección de hardlinks, mueve (no borra) por defecto, exporta resultados a JSON. Es la referencia más cercana a copiar en estructura.
- **Czkawka** / **Fclones**: las herramientas de duplicados en Rust más maduras y más usadas; en una comparación que consulté, Czkawka procesó **363 215 archivos (221,8 GB)** y encontró duplicados en **8 segundos** en la primera ejecución — da una idea realista de a qué velocidad apuntar.

---

## 5. Arquitectura del programa

```
duplix/
├─ Cargo.toml
├─ src/
│  ├─ main.rs              # arranque, parseo de argumentos (clap), orquesta las etapas
│  ├─ cli.rs                # definición de flags: carpeta, --keep, --dry-run, --destino, etc.
│  ├─ scanner.rs             # recorrido de carpetas (jwalk), filtros de tamaño/exclusión
│  ├─ fingerprint.rs          # Fases 1-4: tamaño → tipo (infer) → pre-hash (xxhash) → hash (blake3)
│  ├─ grouping.rs             # arma los grupos finales de duplicados confirmados
│  ├─ original_rule.rs        # aplica la regla --keep (oldest/newest/shortest-path/first-alpha)
│  ├─ mover.rs                # ejecuta el movimiento real, con respaldo copiar+verificar+borrar entre discos
│  ├─ undo.rs                 # lee un log y revierte los movimientos
│  ├─ safety.rs               # detección de hardlinks/symlinks, normalización Unicode
│  └─ ui/
│     ├─ mod.rs
│     ├─ scan_screen.rs        # pantalla de progreso de escaneo
│     ├─ hash_screen.rs        # pantalla de progreso de hashing
│     ├─ review_screen.rs      # pantalla interactiva de revisión (navegación con flechas)
│     ├─ confirm_screen.rs     # pantalla de confirmación final
│     ├─ execute_screen.rs     # pantalla de progreso de movimiento
│     └─ summary_screen.rs     # pantalla de resumen
└─ tests/
   ├─ fingerprint_tests.rs
   ├─ mover_tests.rs
   └─ fixtures/                # carpetas de prueba con duplicados conocidos de antemano
```

### 5.1 Flujo de datos

```
Carpeta objetivo
   │
   ▼ scanner.rs (jwalk, en paralelo)
Lista de archivos (ruta, tamaño, metadatos)
   │
   ▼ fingerprint.rs (rayon reparte el trabajo entre núcleos)
Grupos candidatos por tamaño+tipo → pre-hash → hash completo
   │
   ▼ grouping.rs + original_rule.rs
Grupos de duplicados confirmados, cada uno con un "original" sugerido
   │
   ▼ ui/review_screen.rs (el usuario revisa/ajusta con flechas)
Lista final de "qué mover" aprobada por el usuario
   │
   ▼ mover.rs (genera el log de deshacer ANTES de mover, por seguridad)
Archivos movidos a duplicate/ + log en duplix_undo_<fecha>.json
```

---

## 6. La pantalla de revisión, en detalle (la pieza central de la interfaz)

Esto es lo que resuelve tu pedido de "nada de escribir un número, quiero flechas":

```
┌─ Duplix · Revisión de duplicados (grupo 3 de 47) ──────────────────────────┐
│                                                                             │
│  Grupo: 2 copias · 14.2 MB cada una · tipo: video/mp4                      │
│                                                                             │
│  ▸ ● /home/user/Videos/vacaciones.mp4         (ORIGINAL — se conserva)     │
│    ○ /home/user/Descargas/vacaciones (1).mp4  (se moverá a duplicate/)     │
│                                                                             │
│  ↑/↓ moverse entre archivos del grupo   Enter/Espacio cambiar cuál es el   │
│  original   ←/→ grupo anterior/siguiente   s omitir este grupo   a todo    │
│  o aprobar todos los grupos con la regla por defecto   q salir sin guardar │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **Flechas arriba/abajo**: moverse dentro del grupo actual (entre los archivos que lo componen).
- **Flechas izquierda/derecha**: pasar al grupo anterior/siguiente (así se revisan los 47 grupos uno por uno, de forma secuencial, como pediste).
- **Enter o Espacio**: cambiar cuál de los archivos del grupo actual es "el original" (anula la regla automática solo para ese grupo).
- **`s`**: omitir el grupo completo — no se mueve nada de ahí.
- **`a`**: aprobar de una todos los grupos restantes con la regla por defecto, para no tener que pasar uno por uno si ya confiás en la regla.
- **`q`**: salir sin aplicar ningún cambio.

---

## 7. El log de deshacer (seguridad real, no cosmética)

Antes de mover el primer archivo, Duplix escribe un manifiesto:

```json
{
  "fecha": "2026-10-11T14:32:00Z",
  "carpeta_escaneada": "/home/user/Fotos",
  "carpeta_duplicados": "/home/user/Fotos/duplicate",
  "movimientos": [
    { "origen": "/home/user/Fotos/img_002.jpg", "destino": "/home/user/Fotos/duplicate/img_002.jpg", "hash": "a1b2c3..." }
  ]
}
```

`duplix undo duplix_undo_2026-10-11.json` lee ese archivo y devuelve cada uno a su ruta original, verificando por hash que el contenido no cambió en el camino. Esto es lo que te permite usar la herramienta con confianza incluso si la regla automática eligió mal el "original" en algún grupo que no revisaste a mano.

---

## 8. Casos borde que hay que manejar bien

| Caso | Cómo se resuelve |
|---|---|
| Dos archivos duplicados tienen el mismo nombre pero vienen de subcarpetas distintas | Al moverlos a `duplicate/`, conservar un rastro de la subcarpeta de origen en el nombre (o recrear la subestructura dentro de `duplicate/`) para que no se pisen |
| Archivo sin permisos de lectura/escritura | Se reporta en el resumen final como error puntual, no frena el resto del proceso |
| Mover entre distintas unidades de disco | `rename` directo falla entre sistemas de archivos distintos; respaldo automático: copiar, verificar el hash de la copia, y recién ahí borrar el original |
| Hardlinks (mismo archivo con dos nombres) | Se detectan y se excluyen de los grupos — no son duplicados reales, es el mismo dato físico |
| Symlinks | No se siguen por defecto (evita bucles y mover algo fuera de la carpeta escaneada sin querer); opción explícita para incluirlos |
| Carpeta `duplicate/` ya existe de una corrida anterior | Se reutiliza, no se sobrescribe lo que ya había ahí sin avisar |

---

## 9. Pruebas

| Área | Prueba |
|---|---|
| Fingerprinting | Carpetas de prueba (`tests/fixtures/`) con duplicados exactos conocidos de antemano, casi-duplicados (mismo tamaño, distinto contenido) y archivos únicos — confirmar que el algoritmo no da ni un falso positivo ni un falso negativo |
| Regla de "original" | Cada regla (`oldest`, `newest`, `shortest-path`, `first-alpha`) probada contra un grupo de fechas/rutas conocidas |
| Movimiento seguro | Simular un corte a mitad de la operación (matar el proceso) y confirmar que el log de deshacer permite recuperar el estado |
| Cruce de discos | Prueba con un directorio temporal en otro punto de montaje, confirmando el respaldo copiar+verificar+borrar |
| Hardlinks/symlinks | Carpeta de prueba con ambos casos, confirmando que se excluyen/manejan como corresponde |
| Rendimiento | Carpeta sintética de decenas de miles de archivos, midiendo tiempo de escaneo+hash en el hardware real, comparándolo contra el número de referencia de Czkawka del §4.2 |

---

## 10. Fases de desarrollo

**Fase 0 — Núcleo de detección (sin interfaz):** `scanner.rs` + `fingerprint.rs` + `grouping.rs`, probado por línea de comandos imprimiendo los grupos encontrados en texto plano. Esto ya es usable con `--dry-run` aunque no tenga TUI todavía.

**Fase 1 — Movimiento seguro:** `original_rule.rs` + `mover.rs` + `undo.rs`, con el log de deshacer funcionando de punta a punta antes de construir ninguna pantalla.

**Fase 2 — Interfaz interactiva:** las seis pantallas de `ui/` con `ratatui`, empezando por la de Revisión (la más importante), siguiendo el flujo secuencial completo.

**Fase 3 — Pulido y casos borde:** hardlinks/symlinks, cruce de discos, normalización Unicode, manejo de errores por archivo sin frenar el proceso completo.

**Fase 4 — Rendimiento y empaquetado:** *benchmarks* contra carpetas grandes reales, ajuste de paralelismo, compilación de binarios para Linux/macOS/Windows.

---

## 11. Riesgos

| Riesgo | Mitigación |
|---|---|
| Mover un archivo que en realidad no era duplicado por un bug en el algoritmo | Hash completo con BLAKE3 como confirmación final, nunca solo tamaño+tipo; pruebas de la Fase 0 con casos "casi duplicados" a propósito |
| El usuario confirma sin revisar bien y pierde de vista un archivo importante | El log de deshacer (§7) es obligatorio en toda ejecución real, no opcional |
| Rendimiento pobre en discos de red o muy lentos | El cuello de botella ahí es el disco, no la CPU — ningún cambio de librería lo arregla; documentarlo como limitación esperada, no como bug |
| Corte de luz/cierre forzado a mitad del movimiento | Log de deshacer escrito **antes** de empezar a mover, para que lo ya movido quede siempre reconstruible |

---

## 12. Prompt operativo para el agente CLI

```text
ROL: Desarrollador Rust especializado en herramientas de línea de comandos.

PROYECTO: Duplix — CLI para detectar y mover archivos duplicados a una carpeta "duplicate",
dejando un original por grupo y sin tocar los archivos sin duplicado.

ALGORITMO FIJO (no simplificar a solo tamaño+tipo):
1. Agrupar por tamaño exacto.
2. Agrupar por tipo real (firma de bytes con el crate `infer`, no por extensión).
3. Pre-hash de los primeros KB con `xxhash-rust` (XXH3) como filtro rápido.
4. Hash completo con `blake3` como confirmación final de duplicado real.
Solo lo que sobrevive las 4 fases se considera duplicado.

STACK FIJO: Rust puro (sin Python), un solo binario.
Crates: jwalk, rayon, blake3, xxhash-rust, infer, same-file, ratatui, crossterm,
indicatif, clap (derive), serde + serde_json.

REGLAS DE SEGURIDAD NO NEGOCIABLES:
- Nunca borrar archivos; solo mover a la carpeta de duplicados.
- Generar el log de deshacer (JSON) ANTES de mover el primer archivo de cada ejecución real.
- Nada se mueve sin pasar por la pantalla de Confirmación.
- Excluir hardlinks de los grupos de duplicados.

INTERFAZ: TUI con `ratatui`, flujo secuencial de 6 pantallas (Escaneo → Hash → Revisión →
Confirmación → Ejecución → Resumen). La pantalla de Revisión se navega con flechas
(arriba/abajo entre archivos del grupo, izquierda/derecha entre grupos), Enter/Espacio
para cambiar cuál archivo es el original, `s` para omitir un grupo. Nunca pedir al
usuario que escriba un número para elegir una acción.

TAREA INICIAL:
a) Fase 0: scanner.rs + fingerprint.rs + grouping.rs, con tests contra tests/fixtures/
   que incluyan duplicados exactos, casi-duplicados (mismo tamaño, distinto contenido)
   y archivos únicos.
b) Confirmar que no hay ni un falso positivo ni un falso negativo antes de avanzar a la Fase 1.
c) Reportar: estructura creada, resultados de los tests, siguiente paso.
```

---

### Cierre

El cambio de fondo respecto a lo que pediste no es técnico, es de criterio: tamaño y tipo siguen siendo el primer filtro, tal como lo planteaste, pero la decisión final de "esto es un duplicado" la toma un hash de contenido (BLAKE3), porque es la única forma de que "mover todos los duplicados menos uno" sea seguro de verdad. El resto — Rust puro para velocidad, `ratatui` para la navegación por flechas, y un log de deshacer que se escribe antes de mover nada — está pensado para que puedas confiar en correr esta herramienta sobre tus carpetas reales sin miedo a perder un archivo por accidente.
