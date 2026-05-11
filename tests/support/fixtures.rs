#![allow(dead_code)]

use std::path::PathBuf;

#[derive(Clone, Copy, Debug)]
pub struct Fixture {
    pub language: &'static str,
    pub name: &'static str,
}

impl Fixture {
    pub fn root(self) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(self.language)
            .join(self.name)
    }

    pub fn coverage_json(self) -> PathBuf {
        self.root().join("coverage.json")
    }

    pub fn id(self) -> String {
        format!("{}/{}", self.language, self.name)
    }
}

pub fn rust_basic_fail_fixture() -> Fixture {
    Fixture {
        language: "rust",
        name: "basic-fail",
    }
}

pub fn rust_basic_pass_fixture() -> Fixture {
    Fixture {
        language: "rust",
        name: "basic-pass",
    }
}

pub fn cpp_basic_fail_fixture() -> Fixture {
    Fixture {
        language: "cpp",
        name: "basic-fail",
    }
}

pub fn cpp_basic_pass_fixture() -> Fixture {
    Fixture {
        language: "cpp",
        name: "basic-pass",
    }
}

pub fn swift_basic_fail_fixture() -> Fixture {
    Fixture {
        language: "swift",
        name: "basic-fail",
    }
}

pub fn swift_basic_pass_fixture() -> Fixture {
    Fixture {
        language: "swift",
        name: "basic-pass",
    }
}

pub fn dotnet_basic_fail_fixture() -> Fixture {
    Fixture {
        language: "dotnet",
        name: "basic-fail",
    }
}

pub fn dotnet_basic_pass_fixture() -> Fixture {
    Fixture {
        language: "dotnet",
        name: "basic-pass",
    }
}

pub fn dotnet_duplicate_lines_fixture() -> Fixture {
    Fixture {
        language: "dotnet",
        name: "duplicate-lines",
    }
}

pub fn vitest_basic_fail_fixture() -> Fixture {
    Fixture {
        language: "vitest",
        name: "basic-fail",
    }
}

pub fn vitest_basic_pass_fixture() -> Fixture {
    Fixture {
        language: "vitest",
        name: "basic-pass",
    }
}

pub fn vitest_statement_line_divergence_fixture() -> Fixture {
    Fixture {
        language: "vitest",
        name: "statement-line-divergence",
    }
}

pub fn vitest_empty_branch_locations_fixture() -> Fixture {
    Fixture {
        language: "vitest",
        name: "empty-branch-locations",
    }
}

pub fn vitest_tsx_line_summary_fixture() -> Fixture {
    Fixture {
        language: "vitest",
        name: "tsx-line-summary",
    }
}

pub fn vitest_path_scoped_gates_fixture() -> Fixture {
    Fixture {
        language: "vitest",
        name: "path-scoped-gates",
    }
}

pub fn fail_fixtures_with_regions() -> Vec<Fixture> {
    vec![
        rust_basic_fail_fixture(),
        cpp_basic_fail_fixture(),
        swift_basic_fail_fixture(),
    ]
}

pub fn pass_fixtures_with_regions() -> Vec<Fixture> {
    vec![
        rust_basic_pass_fixture(),
        cpp_basic_pass_fixture(),
        swift_basic_pass_fixture(),
    ]
}

pub fn branch_capable_fail_fixtures() -> Vec<Fixture> {
    vec![
        cpp_basic_fail_fixture(),
        dotnet_basic_fail_fixture(),
        vitest_basic_fail_fixture(),
    ]
}

pub fn branch_capable_pass_fixtures() -> Vec<Fixture> {
    vec![
        cpp_basic_pass_fixture(),
        dotnet_basic_pass_fixture(),
        vitest_basic_pass_fixture(),
    ]
}

pub fn fail_fixtures_with_lines() -> Vec<Fixture> {
    vec![
        rust_basic_fail_fixture(),
        cpp_basic_fail_fixture(),
        swift_basic_fail_fixture(),
        dotnet_basic_fail_fixture(),
        vitest_basic_fail_fixture(),
    ]
}

pub fn pass_fixtures_with_lines() -> Vec<Fixture> {
    vec![
        rust_basic_pass_fixture(),
        cpp_basic_pass_fixture(),
        swift_basic_pass_fixture(),
        dotnet_basic_pass_fixture(),
        vitest_basic_pass_fixture(),
    ]
}

pub fn function_capable_fail_fixtures() -> Vec<Fixture> {
    vec![
        rust_basic_fail_fixture(),
        cpp_basic_fail_fixture(),
        swift_basic_fail_fixture(),
        dotnet_basic_fail_fixture(),
        vitest_basic_fail_fixture(),
    ]
}

pub fn function_capable_pass_fixtures() -> Vec<Fixture> {
    vec![
        rust_basic_pass_fixture(),
        cpp_basic_pass_fixture(),
        swift_basic_pass_fixture(),
        dotnet_basic_pass_fixture(),
        vitest_basic_pass_fixture(),
    ]
}
