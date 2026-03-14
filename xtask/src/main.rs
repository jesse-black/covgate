use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(task) = args.next() else {
        bail!(
            "usage: cargo xtask <task>\n\n  validate\n  regen-fixture-coverage <language>/<scenario>\n  regen-fixture-coverage-all"
        );
    };

    match task.as_str() {
        "validate" => validate(),
        "regen-fixture-coverage" => {
            let Some(fixture_id) = args.next() else {
                bail!("usage: cargo xtask regen-fixture-coverage <language>/<scenario>");
            };
            regen_fixture_coverage(&fixture_id)
        }
        "regen-fixture-coverage-all" => regen_fixture_coverage_all(),
        _ => bail!("unknown xtask `{task}`"),
    }
}

fn validate() -> Result<()> {
    run("cargo", &["fmt", "--check"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;

    let coverage_json = coverage_path();
    let coverage_json_str = coverage_json
        .to_str()
        .context("coverage output path contained non-utf8 characters")?;

    run(
        "cargo",
        &[
            "llvm-cov",
            "--json",
            "--output-path",
            coverage_json_str,
            "--fail-under-regions=88",
        ],
    )?;

    if let Some(base_ref) = resolve_base_ref() {
        run(
            "cargo",
            &[
                "run",
                "--bin",
                "covgate",
                "--",
                "--coverage-json",
                coverage_json_str,
                "--base",
                &base_ref,
            ],
        )?;
    } else {
        eprintln!(
            "warning: skipping covgate dogfooding step; no suitable git base ref was found in this environment"
        );
    }

    run("cargo-machete", &["."])?;
    run("cargo-deny", &["check"])?;

    std::fs::remove_file(&coverage_json).ok();
    Ok(())
}

#[derive(Clone, Copy)]
enum FixtureToolchain {
    Rust,
    Cpp,
    Swift,
    Dotnet,
}

#[derive(Clone, Copy)]
enum RunMode {
    NoCalls,
    PositiveOnly,
    PositiveAndNegative,
}

#[derive(Clone, Copy)]
struct FixtureCoverageSpec {
    id: &'static str,
    source_file: &'static str,
    toolchain: FixtureToolchain,
    run_mode: RunMode,
}

const FIXTURE_COVERAGE_SPECS: &[FixtureCoverageSpec] = &[
    FixtureCoverageSpec {
        id: "rust/basic-fail",
        source_file: "src/lib.rs",
        toolchain: FixtureToolchain::Rust,
        run_mode: RunMode::NoCalls,
    },
    FixtureCoverageSpec {
        id: "rust/basic-pass",
        source_file: "src/lib.rs",
        toolchain: FixtureToolchain::Rust,
        run_mode: RunMode::PositiveOnly,
    },
    FixtureCoverageSpec {
        id: "cpp/basic-fail",
        source_file: "src/lib.cpp",
        toolchain: FixtureToolchain::Cpp,
        run_mode: RunMode::NoCalls,
    },
    FixtureCoverageSpec {
        id: "cpp/basic-pass",
        source_file: "src/lib.cpp",
        toolchain: FixtureToolchain::Cpp,
        run_mode: RunMode::PositiveAndNegative,
    },
    FixtureCoverageSpec {
        id: "swift/basic-fail",
        source_file: "Sources/CovgateDemo/CovgateDemo.swift",
        toolchain: FixtureToolchain::Swift,
        run_mode: RunMode::NoCalls,
    },
    FixtureCoverageSpec {
        id: "swift/basic-pass",
        source_file: "Sources/CovgateDemo/CovgateDemo.swift",
        toolchain: FixtureToolchain::Swift,
        run_mode: RunMode::PositiveAndNegative,
    },
    FixtureCoverageSpec {
        id: "dotnet/basic-fail",
        source_file: "src/CovgateDemo/MathOps.cs",
        toolchain: FixtureToolchain::Dotnet,
        run_mode: RunMode::NoCalls,
    },
    FixtureCoverageSpec {
        id: "dotnet/basic-pass",
        source_file: "src/CovgateDemo/MathOps.cs",
        toolchain: FixtureToolchain::Dotnet,
        run_mode: RunMode::NoCalls,
    },
];

fn regen_fixture_coverage(fixture_id: &str) -> Result<()> {
    let spec = FIXTURE_COVERAGE_SPECS
        .iter()
        .find(|spec| spec.id == fixture_id)
        .ok_or_else(|| anyhow::anyhow!("unknown fixture `{fixture_id}`"))?;
    write_fixture_coverage(spec)
}

fn regen_fixture_coverage_all() -> Result<()> {
    for spec in FIXTURE_COVERAGE_SPECS {
        write_fixture_coverage(spec)?;
    }
    Ok(())
}

fn write_fixture_coverage(spec: &FixtureCoverageSpec) -> Result<()> {
    if matches!(spec.toolchain, FixtureToolchain::Dotnet) {
        return write_dotnet_fixture_coverage(spec);
    }

    let repo_root = project_root()?;
    let source_path = repo_root
        .join("tests")
        .join("fixtures")
        .join(spec.id)
        .join("overlay")
        .join(spec.source_file);
    let output_path = repo_root
        .join("tests")
        .join("fixtures")
        .join(spec.id)
        .join("coverage.json");

    let temp_dir = std::env::temp_dir().join(format!(
        "covgate-xtask-fixture-{}-{}",
        spec.id.replace('/', "-"),
        chrono_like_timestamp()
    ));
    std::fs::create_dir_all(&temp_dir)
        .with_context(|| format!("failed to create temp dir: {}", temp_dir.display()))?;

    let binary_path = temp_dir.join("fixture-bin");
    let profraw_path = temp_dir.join("fixture.profraw");
    let profdata_path = temp_dir.join("fixture.profdata");
    let exported_path = temp_dir.join("exported.json");

    build_fixture_binary(spec, &source_path, &binary_path)?;

    run_env(
        binary_path
            .to_str()
            .context("binary path contained non-utf8 characters")?,
        &[],
        &[&(
            "LLVM_PROFILE_FILE".to_string(),
            profraw_path
                .to_str()
                .context("profraw path contained non-utf8 characters")?
                .to_string(),
        )],
    )?;

    run(
        "llvm-profdata",
        &[
            "merge",
            "-sparse",
            profraw_path
                .to_str()
                .context("profraw path contained non-utf8 characters")?,
            "-o",
            profdata_path
                .to_str()
                .context("profdata path contained non-utf8 characters")?,
        ],
    )?;

    run_to_file(
        "llvm-cov",
        &[
            "export",
            binary_path
                .to_str()
                .context("binary path contained non-utf8 characters")?,
            "-instr-profile",
            profdata_path
                .to_str()
                .context("profdata path contained non-utf8 characters")?,
            source_path
                .to_str()
                .context("source path contained non-utf8 characters")?,
        ],
        &exported_path,
    )?;

    normalize_exported_coverage(&exported_path, spec.source_file, &output_path)?;

    std::fs::remove_dir_all(&temp_dir).ok();
    eprintln!("updated {}", output_path.display());
    Ok(())
}

fn build_fixture_binary(
    spec: &FixtureCoverageSpec,
    source_path: &Path,
    binary_path: &Path,
) -> Result<()> {
    match spec.toolchain {
        FixtureToolchain::Rust => build_rust_fixture_binary(spec, source_path, binary_path),
        FixtureToolchain::Cpp => build_cpp_fixture_binary(spec, source_path, binary_path),
        FixtureToolchain::Swift => build_swift_fixture_binary(spec, source_path, binary_path),
        FixtureToolchain::Dotnet => {
            bail!("dotnet fixtures do not support llvm fixture binary generation")
        }
    }
}

fn write_dotnet_fixture_coverage(spec: &FixtureCoverageSpec) -> Result<()> {
    let repo_root = project_root()?;
    let fixture_root = repo_root.join("tests").join("fixtures").join(spec.id);
    let temp_dir = std::env::temp_dir().join(format!(
        "covgate-xtask-fixture-{}-{}",
        spec.id.replace('/', "-"),
        chrono_like_timestamp()
    ));
    std::fs::create_dir_all(&temp_dir)
        .with_context(|| format!("failed to create temp dir: {}", temp_dir.display()))?;

    copy_tree(&fixture_root.join("repo"), &temp_dir)?;
    copy_tree(&fixture_root.join("overlay"), &temp_dir)?;

    let test_project = temp_dir
        .join("tests")
        .join("CovgateDemo.Tests")
        .join("CovgateDemo.Tests.csproj");
    run(
        "dotnet",
        &[
            "test",
            test_project
                .to_str()
                .context("dotnet test project path contained non-utf8 characters")?,
            "--collect:XPlat Code Coverage;Format=json",
        ],
    )?;

    let raw_coverage = find_coverage_json(&temp_dir)?;
    let output_path = fixture_root.join("coverage.json");
    normalize_coverlet_coverage(&raw_coverage, spec.source_file, &output_path)?;

    std::fs::remove_dir_all(&temp_dir).ok();
    eprintln!("updated {}", output_path.display());
    Ok(())
}

fn build_rust_fixture_binary(
    spec: &FixtureCoverageSpec,
    source_path: &Path,
    binary_path: &Path,
) -> Result<()> {
    let driver = binary_path.with_extension("rs");
    let include_path = source_path
        .to_str()
        .context("rust source path contained non-utf8 characters")?;
    let body = match spec.run_mode {
        RunMode::NoCalls => String::new(),
        RunMode::PositiveOnly => "    let _ = fixture_lib::add(1, 2);\n".to_string(),
        RunMode::PositiveAndNegative => {
            "    let _ = fixture_lib::add(1, 2);\n    let _ = fixture_lib::add(-1, 2);\n"
                .to_string()
        }
    };
    std::fs::write(
        &driver,
        format!(
            "mod fixture_lib {{\n    include!(\"{include_path}\");\n}}\n\nfn main() {{\n{body}}}\n"
        ),
    )
    .with_context(|| format!("failed to write rust driver: {}", driver.display()))?;

    run(
        "rustc",
        &[
            "-C",
            "instrument-coverage",
            "-C",
            "link-dead-code",
            "-C",
            "codegen-units=1",
            "-C",
            "opt-level=0",
            driver
                .to_str()
                .context("rust driver path contained non-utf8 characters")?,
            "-o",
            binary_path
                .to_str()
                .context("rust binary path contained non-utf8 characters")?,
        ],
    )
}

fn build_cpp_fixture_binary(
    spec: &FixtureCoverageSpec,
    source_path: &Path,
    binary_path: &Path,
) -> Result<()> {
    let driver = binary_path.with_extension("cpp");
    let body = match spec.run_mode {
        RunMode::NoCalls => String::new(),
        RunMode::PositiveOnly => "    (void)add(1, 2);\n".to_string(),
        RunMode::PositiveAndNegative => "    (void)add(1, 2);\n    (void)add(-1, 2);\n".to_string(),
    };
    std::fs::write(
        &driver,
        format!("int add(int, int);\n\nint main() {{\n{body}    return 0;\n}}\n"),
    )
    .with_context(|| format!("failed to write cpp driver: {}", driver.display()))?;

    run(
        "clang++",
        &[
            "-fprofile-instr-generate",
            "-fcoverage-mapping",
            source_path
                .to_str()
                .context("cpp source path contained non-utf8 characters")?,
            driver
                .to_str()
                .context("cpp driver path contained non-utf8 characters")?,
            "-o",
            binary_path
                .to_str()
                .context("cpp binary path contained non-utf8 characters")?,
        ],
    )
}

fn build_swift_fixture_binary(
    spec: &FixtureCoverageSpec,
    source_path: &Path,
    binary_path: &Path,
) -> Result<()> {
    let driver = binary_path.with_extension("swift");
    let body = match spec.run_mode {
        RunMode::NoCalls => String::new(),
        RunMode::PositiveOnly => "        _ = add(1, 2)\n".to_string(),
        RunMode::PositiveAndNegative => {
            "        _ = add(1, 2)\n        _ = add(-1, 2)\n".to_string()
        }
    };
    std::fs::write(
        &driver,
        format!("@main\nstruct Runner {{\n    static func main() {{\n{body}    }}\n}}\n"),
    )
    .with_context(|| format!("failed to write swift driver: {}", driver.display()))?;

    run(
        "swiftc",
        &[
            "-profile-generate",
            "-profile-coverage-mapping",
            source_path
                .to_str()
                .context("swift source path contained non-utf8 characters")?,
            driver
                .to_str()
                .context("swift driver path contained non-utf8 characters")?,
            "-o",
            binary_path
                .to_str()
                .context("swift binary path contained non-utf8 characters")?,
        ],
    )
}

fn normalize_exported_coverage(exported: &Path, source_file: &str, output: &Path) -> Result<()> {
    let text = std::fs::read_to_string(exported)
        .with_context(|| format!("failed to read exported coverage: {}", exported.display()))?;
    let mut value: serde_json::Value =
        serde_json::from_str(&text).context("failed to parse llvm-cov exported json")?;

    if let Some(files) = value
        .get_mut("data")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|data| data.first_mut())
        .and_then(|first| first.get_mut("files"))
        .and_then(serde_json::Value::as_array_mut)
    {
        for file in files {
            if let Some(filename) = file.get_mut("filename") {
                *filename = serde_json::Value::String(source_file.to_string());
            }
        }
    }

    let pretty = serde_json::to_string_pretty(&value).context("failed to format json")?;
    std::fs::write(output, format!("{pretty}\n"))
        .with_context(|| format!("failed to write fixture coverage: {}", output.display()))
}

fn normalize_coverlet_coverage(raw: &Path, source_file: &str, output: &Path) -> Result<()> {
    let text = std::fs::read_to_string(raw)
        .with_context(|| format!("failed to read raw coverlet json: {}", raw.display()))?;
    let mut modules: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&text).context("failed to parse coverlet json")?;

    for module in modules.values_mut() {
        let Some(files) = module.as_object_mut() else {
            continue;
        };
        let mut rewritten = serde_json::Map::new();
        for (file, value) in std::mem::take(files) {
            let normalized = file.replace('\\', "/");
            let key = if normalized.ends_with(source_file) {
                source_file.to_string()
            } else {
                normalized
            };
            rewritten.insert(key, value);
        }
        *files = rewritten;
    }

    let pretty = serde_json::to_string_pretty(&modules).context("failed to format json")?;
    std::fs::write(output, format!("{pretty}\n"))
        .with_context(|| format!("failed to write fixture coverage: {}", output.display()))
}

