use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about = "Keen, the universal linter and runner!", long_about = None)]
struct Args {
    file: PathBuf,
    #[arg(short, long)]
    check: bool,
    #[arg(short, long)]
    output: bool,
    #[arg(short, long)]
    proceed: bool,
    #[arg(short = 'P', long)]
    project: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if !args.file.exists() {
        eprintln!(
            "{}: file not found: {}",
            "error".red().bold(),
            args.file.display()
        );
        std::process::exit(1);
    }

    let mut performed_action = false;
    if args.check {
        run_check(&args.file).await?;
        performed_action = true;
    }

    if args.output {
        run_output(&args.file, args.project).await?;
        performed_action = true;
    }
    if args.proceed {
        run_proceed(&args.file).await?;
        performed_action = true;
    }

    if !performed_action {
        run_check(&args.file).await?;
    }
    Ok(())
}

async fn run_check(path: &Path) -> Result<()> {
    println!(
        "{} checking {}",
        "→".cyan(),
        path.display().to_string().bold()
    );

    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension {
        "json" => check_json(path).await?,
        "c" | "cpp" | "h" | "hpp" => check_c_family(path).await?,
        "go" => check_go(path).await?,
        "rs" => check_rust(path).await?,
        _ => println!(
            "{} no analyzer for .{}, skipping",
            "info".yellow(),
            extension
        ),
    }

    Ok(())
}

async fn check_json(path: &Path) -> Result<()> {
    let content = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read file")?;

    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(_) => println!("{} {}", "ok".green().bold(), "valid JSON"),
        Err(e) => {
            println!(
                "{}:{}:{} -> {}",
                path.display().to_string().dimmed(),
                e.line(),
                e.column(),
                e.to_string().red()
            );
        }
    }
    Ok(())
}

async fn check_c_family(path: &Path) -> Result<()> {
    let output = tokio::process::Command::new("clang")
        .arg("-fsyntax-only")
        .arg("-Wall")
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        parse_compiler_output(&String::from_utf8_lossy(&output.stderr));
    } else {
        println!("{} {}", "ok".green().bold(), "C/C++ syntax valid");
    }
    Ok(())
}

async fn check_go(path: &Path) -> Result<()> {
    let output = tokio::process::Command::new("go")
        .arg("build")
        .arg("-o")
        .arg("/dev/null")
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        parse_compiler_output(&String::from_utf8_lossy(&output.stderr));
    } else {
        println!("{} {}", "ok".green().bold(), "go syntax valid");
    }
    Ok(())
}

async fn check_rust(path: &Path) -> Result<()> {
    // cargo check if we're inside a project, rustc otherwise
    // TODO: -Z no-codegen needs nightly, worth adding a stable fallback
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
    } else {
        println!("{} {}", "ok".green().bold(), "rust syntax valid");
    }
    Ok(())
}

fn parse_compiler_output(output: &str) {
    for line in output.lines() {
        if line.contains(": error:") || line.contains(": warning:") || line.contains("error[") {
            println!("{}", line);
        }
    }
}

