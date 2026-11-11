use std::fs::File;
use std::path::Path;
use std::process::Stdio;

use anyhow::{Context, Result};

use crate::compose;

pub fn export(dir: &Path, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let file = File::create(output).with_context(|| format!("create {}", output.display()))?;
    let mut command = compose::command(dir);
    command
        .args([
            "exec",
            "-T",
            "postgres",
            "pg_dump",
            "-U",
            "musiccloud",
            "musiccloud",
        ])
        .stdout(Stdio::from(file));
    compose::run(&mut command)?;
    println!("database exported to {}", output.display());
    Ok(())
}

pub fn import(dir: &Path, input: &Path) -> Result<()> {
    let file = File::open(input).with_context(|| format!("open {}", input.display()))?;
    let mut command = compose::command(dir);
    command
        .args([
            "exec",
            "-T",
            "postgres",
            "psql",
            "-U",
            "musiccloud",
            "-d",
            "musiccloud",
            "-v",
            "ON_ERROR_STOP=1",
        ])
        .stdin(Stdio::from(file));
    println!("restoring {} into the existing database", input.display());
    compose::run(&mut command)
}
