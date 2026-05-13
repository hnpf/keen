use std::time::Instant; 
use crate::utils::{find_root, parse_compiler_output};
use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use std::path::Path;
pub async fn run_check(path: &Path) -> Result<bool> {
    let start = Instant::now();
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    if matches!(extension, "json" | "c" | "cpp" | "h" | "hpp" | "go" | "rs" | "ts" | "tsx" | "py") {
        print!("{} check {} ... ", "→".cyan(), path.display());
        let _ = io::stdout().flush();
    }

    let (ok, msg) = match extension {
        "json" => check_json(path).await?,
        "c" | "cpp" | "h" | "hpp" => check_c_family(path).await?,
        "go" => check_go(path).await?,
        "rs" => check_rust(path).await?,
        "ts" | "tsx" => check_ts(path).await?,
        "py" => check_py(path).await?,
        _ => {
            return Ok(true);
        }
    };

    let duration = start.elapsed();
    if ok {
        println!("{} ({:.2?})", msg.green(), duration);
    } else {
        println!("{}", "failed".red());
    }

    Ok(ok)
}

async fn check_json(path: &Path) -> Result<(bool, String)> {
    let content = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read file")?;

    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(_) => Ok((true, "valid JSON".to_string())),
        Err(e) => {
            println!(
                "{}:{}:{} -> {}",
                path.display().to_string().dimmed(),
                e.line(),
                e.column(),
                e.to_string().red()
            );
            Ok((false, String::new()))
        }
    }
}

async fn check_c_family(path: &Path) -> Result<(bool, String)> {
    let output = tokio::process::Command::new("clang")
        .arg("-fsyntax-only")
        .arg("-Wall")
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        parse_compiler_output(&String::from_utf8_lossy(&output.stderr));
        Ok((false, String::new()))
    } else {
        Ok((true, "C/C++ syntax valid".to_string()))
    }
}

async fn check_go(path: &Path) -> Result<(bool, String)> {
    let output = tokio::process::Command::new("go")
        .arg("build")
        .arg("-o")
        .arg("/dev/null")
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        parse_compiler_output(&String::from_utf8_lossy(&output.stderr));
        Ok((false, String::new()))
    } else {
        Ok((true, "go syntax valid".to_string()))
    }
}

async fn check_rust(path: &Path) -> Result<(bool, String)> {
    let cargo_root = find_root(path, "Cargo.toml");

    let output = if let Some(ref root) = cargo_root {
        tokio::process::Command::new("cargo")
            .arg("check")
            .current_dir(root)
            .output()
            .await?
    } else {
        tokio::process::Command::new("rustc")
            .arg("-Z")
            .arg("no-codegen")
            .arg(path)
            .output()
            .await?
    };

    if !output.status.success() {
        parse_compiler_output(&String::from_utf8_lossy(&output.stderr));
        Ok((false, String::new()))
    } else {
        Ok((true, "rust syntax valid".to_string()))
    }
}

async fn check_ts(path: &Path) -> Result<(bool, String)> {
    let output = tokio::process::Command::new("tsc")
        .arg("--noEmit")
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        parse_compiler_output(&String::from_utf8_lossy(&output.stdout));
        Ok((false, String::new()))
    } else {
        Ok((true, "ts syntax valid".to_string()))
    }
}

async fn check_py(path: &Path) -> Result<(bool, String)> {
    let output = tokio::process::Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        parse_compiler_output(&String::from_utf8_lossy(&output.stderr));
        Ok((false, String::new()))
    } else {
        Ok((true, "python syntax valid".to_string()))
    }
}
