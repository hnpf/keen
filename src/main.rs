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

async fn run_output(path: &Path, _project_mode: bool) -> Result<()> {
    println!(
        "{} running {}",
        "→".green(),
        path.display().to_string().bold()
    );
    // TODO: sandbox runner
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
