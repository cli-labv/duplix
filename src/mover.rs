use crate::fingerprint::Fingerprint;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MoveEntry {
    pub origen: PathBuf,
    pub destino: PathBuf,
    pub hash: String,
    /// `true` si es el archivo que se conserva (va a keep/); `false` si es un
    /// duplicado (va a duplicate/).
    #[serde(default)]
    pub kept: bool,
    /// `true` una vez que el movimiento fue revertido (queda registrado).
    #[serde(default)]
    pub revertido: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UndoLog {
    pub fecha: String,
    /// Fecha (ms) en que se revirtió esta ejecución, si ocurrió.
    #[serde(default)]
    pub revertido_en: Option<String>,
    pub carpeta_escaneada: PathBuf,
    pub carpeta_duplicados: PathBuf,
    pub movimientos: Vec<MoveEntry>,
}

#[derive(Debug, Default)]
pub struct MoveReport {
    pub moved: usize,
    pub freed: u64,
    pub errors: Vec<String>,
}

/// Resumen de un grupo, para la tabla final.
pub struct GroupSummary {
    pub index: usize,
    pub copies: usize,
    pub size: u64,
    pub kept: String,
    pub moved: usize,
}

pub struct MovePlan {
    pub moves: Vec<MoveEntry>,
    pub groups: Vec<GroupSummary>,
    pub freed: u64,
}

impl MovePlan {
    pub fn kept_count(&self) -> usize {
        self.groups.len()
    }
    pub fn duplicates(&self) -> usize {
        self.moves.iter().filter(|m| !m.kept).count()
    }
}

/// Construye los movimientos: cada grupo conserva un archivo (a `dest_keep`) y
/// manda el resto a `dest_dupe`. Evita colisiones de nombre.
pub fn build_moves(
    groups: &[Vec<Fingerprint>],
    keep: &[usize],
    dest_dupe: &Path,
    dest_keep: &Path,
) -> MovePlan {
    let mut moves = Vec::new();
    let mut summary = Vec::new();
    let mut freed = 0u64;
    let mut used: HashSet<PathBuf> = HashSet::new();

    for (gi, group) in groups.iter().enumerate() {
        let keep_idx = keep.get(gi).copied().unwrap_or(0);
        if keep_idx == usize::MAX {
            continue;
        }
        let mut moved = 0;
        for (i, fp) in group.iter().enumerate() {
            let kept = i == keep_idx;
            let target = if kept { dest_keep } else { dest_dupe };
            let destino = unique_dest(target, &fp.path, &mut used);
            if !kept {
                freed += fp.size;
                moved += 1;
            }
            moves.push(MoveEntry {
                origen: fp.path.clone(),
                destino,
                hash: fp.hash.to_hex().to_string(),
                kept,
                revertido: false,
            });
        }
        summary.push(GroupSummary {
            index: summary.len() + 1,
            copies: group.len(),
            size: group[0].size,
            kept: file_name(&group[keep_idx].path),
            moved,
        });
    }

    MovePlan {
        moves,
        groups: summary,
        freed,
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn unique_dest(dest_root: &Path, src: &Path, used: &mut HashSet<PathBuf>) -> PathBuf {
    let name = src.file_name().unwrap_or_default();
    let stem = Path::new(name)
        .file_stem()
        .map(|s| s.to_os_string())
        .unwrap_or_else(|| name.to_os_string());
    let ext = Path::new(name)
        .extension()
        .map(|s| s.to_os_string())
        .unwrap_or_default();

    let mut n = 0u32;
    loop {
        let file = if n == 0 {
            name.to_os_string()
        } else if ext.is_empty() {
            format!("{} ({})", stem.to_string_lossy(), n).into()
        } else {
            format!("{} ({}).{}", stem.to_string_lossy(), n, ext.to_string_lossy()).into()
        };
        let candidate = dest_root.join(file);
        if !candidate.exists() && !used.contains(&candidate) {
            used.insert(candidate.clone());
            return candidate;
        }
        n += 1;
    }
}

pub fn write_log(path: &Path, log: &UndoLog) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let json = serde_json::to_string_pretty(log).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

/// Escribe el log de deshacer ANTES de mover nada, y luego ejecuta cada
/// movimiento. Un fallo no frena el resto: se acumula en `errors`.
pub fn execute_moves<F: FnMut(usize, u64)>(
    log: &UndoLog,
    log_path: &Path,
    mut progress: F,
) -> Result<MoveReport, String> {
    write_log(log_path, log)?;
    let mut report = MoveReport::default();
    for m in &log.movimientos {
        let size = fs::metadata(&m.origen).map(|md| md.len()).unwrap_or(0);
        match move_one(m) {
            Ok(()) => {
                report.moved += 1;
                report.freed += size;
                progress(report.moved, size);
            }
            Err(e) => report.errors.push(format!(
                "{} -> {}: {}",
                m.origen.display(),
                m.destino.display(),
                e
            )),
        }
    }
    Ok(report)
}

fn move_one(m: &MoveEntry) -> io::Result<()> {
    if let Some(parent) = m.destino.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(&m.origen, &m.destino) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_and_verify(&m.origen, &m.destino, &m.hash)?;
            fs::remove_file(&m.origen)
        }
    }
}

fn copy_and_verify(src: &Path, dst: &Path, expected: &str) -> io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dst)?;
    let got = crate::fingerprint::hash_file(dst)?.to_hex().to_string();
    if got != expected {
        let _ = fs::remove_file(dst);
        return Err(io::Error::other("hash distinto tras copia (copia descartada)"));
    }
    Ok(())
}

pub fn timestamp() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    millis.to_string()
}