fn find_coverage_json(root: &Path) -> Result<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)
            .with_context(|| format!("failed to read directory: {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|file| file == "coverage.json")
                && path
                    .components()
                    .any(|component| component.as_os_str() == "TestResults")
            {
                return Ok(path);
            }
        }
    }

    bail!(
        "failed to locate coverlet coverage.json under {}",
        root.display()
    )
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)
        .with_context(|| format!("failed to create directory: {}", destination.display()))?;
    for entry in std::fs::read_dir(source)
        .with_context(|| format!("failed to read directory: {}", source.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest).with_context(|| {
                format!(
                    "failed to copy {} -> {}",
                    entry.path().display(),
                    dest.display()
                )
            })?;
        }
    }
    Ok(())
}

fn project_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .context("xtask manifest should live under the repository root")?;
    Ok(root.to_path_buf())
}

fn resolve_base_ref() -> Option<String> {
    for candidate in ["origin/main", "origin/master", "main", "master"] {
        if git_ref_exists(candidate) {
            return Some(candidate.to_owned());
        }
    }

    fetch_probable_default_branches();

    for candidate in [
        "origin/main",
        "origin/master",
        "refs/remotes/origin/main",
        "refs/remotes/origin/master",
        "main",
        "master",
    ] {
        if git_ref_exists(candidate) {
            return Some(candidate.to_owned());
        }
    }

    None
}

