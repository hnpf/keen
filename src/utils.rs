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
    let specific = match extension {
        "rs" => find_root(path, "Cargo.toml"),
        "js" | "ts" | "jsx" | "tsx" => find_root(path, "package.json"),
        "go" => find_root(path, "go.mod"),
        "py" => find_root(path, "requirements.txt"),
        "c" | "cpp" | "h" | "hpp" => {
            find_root(path, "CMakeLists.txt").or_else(|| find_root(path, "Makefile"))
        }
        _ => None,
    };

    specific.or_else(|| {
        // polyglot fallback: if we're in a rust/node/go project but looking at a different file type
        find_root(path, "Cargo.toml")
            .or_else(|| find_root(path, "package.json"))
            .or_else(|| find_root(path, "go.mod"))
    })
}

pub fn parse_compiler_output(output: &str) {
    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        if line.contains(": error:") || line.contains("error[") {
            println!("{}", line.red());
        } else if line.contains(": warning:") {
            println!("{}", line.yellow());
        } else if line.contains(": note:") {
            println!("{}", line.dimmed());
        } else if line.contains("|") || line.contains("^") {
            // likely a code snippet or pointer from rustc/clang
            println!("{}", line.blue());
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
