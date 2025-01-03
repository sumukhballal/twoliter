#![cfg(test)]

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

mod twoliter_build;
mod twoliter_update;

pub const TWOLITER_PATH: &'static str = env!("CARGO_BIN_FILE_TWOLITER");
pub const KRANE_STATIC_PATH: &'static str = env!("CARGO_BIN_FILE_KRANE_STATIC");

pub fn test_projects_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.join("projects")
}

pub fn run_command<I, S, E>(cmd: S, args: I, env: E) -> std::process::Output
where
    I: IntoIterator<Item = S>,
    E: IntoIterator<Item = (S, S)>,
    S: AsRef<OsStr>,
{
    let args: Vec<S> = args.into_iter().collect();

    println!(
        "Executing '{}' with args [{}]",
        cmd.as_ref().to_string_lossy(),
        args.iter()
            .map(|arg| format!("'{}'", arg.as_ref().to_string_lossy()))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let output = Command::new(cmd)
        .args(args.into_iter())
        .envs(env.into_iter())
        .output()
        .expect("failed to execute process");

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    output
}

struct KitRegistry {
    _temp_dir: TempDir,
    child: std::process::Child,
}

impl KitRegistry {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("failed to create path for oci registry spinup");

        let child = Command::new(KRANE_STATIC_PATH)
            .args(&[
                "registry",
                "serve",
                "--address",
                "127.0.0.1:5000",
                "--disk",
                temp_dir.path().to_str().unwrap(),
            ])
            .spawn()
            .expect("Failed to spawn registry process");

        Self {
            _temp_dir: temp_dir,
            child,
        }
    }
}

impl Drop for KitRegistry {
    fn drop(&mut self) {
        self.child.kill().ok();
    }
}
