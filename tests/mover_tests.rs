use duplix::fingerprint::Fingerprint;
use duplix::mover::{build_moves, execute_moves, UndoLog};
use duplix::undo::undo;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn fp(path: PathBuf, size: u64, hash: blake3::Hash) -> Fingerprint {
    Fingerprint { path, size, hash }
}

fn hash_of(path: &Path) -> blake3::Hash {
    duplix::fingerprint::hash_file(path).unwrap()
}

#[test]
fn test_moves_and_undo_roundtrip() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let a = root.join("a.txt");
    let b = root.join("b.txt");
    fs::write(&a, b"same-content").unwrap();
    fs::write(&b, b"same-content").unwrap();
    let h = hash_of(&a);

    let groups = vec![vec![fp(a.clone(), 12, h), fp(b.clone(), 12, h)]];
    let dest_dupe = root.join("duplicate");
    let dest_keep = root.join("keep");

    let plan = build_moves(&groups, &[0], &dest_dupe, &dest_keep);
    assert_eq!(plan.moves.len(), 2, "1 conservado + 1 duplicado");
    assert_eq!(plan.freed, 12);
    assert_eq!(plan.duplicates(), 1);
    assert_eq!(plan.kept_count(), 1);
    assert!(plan
        .moves
        .iter()
        .any(|m| m.kept && m.destino == dest_keep.join("a.txt")));
    assert!(plan
        .moves
        .iter()
        .any(|m| !m.kept && m.destino == dest_dupe.join("b.txt")));

    let log = UndoLog {
        fecha: "0".into(),
        revertido_en: None,
        carpeta_escaneada: root.to_path_buf(),
        carpeta_duplicados: dest_dupe.clone(),
        movimientos: plan.moves,
    };
    let log_path = root.join("undo.json");
    let report = execute_moves(&log, &log_path, |_, _| {}).unwrap();
    assert_eq!(report.moved, 2);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(!a.exists());
    assert!(!b.exists());
    assert!(dest_keep.join("a.txt").exists());
    assert!(dest_dupe.join("b.txt").exists());

    let undo_report = undo(&log_path).unwrap();
    assert_eq!(undo_report.restored, 2, "{:?}", undo_report.errors);
    assert!(a.exists(), "el conservado debe volver a su origen");
    assert!(b.exists(), "el duplicado debe volver a su origen");
    assert!(!dest_keep.join("a.txt").exists());
    assert!(!dest_dupe.join("b.txt").exists());
}

#[test]
fn test_dest_collision_gets_suffix() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let dest_dupe = root.join("duplicate");
    let dest_keep = root.join("keep");
    fs::create_dir_all(&dest_dupe).unwrap();
    fs::write(dest_dupe.join("img.jpg"), b"already here").unwrap();

    let a = root.join("sub1").join("img.jpg");
    let b = root.join("sub2").join("img.jpg");
    fs::create_dir_all(a.parent().unwrap()).unwrap();
    fs::create_dir_all(b.parent().unwrap()).unwrap();
    fs::write(&a, b"dup").unwrap();
    fs::write(&b, b"dup").unwrap();
    let h = hash_of(&a);

    let groups = vec![vec![fp(a, 3, h), fp(b, 3, h)]];
    let plan = build_moves(&groups, &[1], &dest_dupe, &dest_keep);
    let dupe = plan.moves.iter().find(|m| !m.kept).unwrap();
    assert_eq!(dupe.destino, dest_dupe.join("img (1).jpg"));
}

#[test]
fn test_skipped_group_not_moved() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let a = root.join("a");
    let b = root.join("b");
    fs::write(&a, b"x").unwrap();
    fs::write(&b, b"x").unwrap();
    let h = hash_of(&a);

    let groups = vec![vec![fp(a, 1, h), fp(b, 1, h)]];
    let plan = build_moves(&groups, &[usize::MAX], &root.join("duplicate"), &root.join("keep"));
    assert!(plan.moves.is_empty());
}
