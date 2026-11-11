use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Result};

use crate::{compose, envfile};

pub fn run(dir: &Path) -> Result<()> {
    let mut failures = 0_u32;

    match envfile::load(dir) {
        Ok(content) => {
            ok("environment file");
            for key in ["POSTGRES_PASSWORD", "MC_LIBRARY_PATH"] {
                if envfile::get_value(&content, key).is_some() {
                    ok(format!("{key} is set"));
                } else {
                    fail(&mut failures, format!("{key} is missing"));
                }
            }

            if let Some(library) = envfile::get_value(&content, "MC_LIBRARY_PATH") {
                let path = PathBuf::from(library);
                let path = if path.is_relative() {
                    dir.join(path)
                } else {
                    path
                };
                if path.is_dir() {
                    ok(format!("library {} exists", path.display()));
                } else {
                    fail(
                        &mut failures,
                        format!("library {} is missing", path.display()),
                    );
                }
            }
        }
        Err(error) => fail(&mut failures, format!("environment file: {error:#}")),
    }

    if dir.join("compose.yml").is_file() {
        ok("compose.yml");
    } else {
        fail(&mut failures, "compose.yml is missing");
    }

    if Command::new("docker")
        .arg("info")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        ok("docker");
    } else {
        fail(&mut failures, "docker is unavailable");
    }

    let config = compose::command(dir).arg("config").output();
    if config.is_ok_and(|output| output.status.success()) {
        ok("compose config");
    } else {
        fail(&mut failures, "compose config is invalid");
    }

    match compose::capture(compose::command(dir).args(["ps", "--status", "running", "--quiet"])) {
        Ok(output) if !output.trim().is_empty() => ok("compose project"),
        Ok(_) => fail(&mut failures, "no running containers"),
        Err(error) => fail(&mut failures, format!("compose project: {error:#}")),
    }

    match psql(dir, "SELECT count(*) FROM _sqlx_migrations WHERE success;") {
        Ok(value) => {
            let count = value.trim().parse::<u64>().unwrap_or(0);
            if count > 0 {
                ok(format!("database migrations applied ({count})"));
            } else {
                fail(&mut failures, "database migrations are not applied");
            }
        }
        Err(error) => fail(&mut failures, format!("database: {error:#}")),
    }

    match compose::capture(compose::command(dir).args([
        "exec",
        "-T",
        "nginx",
        "wget",
        "-qO-",
        "http://127.0.0.1/readyz",
    ])) {
        Ok(body) if body.contains("ready") => ok("nginx readyz"),
        Ok(_) => fail(&mut failures, "nginx readyz returned an unexpected body"),
        Err(error) => fail(&mut failures, format!("nginx readyz: {error:#}")),
    }

    match psql(
        dir,
        "SELECT count(*) FROM worker_heartbeats WHERE last_seen_at > now() - interval '30 seconds';",
    ) {
        Ok(value) => {
            let count = value.trim().parse::<u64>().unwrap_or(0);
            if count > 0 {
                ok("worker heartbeat");
            } else {
                println!("[warn] worker heartbeat: worker is not running");
            }
        }
        Err(error) => println!("[warn] worker heartbeat: {error:#}"),
    }

    if failures > 0 {
        bail!("{failures} checks failed");
    }

    println!("all checks passed");
    Ok(())
}

fn psql(dir: &Path, sql: &str) -> Result<String> {
    let mut command = compose::command(dir);
    command.args([
        "exec",
        "-T",
        "postgres",
        "psql",
        "-U",
        "musiccloud",
        "-d",
        "musiccloud",
        "-tA",
        "-c",
        sql,
    ]);
    compose::capture(&mut command)
}

fn ok(message: impl AsRef<str>) {
    println!("[ok] {}", message.as_ref());
}

fn fail(failures: &mut u32, message: impl AsRef<str>) {
    *failures += 1;
    println!("[fail] {}", message.as_ref());
}
