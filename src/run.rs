use crate::utils::{get_project_root};
use anyhow::{Context, Result};
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
            let compiler = if tokio::process::Command::new("clang").arg("--version").output().await.is_ok() {
                "clang"
            } else if extension == "cpp" {
                "g++"
            } else {
                "gcc"
            };
            compile_and_run(path, compiler, &[], args).await?
        }
        "go" => {
            tokio::process::Command::new("go")
                .arg("run")
                .arg(path)
                .args(args)
                .status()
                .await
                .context("go not found in PATH")?
        }
        "rs" => {
            compile_and_run(path, "rustc", &[], args).await?
        }
        "js" | "mjs" => {
            tokio::process::Command::new("node")
                .arg(path)
                .args(args)
                .status()
                .await
                .context("node not found in PATH")?
        }
        "ts" | "tsx" => {
            tokio::process::Command::new("ts-node")
                .arg(path)
                .args(args)
                .status()
                .await
                .context("ts-node not found in PATH. install it globally with npm.")?
        }
        "py" => {
            tokio::process::Command::new("python3")
                .arg(path)
                .args(args)
                .status()
                .await
                .context("python3 not found in PATH")?
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

    let mut args = Vec::new();
    for arg in extra_args {
        args.push(arg.to_string());
    }

    // smart include: look for "include" dir in parent and grandparent
    if let Some(parent) = src.parent() {
        let include_dir = parent.join("include");
        if include_dir.exists() {
            args.push(format!("-I{}", include_dir.display()));
        }
        if let Some(grandparent) = parent.parent() {
            let include_dir = grandparent.join("include");
            if include_dir.exists() {
                args.push(format!("-I{}", include_dir.display()));
            }
        }
    }

    let status = tokio::process::Command::new(compiler)
        .args(&args)
        .arg(src)
        .arg("-o")
        .arg(&bin)
        .status()
        .await
        .with_context(|| format!("{} not found in PATH", compiler))?;

    if !status.success() {
        anyhow::bail!("compile failed");
    }

    if !bin.exists() {
        anyhow::bail!("binary missing after compile");
    }

    let status = tokio::process::Command::new(&bin)
        .args(run_args)
        .status()
        .await
        .context("failed to execute the compiled binary")?;
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
                    .await
                    .context("cmake not found in PATH")?;
                
                if !cmake_status.success() {
                    return Ok(false);
                }

                let make_status = tokio::process::Command::new("make")
                    .current_dir(&build_dir)
                    .status()
                    .await
                    .context("make not found in PATH")?;

                return Ok(make_status.success());
            } else if cpp_root.join("Makefile").exists() {
                let status = tokio::process::Command::new("make")
                    .current_dir(cpp_root)
                    .status()
                    .await
                    .context("make not found in PATH")?;
                return Ok(status.success());
            } else if cpp_root.join("Cargo.toml").exists() {
                let status = tokio::process::Command::new("cargo")
                    .arg("run")
                    .arg("--")
                    .args(args)
                    .current_dir(cpp_root)
                    .status()
                    .await
                    .context("cargo not found in PATH")?;
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
                    .await
                    .context("cargo not found in PATH")?;
                return Ok(status.success());
            }
            "go" => {
                let status = tokio::process::Command::new("go")
                    .arg("build")
                    .arg(".")
                    .current_dir(r)
                    .status()
                    .await
                    .context("go not found in PATH")?;
                return Ok(status.success());
            }
            "c" | "cpp" => {
                if r.join("CMakeLists.txt").exists() {
                    let build_dir = r.join("build");
                    let _ = tokio::fs::create_dir_all(&build_dir).await;
                    let _ = tokio::process::Command::new("cmake").arg("..").current_dir(&build_dir).status().await.context("cmake not found in PATH")?;
                    let status = tokio::process::Command::new("make").current_dir(&build_dir).status().await.context("make not found in PATH")?;
                    return Ok(status.success());
                } else if r.join("Makefile").exists() {
                    let status = tokio::process::Command::new("make").current_dir(r).status().await.context("make not found in PATH")?;
                    return Ok(status.success());
                } else if r.join("Cargo.toml").exists() {
                    let status = tokio::process::Command::new("cargo").arg("build").current_dir(r).status().await.context("cargo not found in PATH")?;
                    return Ok(status.success());
                }
            }
            _ => {}
        }
    }

    println!("{} don't know how to build this yet", "info".yellow());
    Ok(true)
}
