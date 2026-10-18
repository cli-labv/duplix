use rayon::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fingerprint {
    pub path: PathBuf,
    pub size: u64,
    pub hash: blake3::Hash,
}

pub fn hash_file(path: &Path) -> std::io::Result<blake3::Hash> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize())
}

/// Calcula la huella de cada archivo en paralelo con BLAKE3.
/// `counter`, si se pasa, se incrementa por cada archivo procesado (progreso).
pub fn fingerprint_files(
    paths: &[PathBuf],
    counter: Option<Arc<AtomicUsize>>,
) -> Vec<Fingerprint> {
    paths
        .par_iter()
        .filter_map(|path| {
            let size = std::fs::metadata(path).ok()?.len();
            let hash = hash_file(path).ok()?;
            if let Some(c) = &counter {
                c.fetch_add(1, Ordering::Relaxed);
            }
            Some(Fingerprint {
                path: path.clone(),
                size,
                hash,
            })
        })
        .collect()
}


