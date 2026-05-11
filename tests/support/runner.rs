#![allow(dead_code)]

use std::{
    path::Path,
    process::{Command, Output},
};

pub fn run_covgate(
    worktree: &Path,
    coverage_json: &Path,
    extra_args: &[String],
    env_vars: &[(&str, &str)],
) -> Output {
    let binary = env!("CARGO_BIN_EXE_covgate");
    let mut command = Command::new(binary);
    command.env_clear();
    command.env("PATH", std::env::var_os("PATH").unwrap_or_default());
    command.envs(env_vars.iter().copied());
    command.arg("check");
    command.arg(coverage_json);
    command.args(extra_args);
    command.current_dir(worktree);
    command.output().expect("covgate should run")
}

pub fn run_covgate_raw(worktree: &Path, args: &[String], env_vars: &[(&str, &str)]) -> Output {
    let binary = env!("CARGO_BIN_EXE_covgate");
    let mut command = Command::new(binary);
    command.env_clear();
    command.env("PATH", std::env::var_os("PATH").unwrap_or_default());
    command.envs(env_vars.iter().copied());
    command.args(args);
    command.current_dir(worktree);
    command.output().expect("covgate should run")
}
