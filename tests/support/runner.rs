#![allow(dead_code)]

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub fn covgate(worktree: &Path) -> CovgateCommand {
    CovgateCommand {
        worktree: worktree.to_path_buf(),
        args: Vec::new(),
        envs: Vec::new(),
    }
}

pub struct CovgateCommand {
    worktree: PathBuf,
    args: Vec<OsString>,
    envs: Vec<(OsString, OsString)>,
}

impl CovgateCommand {
    pub fn check(mut self, coverage_json: &Path) -> Self {
        self.args.push("check".into());
        self.args.push(coverage_json.into());
        self
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.envs.push((key.into(), value.into()));
        self
    }

    pub fn run(self) -> Output {
        let binary = env!("CARGO_BIN_EXE_covgate");
        let mut command = Command::new(binary);
        command.env_clear();
        command.env("PATH", std::env::var_os("PATH").unwrap_or_default());
        if let Some(profile_file) = std::env::var_os("LLVM_PROFILE_FILE") {
            command.env("LLVM_PROFILE_FILE", profile_file);
        }
        command.envs(self.envs);
        command.args(self.args);
        command.current_dir(self.worktree);
        command.output().expect("covgate should run")
    }
}
