use crate::{cli::KeepRule, fingerprint::Fingerprint};
use std::fs;

pub fn choose_original(group: &[Fingerprint], rule: &KeepRule) -> usize {
    if group.is_empty() {
        return 0;
    }
    let mut best_idx = 0;
    for (i, fp) in group.iter().enumerate().skip(1) {
        let better = match rule {
            KeepRule::Oldest => {
                let a = fs::metadata(&group[best_idx].path).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let b = fs::metadata(&fp.path).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                b < a
            }
            KeepRule::Newest => {
                let a = fs::metadata(&group[best_idx].path).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let b = fs::metadata(&fp.path).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                b > a
            }
            KeepRule::ShortestPath => {
                let a_len = group[best_idx].path.to_string_lossy().len();
                let b_len = fp.path.to_string_lossy().len();
                b_len < a_len
            }
            KeepRule::FirstAlpha => {
                fp.path.to_string_lossy() < group[best_idx].path.to_string_lossy()
            }
        };
        if better {
            best_idx = i;
        }
    }
    best_idx
}
