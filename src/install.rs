use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::assets;
use crate::envfile::{self, set_value};

pub struct InstallOptions {
    pub library: Option<PathBuf>,
    pub domain: Option<String>,
    pub email: Option<String>,
    pub profiles: Option<String>,
    pub tag: Option<String>,
    pub admin_user: Option<String>,
    pub admin_password: Option<String>,
    pub master_key: Option<String>,
    pub tunnel_token: Option<String>,
    pub ghcr_user: Option<String>,
    pub no_start: bool,
    pub no_docker: bool,
    pub force: bool,
}

pub fn run(dir: &Path, options: InstallOptions) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;

    let library = options.library.unwrap_or_else(|| dir.join("library"));
    fs::create_dir_all(&library).with_context(|| format!("create {}", library.display()))?;
    let library =
        fs::canonicalize(&library).with_context(|| format!("resolve {}", library.display()))?;

    let admin_user = options.admin_user.unwrap_or_else(|| "admin".to_owned());
    let generated_admin_password = options.admin_password.is_none();
    let admin_password = options.admin_password.unwrap_or_else(random_password);
    let postgres_password = random_password();
    let master_key = options.master_key.unwrap_or_else(mc_auth::generate_token);
    let profiles = options.profiles.unwrap_or_default();
    let tag = options.tag.unwrap_or_else(|| "latest".to_owned());
    let metrics = if profiles
        .split(',')
        .any(|profile| profile.trim() == "monitoring")
    {
        "true"
    } else {
        "false"
    };
    let secure_cookie = if profiles.split(',').any(|profile| profile.trim() == "home") {
        "true"
    } else {
        "false"
    };

    let mut env = assets::ENV_EXAMPLE.to_owned();
    for (key, value) in [
        ("POSTGRES_PASSWORD", postgres_password.as_str()),
        ("MC_ADMIN_USER", admin_user.as_str()),
        ("MC_ADMIN_PASSWORD", admin_password.as_str()),
        ("MC_LIBRARY_PATH", library.to_string_lossy().as_ref()),
        ("MC_MASTER_KEY", master_key.as_str()),
        ("MC_DOMAIN", options.domain.as_deref().unwrap_or("")),
        ("MC_EMAIL", options.email.as_deref().unwrap_or("")),
        ("COMPOSE_PROFILES", profiles.as_str()),
        ("MC_TAG", tag.as_str()),
        ("MC_METRICS", metrics),
        ("MC_COOKIE_SECURE", secure_cookie),
        ("MC_TLS", "off"),
        (
            "CLOUDFLARE_TUNNEL_TOKEN",
            options.tunnel_token.as_deref().unwrap_or(""),
        ),
    ] {
        env = set_value(&env, key, value);
    }

    if options.no_docker {
        let staging = dir.join("staging");
        let cache = dir.join("cache");
        fs::create_dir_all(&staging).with_context(|| format!("create {}", staging.display()))?;
        fs::create_dir_all(&cache).with_context(|| format!("create {}", cache.display()))?;
        env = set_value(
            &env,
            "MC_DATABASE_URL",
            &format!("postgres://musiccloud:{postgres_password}@127.0.0.1:5432/musiccloud"),
        );
        env = set_value(&env, "MC_STAGING_PATH", staging.to_string_lossy().as_ref());
        env = set_value(&env, "MC_CACHE_DIR", cache.to_string_lossy().as_ref());
        let env_path = dir.join("music-cloud.env");
        write_file(&env_path, &env, options.force)?;
        write_native_units(dir, &profiles, options.force)?;

        println!("Native configuration written to {}", env_path.display());
        println!(
            "Install the binaries into /usr/local/bin and enable the systemd units from {}/systemd",
            dir.display()
        );
        println!("Library: {}", library.display());
        println!("Create user 'musiccloud' and chown the library to it.");
        if generated_admin_password {
            println!("Admin password: {admin_password}");
        }
        return Ok(());
    }

    write_file(&dir.join("compose.yml"), assets::COMPOSE, options.force)?;
    write_monitoring(dir, options.force)?;
    let env_path = dir.join(".env");
    if env_path.exists() && !options.force {
        bail!(
            "{} already exists, use --force to overwrite",
            env_path.display()
        );
    }
    envfile::write_secure(&env_path, &env)?;

    println!("Music Cloud installed in {}", dir.display());
    println!("Library: {}", library.display());
    println!("Grant write access to the container user:");
    println!("  sudo chown -R 65534:65534 {}", library.display());
    println!("Admin user: {admin_user}");
    if generated_admin_password {
        println!("Admin password: {admin_password}");
    }
    if profiles.split(',').any(|profile| profile.trim() == "home")
        && options.tunnel_token.as_deref().unwrap_or("").is_empty()
    {
        println!("Set CLOUDFLARE_TUNNEL_TOKEN in .env before starting the home profile.");
    }

    if options.no_start {
        println!(
            "Start with: docker compose -f {}/compose.yml --env-file {}/.env up -d",
            dir.display(),
            dir.display()
        );
        return Ok(());
    }

    crate::compose::ghcr_login(options.ghcr_user.as_deref())?;
    crate::compose::run(crate::compose::command(dir).args(["pull"]))?;
    crate::compose::run(crate::compose::command(dir).args(["up", "-d"]))?;
    println!("Stack started. Open the server in a browser.");
    Ok(())
}

