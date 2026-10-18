use crate::paths;
use std::fs;
use std::path::Path;

/// Elemento eliminado durante la limpieza.
pub struct CleanItem {
    pub what: String,
    pub bytes: u64,
    pub files: usize,
}

#[derive(Default)]
pub struct CleanReport {
    pub items: Vec<CleanItem>,
    pub errors: Vec<String>,
}

/// Limpia lo que genera la app: el historial JSON (fuera del root) y las
/// carpetas `duplicate/` y `keep/` cuando quedaron vacías (restos de una
/// ejecución). Nunca toca los archivos del usuario.
pub fn clean(root: &Path) -> CleanReport {
    let mut report = CleanReport::default();

    let history = paths::history_dir();
    if history.exists() {
        remove(&history, "historial json", &mut report);
    }

    for name in paths::generated_dirs() {
        let dir = root.join(name);
        if dir.is_dir() && is_empty(&dir) {
            remove(&dir, name, &mut report);
        }
    }

    report
}

fn remove(path: &Path, what: &str, report: &mut CleanReport) {
    let (bytes, files) = size_of(path);
    match fs::remove_dir_all(path) {
        Ok(()) => report.items.push(CleanItem {
            what: what.to_string(),
            bytes,
            files,
        }),
        Err(e) => report.errors.push(format!("{}: {}", path.display(), e)),
    }
}

fn is_empty(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|mut it| it.next().is_none())
        .unwrap_or(false)
}

fn size_of(path: &Path) -> (u64, usize) {
    if path.is_file() {
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        return (size, 1);
    }
    let mut bytes = 0;
    let mut files = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let (b, f) = size_of(&entry.path());
            bytes += b;
            files += f;
        }
    }
    (bytes, files)
}
