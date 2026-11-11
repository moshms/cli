use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

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

pub fn ghcr_login(user: Option<&str>) -> Result<()> {
    let Ok(token) = std::env::var("GHCR_TOKEN") else {
        return Ok(());
    };
    let token = token.trim();
    if token.is_empty() {
        return Ok(());
    }
    let user = user.context("--ghcr-user is required when GHCR_TOKEN is set")?;
    login(user, token)
}

fn login(user: &str, token: &str) -> Result<()> {
    let mut child = Command::new("docker")
        .args(["login", "ghcr.io", "-u", user, "--password-stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("run docker login")?;

    child
        .stdin
        .take()
        .context("docker login stdin")?
        .write_all(token.as_bytes())
        .context("write docker login token")?;

    let output = child.wait_with_output().context("wait for docker login")?;
    if !output.status.success() {
        bail!(
            "docker login failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    println!("logged in to ghcr.io as {user}");
    Ok(())
}
