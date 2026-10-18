use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
}

pub struct ScanOptions {
    pub min_size: u64,
    pub exclude: Vec<String>,
    pub follow_links: bool,
}

/// Recorre `root` de forma recursiva. Por defecto NO sigue symlinks (evita
/// bucles y archivos fuera de la carpeta); `follow_links` lo habilita a
/// propósito. Respeta `exclude` (nombres de carpeta a ignorar en cualquier
/// nivel) e incrementa `counter` por archivo aceptado.
pub fn scan(root: &Path, opts: ScanOptions, counter: Arc<AtomicUsize>) -> Vec<FileInfo> {
    let mut files = Vec::new();
    let walker = jwalk::WalkDir::new(root).follow_links(opts.follow_links);

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if entry.path_is_symlink() && !opts.follow_links {
            continue;
        }
        if !opts.exclude.is_empty() && is_excluded(&path, &opts.exclude) {
            continue;
        }
        if path.is_file() {
            if let Ok(meta) = entry.metadata() {
                let size = meta.len();
                if size >= opts.min_size {
                    files.push(FileInfo {
                        path,
                        size,
                        modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    });
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
    files
}

fn is_excluded(path: &Path, exclude: &[String]) -> bool {
    path.components().any(|c| match c {
        Component::Normal(name) => {
            let name = name.to_string_lossy();
            exclude.iter().any(|e| e == &*name)
        }
        _ => false,
    })
}
