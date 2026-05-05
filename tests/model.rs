use covgate::model::{MetricKind, OpportunityKind, SourceSpan, SpanKey};
use std::path::PathBuf;

#[test]
fn parses_function_metric_kind() {
    let metric: MetricKind = serde_json::from_str("\"function\"").expect("function should parse");
    assert_eq!(metric, MetricKind::Function);
    assert_eq!(metric.as_str(), "function");
    assert_eq!(metric.label(), "functions");
    assert_eq!(metric.to_opportunity_kind(), OpportunityKind::Function);
}

#[test]
fn formats_spans_with_and_without_columns() {
    let path = PathBuf::from("src/lib.rs");

    // Single line, no columns
    let s1 = SourceSpan {
        path: path.clone(),
        start_line: 10,
        end_line: 10,
        start_col: None,
        end_col: None,
    };
    assert_eq!(s1.format_span(), "10");
    assert_eq!(s1.display(), "src/lib.rs:10");

    // Multi line, no columns
    let s2 = SourceSpan {
        path: path.clone(),
        start_line: 10,
        end_line: 12,
        start_col: None,
        end_col: None,
    };
    assert_eq!(s2.format_span(), "10-12");

    // Single line, single column
    let s3 = SourceSpan {
        path: path.clone(),
        start_line: 10,
        end_line: 10,
        start_col: Some(5),
        end_col: Some(5),
    };
    assert_eq!(s3.format_span(), "10:5");

    // Single line, column range
    let s4 = SourceSpan {
        path: path.clone(),
        start_line: 10,
        end_line: 10,
        start_col: Some(5),
        end_col: Some(15),
    };
    assert_eq!(s4.format_span(), "10:5-15");

    // Multi line, column range
    let s5 = SourceSpan {
        path: path.clone(),
        start_line: 10,
        end_line: 11,
        start_col: Some(5),
        end_col: Some(10),
    };
    assert_eq!(s5.format_span(), "10:5-11:10");
}

#[test]
fn groups_spans_and_counts_occurrences() {
    let path = PathBuf::from("src/lib.rs");
    let s1 = SourceSpan {
        path: path.clone(),
        start_line: 10,
        end_line: 10,
        start_col: None,
        end_col: None,
    };
    let s2 = s1.clone();
    let s3 = SourceSpan {
        path: path.clone(),
        start_line: 20,
        end_line: 20,
        start_col: None,
        end_col: None,
    };

    let groups = SourceSpan::group_by_span(&[s1, s2, s3]);
    assert_eq!(groups.len(), 2);
    assert_eq!(
        groups[&SpanKey {
            start_line: 10,
            end_line: 10,
            start_col: None,
            end_col: None
        }],
        2
    );
    assert_eq!(
        groups[&SpanKey {
            start_line: 20,
            end_line: 20,
            start_col: None,
            end_col: None
        }],
        1
    );
}
