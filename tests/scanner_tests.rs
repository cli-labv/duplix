use duplix::scanner::{scan, ScanOptions};
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tempfile::tempdir;

fn count(opts: ScanOptions, root: &Path) -> usize {
    scan(root, opts, Arc::new(AtomicUsize::new(0))).len()
}

#[cfg(unix)]
#[test]
fn test_symlinks_not_followed_by_default() {
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let outside_file = outside.path().join("secret.txt");
    std::fs::write(&outside_file, b"outside").unwrap();

    let inner = dir.path().join("inner");
    std::fs::create_dir_all(&inner).unwrap();
    std::fs::write(dir.path().join("real.txt"), b"real").unwrap();
    std::os::unix::fs::symlink(&outside_file, inner.join("link.txt")).unwrap();
    std::fs::write(inner.join("inside.txt"), b"inside").unwrap();

    let opts = ScanOptions {
        min_size: 0,
        exclude: Vec::new(),
        follow_links: false,
    };
    assert_eq!(count(opts, dir.path()), 2, "el symlink no debe contarse");

    let opts = ScanOptions {
        min_size: 0,
        exclude: Vec::new(),
        follow_links: true,
    };
    assert_eq!(count(opts, dir.path()), 3, "con follow_links se incluye");
}

#[test]
fn test_exclude_any_depth() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("node_modules").join("deep");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(dir.path().join("a.txt"), b"a").unwrap();
    std::fs::write(sub.join("b.txt"), b"b").unwrap();

    let opts = ScanOptions {
        min_size: 0,
        exclude: vec!["node_modules".to_string()],
        follow_links: false,
    };
    assert_eq!(count(opts, dir.path()), 1);
}