fn fetch_probable_default_branches() {
    for branch in ["main", "master"] {
        let _ = Command::new("git")
            .args([
                "fetch",
                "--no-tags",
                "--depth=200",
                "origin",
                &format!("{branch}:refs/remotes/origin/{branch}"),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    let _ = Command::new("git")
        .args(["fetch", "--no-tags", "--depth=200", "origin"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn git_ref_exists(reference: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", reference])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn coverage_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "covgate-xtask-validate-{}-{}.json",
        std::process::id(),
        chrono_like_timestamp()
    ));
    path
}

fn chrono_like_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn run(program: &str, args: &[&str]) -> Result<()> {
    eprintln!("> {} {}", program, args.join(" "));
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute `{program}`"))?;

    if !status.success() {
        bail!(
            "command `{program} {}` failed with status {status}",
            args.join(" ")
        );
    }

    Ok(())
}

fn run_env(program: &str, args: &[&str], envs: &[&(String, String)]) -> Result<()> {
    eprintln!("> {} {}", program, args.join(" "));
    let mut command = Command::new(program);
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }

    let status = command
        .status()
        .with_context(|| format!("failed to execute `{program}`"))?;
    if !status.success() {
        bail!(
            "command `{program} {}` failed with status {status}",
            args.join(" ")
        );
    }
    Ok(())
}

fn run_to_file(program: &str, args: &[&str], destination: &Path) -> Result<()> {
    eprintln!(
        "> {} {} > {}",
        program,
        args.join(" "),
        destination.display()
    );
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute `{program}`"))?;

    if !output.status.success() {
        bail!(
            "command `{program} {}` failed with status {}: {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    std::fs::write(destination, output.stdout).with_context(|| {
        format!(
            "failed to write command output to {}",
            destination.display()
        )
    })
}