pub fn update(dir: &Path, tag: Option<&str>, ghcr_user: Option<&str>) -> Result<()> {
    let env_path = dir.join(".env");
    let content = envfile::load(dir)?;
    let content = if let Some(tag) = tag {
        envfile::set_value(&content, "MC_TAG", tag)
    } else {
        content
    };
    envfile::write_secure(&env_path, &content)?;

    crate::compose::ghcr_login(ghcr_user)?;
    crate::compose::run(crate::compose::command(dir).args(["pull"]))?;
    crate::compose::run(crate::compose::command(dir).args(["up", "-d"]))?;
    println!("stack updated");
    Ok(())
}

fn write_monitoring(dir: &Path, force: bool) -> Result<()> {
    let files = [
        ("monitoring/prometheus.yml", assets::PROMETHEUS),
        ("monitoring/alerts.yml", assets::ALERTS),
        (
            "monitoring/grafana/provisioning/datasources/prometheus.yml",
            assets::GRAFANA_DATASOURCE,
        ),
        (
            "monitoring/grafana/provisioning/dashboards/dashboards.yml",
            assets::GRAFANA_DASHBOARDS,
        ),
        (
            "monitoring/grafana/dashboards/music-cloud.json",
            assets::GRAFANA_DASHBOARD,
        ),
    ];

    for (path, content) in files {
        write_file(&dir.join(path), content, force)?;
    }

    Ok(())
}

fn write_native_units(dir: &Path, profiles: &str, force: bool) -> Result<()> {
    let unit_dir = dir.join("systemd");
    fs::create_dir_all(&unit_dir).with_context(|| format!("create {}", unit_dir.display()))?;
    let env_path = dir.join("music-cloud.env");

    let api = format!(
        "[Unit]\nDescription=Music Cloud API\nAfter=network-online.target postgresql.service\nWants=network-online.target\n\n[Service]\nEnvironmentFile={}\nExecStart=/usr/local/bin/mc-api\nRestart=on-failure\nUser=musiccloud\nGroup=musiccloud\n\n[Install]\nWantedBy=multi-user.target\n",
        env_path.display()
    );
    write_file(&unit_dir.join("mc-api.service"), &api, force)?;

    let profiles = profiles.split(',').map(str::trim).collect::<Vec<_>>();
    if profiles.contains(&"worker") {
        let worker = format!(
            "[Unit]\nDescription=Music Cloud Worker\nAfter=network-online.target mc-api.service\nWants=network-online.target\n\n[Service]\nEnvironmentFile={}\nExecStart=/usr/local/bin/mc-worker\nRestart=on-failure\nUser=musiccloud\nGroup=musiccloud\n\n[Install]\nWantedBy=multi-user.target\n",
            env_path.display()
        );
        write_file(&unit_dir.join("mc-worker.service"), &worker, force)?;
    }

    if profiles.contains(&"transcode") {
        let transcoder = format!(
            "[Unit]\nDescription=Music Cloud Transcoder\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nEnvironmentFile={}\nExecStart=/usr/local/bin/mc-transcoder\nRestart=on-failure\nUser=musiccloud\nGroup=musiccloud\n\n[Install]\nWantedBy=multi-user.target\n",
            env_path.display()
        );
        write_file(&unit_dir.join("mc-transcoder.service"), &transcoder, force)?;
    }

    Ok(())
}

fn write_file(path: &Path, content: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        bail!(
            "{} already exists, use --force to overwrite",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn random_password() -> String {
    mc_auth::generate_token().chars().take(32).collect()
}
