use anyhow::Result;
use colored::*;
use std::path::{Path, PathBuf};

pub fn find_root(path: &Path, marker: &str) -> Option<PathBuf> {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.join(marker).exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

pub fn get_project_root(path: &Path, extension: &str) -> Option<PathBuf> {
    match extension {
        "rs" => find_root(path, "Cargo.toml"),
        "js" | "ts" | "jsx" | "tsx" => find_root(path, "package.json"),
        "go" => find_root(path, "go.mod"),
        "py" => find_root(path, "requirements.txt"),
        _ => None,
    }
}

pub fn parse_compiler_output(output: &str) {
    for line in output.lines() {
        if line.contains(": error:") || line.contains(": warning:") || line.contains("error[") {
            println!("{}", line);
        }
    }
}

pub async fn shell_integ_check() -> Result<()> {
    if std::env::var("CARGO").is_ok() {
        return Ok(());
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let state_dir = PathBuf::from(&home).join(".config/keen");
    let state_file = state_dir.join(".warned");

    if state_file.exists() {
        return Ok(());
    }

    let exe_path = std::env::current_exe()?;
    let path_env = std::env::var("PATH").unwrap_or_default();

    let in_path = path_env.split(':').any(|p| {
        exe_path
            .parent()
            .map_or(false, |parent| parent == Path::new(p))
    });

    if !in_path {
        println!("{} keen not in $PATH, use --install to fix", "info".yellow());

        // only warn once
        if !state_dir.exists() {
            let _ = std::fs::create_dir_all(&state_dir);
        }
        let _ = std::fs::write(state_file, "");
    }

    Ok(())
}
