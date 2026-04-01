use std::path::{Path, PathBuf};

pub(super) fn lexical_normalize(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().components().collect()
}

pub(super) fn relativize_absolute_path(path: &Path, repo_root: &Path) -> PathBuf {
    let normalized_path = lexical_normalize(path);
    let normalized_repo_root = lexical_normalize(repo_root);

    if normalized_path.is_absolute() {
        normalized_path
            .strip_prefix(&normalized_repo_root)
            .map(lexical_normalize)
            .unwrap_or(normalized_path)
    } else {
        normalized_path
    }
}
