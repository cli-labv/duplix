use std::path::{Path, PathBuf};

/// Carpeta dentro del root con los duplicados movidos.
pub const DUPLICATE_DIR: &str = "duplicate";
/// Carpeta dentro del root con los archivos conservados.
pub const KEEP_DIR: &str = "keep";
/// Carpeta (fuera del root escaneado) con el historial de ejecuciones.
pub const HISTORY_DIR: &str = "json";

pub fn duplicate_dir(root: &Path) -> PathBuf {
    root.join(DUPLICATE_DIR)
}

pub fn keep_dir(root: &Path) -> PathBuf {
    root.join(KEEP_DIR)
}

/// Carpeta base de la app. Todo lo que genera la app queda aquí, sin tocar el
/// home ni el PATH del sistema. Se puede fijar con `DUPLIX_HOME`; si no, usa el
/// directorio del ejecutable; como último recurso, el directorio actual.
pub fn home() -> PathBuf {
    std::env::var_os("DUPLIX_HOME")
        .map(PathBuf::from)
        .or_else(|| exe_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

/// El historial vive en `<home>/json`, aislado dentro de la carpeta local.
pub fn history_dir() -> PathBuf {
    home().join(HISTORY_DIR)
}

/// Carpetas que la app genera dentro del root (no deben re-escanearse).
pub fn generated_dirs() -> [&'static str; 2] {
    [DUPLICATE_DIR, KEEP_DIR]
}

fn is_log(name: &str) -> bool {
    name.starts_with("duplix-undo-") && name.ends_with(".json")
}

/// Todos los logs del historial, ordenados por nombre (más reciente al final).
pub fn all_logs() -> Vec<PathBuf> {
    let dir = history_dir();
    let mut logs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| is_log(&e.file_name().to_string_lossy()))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    logs.sort();
    logs
}

/// El log más reciente del historial, si existe.
pub fn latest_log() -> Option<PathBuf> {
    all_logs().pop()
}
