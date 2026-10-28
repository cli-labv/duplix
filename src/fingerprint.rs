use rayon::prelude::*;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fingerprint {
    pub path: PathBuf,
    pub size: u64,
    pub hash: blake3::Hash,
}

#[derive(Debug, Clone)]
pub struct PreFingerprint {
    pub path: PathBuf,
    pub size: u64,
    pub pre_hash: u64,
}

pub fn hash_file(path: &Path) -> std::io::Result<blake3::Hash> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize())
}

pub fn pre_hash_file(path: &Path) -> std::io::Result<u64> {
    let mut file = File::open(path)?;
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut buf = [0u8; 8192];
    let n = file.read(&mut buf)?;
    hasher.update(&buf[..n]);
    Ok(hasher.digest())
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

/// Pre-fingerprint rápido: tamaño + xxhash de los primeros 8 KB.
pub fn pre_fingerprint_files(
    paths: &[PathBuf],
    counter: Option<Arc<AtomicUsize>>,
) -> Vec<PreFingerprint> {
    paths
        .par_iter()
        .filter_map(|path| {
            let size = std::fs::metadata(path).ok()?.len();
            let pre_hash = pre_hash_file(path).ok()?;
            if let Some(c) = &counter {
                c.fetch_add(1, Ordering::Relaxed);
            }
            Some(PreFingerprint {
                path: path.clone(),
                size,
                pre_hash,
            })
        })
        .collect()
}


