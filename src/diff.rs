use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Context, Result};

use crate::{
    git,
    model::{ChangedFile, LineRange},
};

#[derive(Debug, Clone)]
pub enum DiffSource {
    GitBase(String),
    DiffFile(PathBuf),
}

impl DiffSource {
    pub fn describe(&self) -> String {
        match self {
            Self::GitBase(base) => format!("{base}...HEAD, staged and unstaged changes"),
            Self::DiffFile(path) => path.display().to_string(),
        }
    }
}

pub fn load_changed_lines(source: &DiffSource) -> Result<Vec<ChangedFile>> {
    let text = match source {
        DiffSource::GitBase(base) => {
            let merge_base_sha = git::merge_base(base, "HEAD")?;
            git::diff_with_unified_zero(&merge_base_sha)?
        }
        DiffSource::DiffFile(path) => std::fs::read_to_string(path)
            .with_context(|| format!("failed to read diff file: {}", path.display()))?,
    };

    parse_unified_diff(&text)
}

pub fn parse_unified_diff(input: &str) -> Result<Vec<ChangedFile>> {
    let mut current_path: Option<PathBuf> = None;
    let mut current_file_is_deleted = false;
    let mut by_file: BTreeMap<PathBuf, Vec<LineRange>> = BTreeMap::new();

    for line in input.lines() {
        if let Some(rest) = line.strip_prefix("diff --git a/") {
            let (_, right) = rest
                .split_once(" b/")
                .context("malformed diff --git header")?;
            current_path = Some(PathBuf::from(right));
            current_file_is_deleted = false;
            continue;
        }

        if line == "+++ /dev/null" {
            current_file_is_deleted = true;
            continue;
        }

        if let Some(path) = line.strip_prefix("+++ b/") {
            current_path = Some(PathBuf::from(path));
            current_file_is_deleted = false;
            by_file.entry(PathBuf::from(path)).or_default();
            continue;
        }

        if let Some(rest) = line.strip_prefix("@@ ") {
            if current_file_is_deleted {
                continue;
            }
            let path = current_path
                .clone()
                .context("encountered hunk before file header")?;
            let plus_start = rest.find('+').context("missing added hunk marker")?;
            let plus = &rest[plus_start + 1..];
            let range_end = plus.find(' ').unwrap_or(plus.len());
            let range = &plus[..range_end];
            let (start, count) = parse_range(range)?;
            if count == 0 {
                continue;
            }
            by_file.entry(path).or_default().push(LineRange {
                start,
                end: start + count - 1,
            });
        }
    }

    Ok(by_file
        .into_iter()
        .map(|(path, changed_lines)| ChangedFile {
            path,
            changed_lines,
        })
        .collect())
}

fn parse_range(input: &str) -> Result<(u32, u32)> {
    let (start, count) = if let Some((start, count)) = input.split_once(',') {
        (start, count)
    } else {
        (input, "1")
    };
    Ok((
        start.parse().context("invalid hunk start line")?,
        count.parse().context("invalid hunk count")?,
    ))
}

#[cfg(test)]
mod tests {
    use super::parse_unified_diff;

    #[test]
    fn parses_added_hunks() {
        let input = "\
diff --git a/src/lib.rs b/src/lib.rs\n\
+++ b/src/lib.rs\n\
@@ -1,0 +2,3 @@\n";

        let changed = parse_unified_diff(input).expect("diff should parse");
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].path, std::path::PathBuf::from("src/lib.rs"));
        assert_eq!(changed[0].changed_lines[0].start, 2);
        assert_eq!(changed[0].changed_lines[0].end, 4);
    }

    #[test]
    fn ignores_deleted_only_hunks() {
        let input = "\
diff --git a/src/lib.rs b/src/lib.rs\n\
+++ b/src/lib.rs\n\
@@ -4,2 +4,0 @@\n";

        let changed = parse_unified_diff(input).expect("diff should parse");
        assert_eq!(changed.len(), 1);
        assert!(changed[0].changed_lines.is_empty());
    }

    #[test]
    fn ignores_deleted_file_headers_before_tracked_files() {
        let input = "\
diff --git a/src/deleted.rs b/src/deleted.rs\n\
+++ /dev/null\n\
@@ -1,2 +0,0 @@\n\
diff --git a/src/lib.rs b/src/lib.rs\n\
+++ b/src/lib.rs\n\
@@ -1,0 +2,2 @@\n";

        let changed = parse_unified_diff(input).expect("diff should parse");
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].path, std::path::PathBuf::from("src/lib.rs"));
        assert_eq!(changed[0].changed_lines[0].start, 2);
        assert_eq!(changed[0].changed_lines[0].end, 3);
    }
}
