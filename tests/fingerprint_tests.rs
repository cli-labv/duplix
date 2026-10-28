use std::fs;
use tempfile::tempdir;

#[test]
fn test_duplicate_detection() {
    let dir = tempdir().unwrap();
    let f1 = dir.path().join("a.txt");
    let f2 = dir.path().join("b.txt");
    let f3 = dir.path().join("c.txt");
    fs::write(&f1, b"hello").unwrap();
    fs::write(&f2, b"hello").unwrap();
    fs::write(&f3, b"world").unwrap();

    let paths = vec![f1, f2, f3];
    let fps = duplix::fingerprint::fingerprint_files(&paths, None);
    let groups = duplix::grouping::group_duplicates(fps);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].len(), 2);
}

#[test]
fn test_original_rule() {
    use duplix::{cli::KeepRule, fingerprint::Fingerprint, original_rule::choose_original};
    let tmp = tempdir().unwrap();
    let a = tmp.path().join("a.txt");
    let b = tmp.path().join("b.txt");
    fs::write(&a, b"same").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(&b, b"same").unwrap();

    let h = duplix::fingerprint::hash_file(&a).unwrap();
    let fps = vec![
        Fingerprint {
            path: a.clone(),
            size: 4,
            hash: h,
        },
        Fingerprint {
            path: b.clone(),
            size: 4,
            hash: h,
        },
    ];
    assert_eq!(choose_original(&fps, &KeepRule::Oldest), 0);
    assert_eq!(choose_original(&fps, &KeepRule::Newest), 1);
}
