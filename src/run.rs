use crate::utils::{get_project_root};
use anyhow::Result;
use colored::*;
use std::path::{Path, PathBuf};

pub async fn run_output(path: &Path, project_mode: bool, args: &[String]) -> Result<bool> {
    println!(
        "{} running {}",
        "→".green(),
        path.display().to_string().bold()
    );

    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let root = get_project_root(path, extension);

    let ok = if project_mode || root.is_some() {
        println!("{} detected project root at {}", "info".yellow(), root.as_ref().unwrap().display());
        run_project(path, extension, root, args).await?
    } else {
        run_snippet(path, extension, args).await?
    };

    Ok(ok)
}

async fn run_snippet(path: &Path, extension: &str, args: &[String]) -> Result<bool> {
    let status = match extension {
        "c" | "cpp" => {
            compile_and_run(path, "clang", &[], args).await?
        }
        "go" => {
            tokio::process::Command::new("go")
                .arg("run")
                .arg(path)
                .args(args)
                .status()
                .await?
        }
        "rs" => {
            compile_and_run(path, "rustc", &[], args).await?
        }
        "js" | "mjs" => {
            tokio::process::Command::new("node")
                .arg(path)
                .args(args)
                .status()
                .await?
        }
        "ts" | "tsx" => {
            tokio::process::Command::new("ts-node")
                .arg(path)
                .args(args)
                .status()
                .await?
        }
        "json" => {
            let content = tokio::fs::read_to_string(path).await?;
            println!("{}", content);
            return Ok(true);
        }
        _ => {
            println!(
                "{} don't know how to run .{} files yet",
                "info".yellow(),
                extension
            );
            return Ok(true);
        }
    };

    Ok(status.success())
}

async fn compile_and_run(src: &Path, compiler: &str, extra_args: &[&str], run_args: &[String]) -> Result<std::process::ExitStatus> {
    let tmp = tempfile::tempdir()?;
    let bin = tmp.path().join("keen_exec");

    let status = tokio::process::Command::new(compiler)
        .args(extra_args)
        .arg(src)
        .arg("-o")
        .arg(&bin)
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("compile failed");
    }

    if !bin.exists() {
        anyhow::bail!("binary missing after compile");
    }

    let status = tokio::process::Command::new(&bin)
        .args(run_args)
        .status()
        .await?;
    Ok(status)
}

async fn run_project(path: &Path, extension: &str, root: Option<PathBuf>, args: &[String]) -> Result<bool> {
    if extension == "rs" {
        if let Some(cargo_root) = root {
            let status = tokio::process::Command::new("cargo")
                .arg("run")
                .arg("--")
                .args(args)
                .current_dir(cargo_root)
                .status()
                .await?;
            return Ok(status.success());
        }
    }

    if matches!(extension, "js" | "ts" | "jsx" | "tsx") {
        if let Some(node_root) = root {
            let status = tokio::process::Command::new("npm")
                .arg("start")
                .arg("--")
                .args(args)
                .current_dir(node_root)
                .status()
                .await?;
            return Ok(status.success());
        }
    }

    if extension == "go" {
        if let Some(go_root) = root {
            let status = tokio::process::Command::new("go")
                .arg("run")
                .arg(".")
                .args(args)
                .current_dir(go_root)
                .status()
                .await?;
            return Ok(status.success());
        }
    }

    if extension == "py" {
        let status = tokio::process::Command::new("python3")
            .arg(path)
            .args(args)
            .status()
            .await?;
        return Ok(status.success());
    }

    if matches!(extension, "c" | "cpp" | "h" | "hpp") {
        if let Some(cpp_root) = root {
            if cpp_root.join("CMakeLists.txt").exists() {
                let build_dir = cpp_root.join("build");
                let _ = tokio::fs::create_dir_all(&build_dir).await;
                
                let cmake_status = tokio::process::Command::new("cmake")
                    .arg("..")
                    .current_dir(&build_dir)
                    .status()
                    .await?;
                
                if !cmake_status.success() {
                    return Ok(false);
                }

                let make_status = tokio::process::Command::new("make")
                    .current_dir(&build_dir)
                    .status()
                    .await?;

                return Ok(make_status.success());
            } else if cpp_root.join("Makefile").exists() {
                let status = tokio::process::Command::new("make")
                    .current_dir(cpp_root)
                    .status()
                    .await?;
                return Ok(status.success());
            }
        }
    }

    println!(
        "{} detected project but no recognized runner found, falling back to snippet mode",
        "info".yellow()
    );
    run_snippet(path, extension, args).await
}

pub async fn run_proceed(path: &Path) -> Result<bool> {
    println!(
        "{} building {}",
        "→".blue(),
        path.display().to_string().bold()
    );
    
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let root = get_project_root(path, extension);

    if let Some(r) = root {
        match extension {
            "rs" => {
                let status = tokio::process::Command::new("cargo")
                    .arg("build")
                    .current_dir(r)
                    .status()
                    .await?;
                return Ok(status.success());
            }
            "go" => {
                let status = tokio::process::Command::new("go")
                    .arg("build")
                    .arg(".")
                    .current_dir(r)
                    .status()
                    .await?;
                return Ok(status.success());
            }
            "c" | "cpp" => {
                if r.join("CMakeLists.txt").exists() {
                    let build_dir = r.join("build");
                    let _ = tokio::fs::create_dir_all(&build_dir).await;
                    let _ = tokio::process::Command::new("cmake").arg("..").current_dir(&build_dir).status().await?;
                    let status = tokio::process::Command::new("make").current_dir(&build_dir).status().await?;
                    return Ok(status.success());
                } else if r.join("Makefile").exists() {
                    let status = tokio::process::Command::new("make").current_dir(r).status().await?;
                    return Ok(status.success());
                }
            }
            _ => {}
        }
    }

    println!("{} don't know how to build this yet", "info".yellow());
    Ok(true)
}
