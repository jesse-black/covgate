use covgate::diff::parse_unified_diff;
use std::path::PathBuf;

#[test]
fn parses_added_hunks() {
    let input = "\
diff --git a/src/lib.rs b/src/lib.rs\n\
+++ b/src/lib.rs\n\
@@ -1,0 +2,3 @@\n";

    let changed = parse_unified_diff(input).expect("diff should parse");
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].path, PathBuf::from("src/lib.rs"));
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
    assert_eq!(changed[0].path, PathBuf::from("src/lib.rs"));
    assert_eq!(changed[0].changed_lines[0].start, 2);
    assert_eq!(changed[0].changed_lines[0].end, 3);
}