fn find_root(path: &Path, marker: &str) -> Option<PathBuf> {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.join(marker).exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

async fn run_output(path: &Path, project_mode: bool) -> Result<()> {
    println!(
        "{} running {}",
        "→".green(),
        path.display().to_string().bold()
    );

    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let is_project = project_mode || (detect_project(path) && is_relevant_for_project(extension, path));

    if is_project {
        run_project(path, extension).await?;

    } else {
        run_snippet(path, extension).await?;
    }
    
    Ok(())
}

fn detect_project(path: &Path) -> bool {
    find_root(path, "Cargo.toml").is_some() ||
    find_root(path, "package.json").is_some() ||
    find_root(path, "go.mod").is_some() ||
}


fn is_relevant_for_project(extension: &str, path: &Path) -> bool {
    match extension {
        "rs" => find_root(path, "Cargo.toml").is_some(),
        "js" | "ts" | "jsx" | "tsx" => find_root(path, "package.json").is_some(),
        "go" => find_root(path, "go.mod").is_some(),
        _ => false,
    }
}

async fn run_snippet(path: &Path, extension: &str) -> Result<()> {
    match extension {
        "c" | "cpp" => {
            let tmp_dir = tempfile::tempdir()?;
            let tmp_path = tmp_dir.path().join("keen_exec");
            
            let status = tokio::process::Command::new("clang")
                .arg(path)
                .arg("-o")
                .arg(&tmp_path)
                .status()
                .await?;

            if status.success() {
                if tmp_path.exists() {
                    tokio::process::Command::new(&tmp_path).status().await?;
                } else {
                    anyhow::bail!("binary created by compiler not found at {}", tmp_path.display());
                }
            } else {
                anyhow::bail!("compiler failed to compile snippet");
            }
        }
        "go" => {
            tokio::process::Command::new("go")
                .arg("run")
                .arg(path)
                .status()
                .await?;
        }
        "rs" => {
            let tmp_dir = tempfile::tempdir()?;
            let tmp_path = tmp_dir.path().join("keen_exec");

            let status = tokio::process::Command::new("rustc")
                .arg(path)
                .arg("-o")
                .arg(&tmp_path)
                .status()
                .await?;

            if status.success() {
                if tmp_path.exists() {
                    tokio::process::Command::new(&tmp_path).status().await?;
                } else {
                    anyhow::bail!("binary created by compiler not found at {}", tmp_path.display());
                }
            } else {
                anyhow::bail!("compiler failed to compile snippet");
            }
        }
        "js" | "mjs" => {
            tokio::process::Command::new("node")
                .arg(path)
                .status()
                .await?;
        }
        "json" => {
            let content = tokio::fs::read_to_string(path).await?;
            println!("{}", content);
        }
        _ => println!("{} don't know how to run .{} files yet", "info".yellow(), extension),
    }
    Ok(())
}

async fn run_project(path: &Path, extension: &str) -> Result<()> {
    if extension == "rs" {
        if let Some(cargo_root) = find_root(path, "Cargo.toml") {
            tokio::process::Command::new("cargo")
                .arg("run")
                .current_dir(cargo_root)
                .status()
                .await?;
            return Ok(());
        }
    }
    
    if matches!(extension, "js" | "ts" | "jsx" | "tsx") {
        if let Some(node_root) = find_root(path, "package.json") {
            tokio::process::Command::new("npm")
                .arg("start")
                .current_dir(node_root)
                .status()
                .await?;
            return Ok(());
        }
    }

    if extension == "go" {
        if let Some(go_root) = find_root(path, "go.mod") {
            tokio::process::Command::new("go")
                .arg("run")
                .arg(".")
                .current_dir(go_root)
                .status()
                .await?;
            return Ok(());
        }
    }

    println!("{} detected project but no recognized runner found, falling back to snippet mode", "info".yellow());
    run_snippet(path, extension).await?;
    Ok(())
}

async fn check_shell_integration() -> Result<()> {
    let exe_path = std::env::current_exe()?;
    let path_env = std::env::var("PATH").unwrap_or_default();
    
    let in_path = path_env.split(':').any(|p| {
        let p = Path::new(p);
        exe_path.starts_with(p)
    });

    if !in_path {
        println!("{} Keen is not in your $PATH! do you wish to add it? (run with --install)", "info".yellow());
    }
    
    Ok(())
}

async fn install_keen() -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let home = std::env::var("HOME").context("Could not find HOME directory")?;
    let local_bin = PathBuf::from(&home).join(".local/bin");
    
    if !local_bin.exists() {
        std::fs::create_dir_all(&local_bin)?;
    }
    
    let target = local_bin.join("keen");
    std::fs::copy(&current_exe, &target)?;
    
    println!("{} installed to {}", "ok".green().bold(), target.display().to_string().cyan());
    
    let path_env = std::env::var("PATH").unwrap_or_default();
    if !path_env.contains(local_bin.to_str().unwrap()) {
        println!("{} {} is not in your $PATH", "warn".yellow(), local_bin.display());
        
        let shell = std::env::var("SHELL").unwrap_or_default();
        let config_file = if shell.contains("zsh") {
            Some(".zshrc")
        } else if shell.contains("bash") {
            Some(".bashrc")
        } else if shell.contains("fish") {
            Some(".config/fish/config.fish")
        } else {
            None
        };

        if let Some(cfg) = config_file {
            let cfg_path = PathBuf::from(&home).join(cfg);
            println!("{} add to {}? (y/n)", "prompt".magenta(), cfg);
            // need read stdin.. temporary
            println!("run this: echo 'export PATH=\"$PATH:{}\"' >> {}", local_bin.display(), cfg_path.display());
        }
    }
    
    Ok(())
}

async fn run_proceed(path: &Path) -> Result<()> {
    println!(
        "{} building {}",
        "→".blue(),
        path.display().to_string().bold()
    );
    // TODO: build logic
    Ok(())
}
