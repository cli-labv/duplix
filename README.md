# 🧬 Duplix

CLI minimalista en **Rust** para encontrar archivos **duplicados** y ordenarlos
sin romper nada: los duplicados van a `duplicate/`, el que conservas va a
`keep/`, y todo se puede **revertir**. Cien por ciento **local**, sin nube.

---

## 🎯 ¿Para qué sirve?
Cuando tienes carpetas con copias repetidas (fotos, PDFs, música), Duplix:
- 🔍 compara el **contenido** real (hash BLAKE3), no solo el nombre,
- 📊 te muestra una tabla clara y decide qué conservar,
- 📦 mueve el resto a carpetas ordenadas,
- ↩️ guarda un historial para **deshacer** cuando quieras.

## ✨ Características
- 🚀 Rápido (escaneo y hash en paralelo con `rayon`).
- 🧠 Solo se queda con **un** original por grupo; el resto son duplicados.
- ↩️ **Reversión** total o de la última ejecución.
- 🧹 Comando para limpiar caché/historial (nunca toca tus archivos).
- 🔒 Todo queda en la carpeta del proyecto: **nada** sale a tu HOME ni al PATH.

## 🧰 Requisitos
- 🦀 Rust + Cargo instalados (`rustup`).

## 🚀 Primer uso
La **primera vez** compila (tarda un poco). Las siguientes son instantáneas:

```bash
./start.sh      # compila y deja la app lista (solo la primera vez)
./start         # ya puedes usarla
```

## 🎮 Uso
```bash
./start            # organiza los duplicados de ./input
./start --menu     # menú interactivo (buscar, revertir, historial, limpiar)
./start undo       # revierte la última ejecución
./start undo <log> # revierte una ejecución concreta
./start clean      # limpia historial y carpetas vacías
./start --yes      # sin preguntar, aplica la regla automática
```
> ⚠️ Importante: hay un **espacio** entre `start` y la opción → `./start --menu`.

## 🗂️ Cómo organiza
```
tu-proyecto/
├── input/            ← pones aquí tus archivos
│   ├── duplicate/    ← duplicados movidos aquí
│   └── keep/         ← el original que se conserva
└── json/             ← historial (para poder revertir)
```

## 🛠️ Cómo se hizo
- 🦀 **Rust** con `clap` para la CLI, `jwalk` + `rayon` para recorrer y hashear
  en paralelo, `blake3` para comparar contenido, `dialoguer` + `indicatif` +
  `console` para el menú y las tablas (render propio, sin TUI pesada).
- 🔒 Sin escrituras fuera de la carpeta local (`DUPLIX_HOME`, por defecto el
  directorio del proyecto).

## 📄 Licencia
MIT OR Apache-2.0.
