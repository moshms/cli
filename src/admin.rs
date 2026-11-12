use std::io::Write;
use std::path::Path;
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use mc_core::Role;

use crate::compose;

pub fn create(dir: &Path, username: &str, password: &str, role: &str) -> Result<()> {
    let username = username.trim();
    if username.is_empty() {
        bail!("username is required");
    }
    if password.chars().count() < 12 {
        bail!("password must contain at least 12 characters");
    }
    if password.eq_ignore_ascii_case("change-me") {
        bail!("default password is not allowed");
    }
    let role = role
        .parse::<Role>()
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let id = mc_core::UserId::new();
    let password_hash = mc_auth::hash_password(password)?;
    let subsonic_secret = mc_auth::generate_secret();

    let output = psql_capture(
        dir,
        "INSERT INTO users (id, username, password_hash, subsonic_secret, role)
         VALUES (:'id'::uuid, :'username', :'password_hash', :'subsonic_secret', :'role')
         RETURNING id;",
        &[
            ("id", id.to_string()),
            ("username", username.to_owned()),
            ("password_hash", password_hash),
            ("subsonic_secret", subsonic_secret),
            ("role", role.as_str().to_owned()),
        ],
    )?;

    if output.trim().is_empty() {
        bail!("failed to create user");
    }

    let condition = match role {
        Role::Admin => "(permissions & 1) <> 0 AND NOT is_everyone",
        Role::Uploader => "lower(name) = lower('Загрузчик') AND NOT is_everyone",
        Role::Listener => "",
    };

    if !condition.is_empty() {
        psql_capture(
            dir,
            &format!(
                "INSERT INTO user_roles (user_id, role_id)
                 SELECT :'id'::uuid, r.id FROM roles r
                 WHERE {condition}
                 LIMIT 1
                 ON CONFLICT DO NOTHING;"
            ),
            &[("id", id.to_string())],
        )?;
    }

    println!("user {username} created with role {role}");
    Ok(())
}

pub fn reset_password(dir: &Path, username: &str, password: &str) -> Result<()> {
    if password.chars().count() < 12 {
        bail!("password must contain at least 12 characters");
    }
    if password.eq_ignore_ascii_case("change-me") {
        bail!("default password is not allowed");
    }
    let password_hash = mc_auth::hash_password(password)?;

    let output = psql_capture(
        dir,
        "UPDATE users
         SET password_hash = :'password_hash', updated_at = now()
         WHERE lower(username) = lower(:'username')
         RETURNING id;",
        &[
            ("username", username.trim().to_owned()),
            ("password_hash", password_hash),
        ],
    )?;

    if output.trim().is_empty() {
        bail!("user {username} not found");
    }

    println!("password for {username} updated");
    Ok(())
}

pub fn list(dir: &Path) -> Result<()> {
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
        "-c",
        "SELECT username, role, is_active FROM users ORDER BY lower(username);",
    ]);
    compose::run(&mut command)
}

fn psql_capture(dir: &Path, sql: &str, vars: &[(&str, String)]) -> Result<String> {
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
            "-tA",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, value) in vars {
        command.arg("-v").arg(format!("{key}={value}"));
    }

    let mut child = command.spawn().context("run psql")?;
    child
        .stdin
        .take()
        .context("psql stdin")?
        .write_all(sql.as_bytes())
        .context("write psql stdin")?;
    let output = child.wait_with_output().context("wait for psql")?;

    if !output.status.success() {
        bail!(
            "psql exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
