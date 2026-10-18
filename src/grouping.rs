use crate::fingerprint::Fingerprint;
use crate::safety::is_same_file;
use std::collections::HashMap;

pub type DuplicateGroup = Vec<Fingerprint>;

/// Agrupa duplicados reales por contenido. Separa por tamaño y luego por hash
/// (BLAKE3). Descarta hardlinks/symlinks al mismo dato físico.
pub fn group_duplicates(fps: Vec<Fingerprint>) -> Vec<DuplicateGroup> {
    let mut by_size: HashMap<u64, Vec<Fingerprint>> = HashMap::new();
    for fp in fps {
        by_size.entry(fp.size).or_default().push(fp);
    }

    let mut groups = Vec::new();
    for items in by_size.into_values() {
        if items.len() < 2 {
            continue;
        }
        let mut by_hash: HashMap<blake3::Hash, Vec<Fingerprint>> = HashMap::new();
        for fp in items {
            by_hash.entry(fp.hash).or_default().push(fp);
        }
        for g in by_hash.into_values() {
            let distinct = dedupe_same_file(g);
            if distinct.len() > 1 {
                groups.push(distinct);
            }
        }
    }
    groups
}

/// Elimina del grupo los archivos que son el mismo dato físico (hardlinks).
fn dedupe_same_file(group: Vec<Fingerprint>) -> Vec<Fingerprint> {
    let mut kept: Vec<Fingerprint> = Vec::new();
    for fp in group {
        if kept.iter().any(|k| is_same_file(&k.path, &fp.path)) {
            continue;
        }
        kept.push(fp);
    }
    kept
}
