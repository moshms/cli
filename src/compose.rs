use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

pub fn command(dir: &Path) -> Command {
    let mut command = Command::new("docker");
    command
        .arg("compose")
        .arg("-f")
        .arg(dir.join("compose.yml"))
        .arg("--env-file")
        .arg(dir.join(".env"));
    command
}

pub fn run(command: &mut Command) -> Result<()> {
    let status = command.status().context("run docker compose")?;
    if !status.success() {
        bail!("docker compose exited with {status}");
    }
    Ok(())
}

pub fn capture(command: &mut Command) -> Result<String> {
    let output = command.output().context("run docker compose")?;
    if !output.status.success() {
        bail!(
            "docker compose exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
