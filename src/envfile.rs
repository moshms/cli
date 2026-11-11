use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

pub fn load(dir: &Path) -> Result<String> {
    let path = dir.join(".env");
    fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))
}

pub fn set_value(content: &str, key: &str, value: &str) -> String {
    let prefix = format!("{key}=");
    let mut found = false;
    let mut lines = Vec::new();

    for line in content.lines() {
        if line.starts_with(&prefix) {
            lines.push(format!("{key}={value}"));
            found = true;
        } else {
            lines.push(line.to_owned());
        }
    }

    if !found {
        lines.push(format!("{key}={value}"));
    }

    lines.join("\n") + "\n"
}

pub fn get_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    content.lines().find_map(|line| line.strip_prefix(&prefix))
}

pub fn write_secure(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content).with_context(|| format!("write {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("chmod {}", path.display()))?;
    }

    Ok(())
}
