use std::path::Path;

/// `true` si ambas rutas apuntan al mismo dato físico en disco
/// (hardlink, symlink resuelto o la misma ruta). No son duplicados reales.
pub fn is_same_file(a: &Path, b: &Path) -> bool {
    same_file::is_same_file(a, b).unwrap_or(false)
}
