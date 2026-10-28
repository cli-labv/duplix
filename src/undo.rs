use crate::mover::{self, UndoLog};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct UndoReport {
    pub restored: usize,
    pub errors: Vec<String>,
}

/// Revierte los movimientos de un log y deja registrado en el propio JSON qué
/// se revirtió (`revertido` por entrada y `revertido_en` a nivel de log).
pub fn undo(log_path: &Path) -> Result<UndoReport, String> {
    let data = fs::read_to_string(log_path).map_err(|e| e.to_string())?;
    let mut log: UndoLog = serde_json::from_str(&data).map_err(|e| e.to_string())?;

    let mut report = UndoReport::default();
    for m in log.movimientos.iter_mut().rev() {
        if m.revertido || !m.destino.exists() {
            continue;
        }
        if !m.hash.is_empty() {
            match crate::fingerprint::hash_file(&m.destino) {
                Ok(h) if h.to_hex().to_string() != m.hash => {
                    report.errors.push(format!(
                        "hash cambiado en {}: se omite por seguridad",
                        m.destino.display()
                    ));
                    continue;
                }
                Ok(_) => {}
                Err(e) => {
                    report
                        .errors
                        .push(format!("no se pudo leer {}: {}", m.destino.display(), e));
                    continue;
                }
            }
        }
        if let Some(parent) = m.origen.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                report.errors.push(format!("{}: {}", parent.display(), e));
                continue;
            }
        }
        if m.origen.exists() {
            report
                .errors
                .push(format!("ya existe {}: no se sobrescribe", m.origen.display()));
            continue;
        }
        match fs::rename(&m.destino, &m.origen) {
            Ok(()) => {
                m.revertido = true;
                report.restored += 1;
            }
            Err(e) => report.errors.push(format!(
                "{} -> {}: {}",
                m.destino.display(),
                m.origen.display(),
                e
            )),
        }
    }

    if report.restored > 0 {
        log.revertido_en = Some(mover::timestamp());
        mover::write_log(log_path, &log)?;
    }
    Ok(report)
}

/// Revierte la ejecución más reciente del historial local.
pub fn undo_latest() -> Result<UndoReport, String> {
    let log = crate::paths::latest_log()
        .ok_or_else(|| "no hay historial de ejecuciones para revertir".to_string())?;
    undo(&log)
}

/// Entrada del historial para mostrarla en el menú.
pub struct HistoryEntry {
    pub path: PathBuf,
    pub reverted: bool,
    pub fecha: String,
    pub movimientos: usize,
}

pub fn history() -> Vec<HistoryEntry> {
    let mut entries: Vec<HistoryEntry> = crate::paths::all_logs()
        .into_iter()
        .filter_map(|path| {
            let data = fs::read_to_string(&path).ok()?;
            let log: UndoLog = serde_json::from_str(&data).ok()?;
            Some(HistoryEntry {
                reverted: log.revertido_en.is_some(),
                fecha: log.fecha,
                movimientos: log.movimientos.len(),
                path,
            })
        })
        .collect();
    entries.sort_by(|a, b| b.fecha.cmp(&a.fecha));
    entries
}
