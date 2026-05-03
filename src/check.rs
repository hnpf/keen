use crate::utils::{find_root, parse_compiler_output};
use anyhow::{Context, Result};
use colored::*;
use std::path::Path;
use std::time::Instant;

pub async fn run_check(path: &Path) -> Result<bool> {
    let start = Instant::now();
    println!(
        "{} checking {}",
        "→".cyan(),
        path.display().to_string().bold()
    );

    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let (ok, msg) = match extension {
        "json" => check_json(path).await?,
        "c" | "cpp" | "h" | "hpp" => check_c_family(path).await?,
        "go" => check_go(path).await?,
        "rs" => check_rust(path).await?,
        "ts" | "tsx" => check_ts(path).await?,
        _ => {
            println!(
                "{} no analyzer for .{}, skipping",
                "info".yellow(),
                extension
            );
            return Ok(true);
        }
    };

    let duration = start.elapsed();
    if ok {
        println!("{} {} ({:.2?})", "ok".green().bold(), msg, duration);
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
