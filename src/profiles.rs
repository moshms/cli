use std::path::Path;

use anyhow::{bail, Result};

use crate::envfile;

const VALID_PROFILES: &[&str] = &["worker", "transcode", "monitoring", "tls", "home"];

pub fn list(dir: &Path) -> Result<()> {
    let content = envfile::load(dir)?;
    let profiles = envfile::get_value(&content, "COMPOSE_PROFILES").unwrap_or("");
    let metrics = envfile::get_value(&content, "MC_METRICS").unwrap_or("false");

    println!("profiles: {profiles}");
    println!("metrics: {metrics}");
    Ok(())
}

pub fn enable(dir: &Path, profile: &str) -> Result<()> {
    validate(profile)?;
    let content = envfile::load(dir)?;
    let mut profiles = current(&content);
    if !profiles.iter().any(|item| item == profile) {
        profiles.push(profile.to_owned());
    }
    save(dir, &content, &profiles)
}

pub fn disable(dir: &Path, profile: &str) -> Result<()> {
    validate(profile)?;
    let content = envfile::load(dir)?;
    let profiles = current(&content)
        .into_iter()
        .filter(|item| item != profile)
        .collect::<Vec<_>>();
    save(dir, &content, &profiles)
}

fn current(content: &str) -> Vec<String> {
    envfile::get_value(content, "COMPOSE_PROFILES")
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .map(str::to_owned)
        .collect()
}

fn save(dir: &Path, content: &str, profiles: &[String]) -> Result<()> {
    let joined = profiles.join(",");
    let metrics = if profiles.iter().any(|profile| profile == "monitoring") {
        "true"
    } else {
        "false"
    };
    let secure_cookie = if profiles.iter().any(|profile| profile == "home") {
        "true"
    } else {
        "false"
    };
    let updated = envfile::set_value(
        &envfile::set_value(
            &envfile::set_value(content, "COMPOSE_PROFILES", &joined),
            "MC_METRICS",
            metrics,
        ),
        "MC_COOKIE_SECURE",
        secure_cookie,
    );
    envfile::write_secure(&dir.join(".env"), &updated)?;
    println!("profiles: {joined}");
    println!("metrics: {metrics}");
    Ok(())
}

fn validate(profile: &str) -> Result<()> {
    if VALID_PROFILES.contains(&profile) {
        Ok(())
    } else {
        bail!(
            "unknown profile {profile}; expected one of {}",
            VALID_PROFILES.join(", ")
        )
    }
}
