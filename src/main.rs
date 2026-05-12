use anyhow::Result;
use clap::Parser;
use colored::*;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod args;
mod check;
mod install;
mod run;
mod utils;

use crate::args::Args;
use crate::check::run_check;
use crate::install::install_keen;
use crate::run::{run_output, run_proceed};
use crate::utils::shell_integ_check;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.install {
        return install_keen().await;
    }

    if args.init {
        return init_project().await;
    }

    let file = match args.file.as_ref() {
        Some(f) => f,
        None => {
            println!(
                "{} provide a file to check or run, or use --help",
                "info".yellow()
            );
            return Ok(());
        }
    };

    if !file.exists() {
        eprintln!(
            "{}: file not found: {}",
            "error".red().bold(),
            file.display()
        );
        std::process::exit(1);
    }

    shell_integ_check().await?;

    if args.watch {
        return watch_mode(file, &args).await;
    }

    let success = run_actions(file, &args).await?;
    if !success {
        std::process::exit(1);
    }

    Ok(())
}

async fn run_actions(file: &Path, args: &Args) -> Result<bool> {
    let start = Instant::now();
    let mut performed_action = false;
    let mut overall_success = true;

    if args.fmt {
        if !run_fmt(file).await? {
            overall_success = false;
        }
        performed_action = true;
    }

    if args.check {
        println!("{} checking {}", "→".cyan(), file.display());
        if file.is_dir() {
            if !check_dir(file).await? {
                overall_success = false;
            }
        } else {
            if !run_check(file).await? {
                overall_success = false;
            }
        }
        performed_action = true;
    }

    if args.proceed {
        if !run_proceed(file).await? {
            overall_success = false;
        }
        performed_action = true;
    }

    if args.output {
        if !run_output(file, args.project, &args.trailing).await? {
            overall_success = false;
        }
        performed_action = true;
    }

    // default to check if no flags provided
    if !performed_action {
        println!("{} checking {}", "→".cyan(), file.display());
        if file.is_dir() {
            if !check_dir(file).await? {
                overall_success = false;
            }
        } else {
            if !run_check(file).await? {
                overall_success = false;
            }
        }
    }

    let duration = start.elapsed();
    if performed_action || !args.check && !args.output && !args.proceed {
        let status = if overall_success { "success".green() } else { "failed".red() };
        println!("{} {} in {:.2?}", "info".yellow(), status, duration);
    }

    Ok(overall_success)
}

async fn check_dir(dir: &Path) -> Result<bool> {
    let mut all_ok = true;
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if !run_check(entry.path()).await? {
            all_ok = false;
        }
    }
    Ok(all_ok)
}

async fn run_fmt(path: &Path) -> Result<bool> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let cmd = match ext {
        "rs" => ("rustfmt", vec![path.to_str().unwrap()]),
        "go" => ("gofmt", vec!["-w", path.to_str().unwrap()]),
        "c" | "cpp" | "h" | "hpp" => ("clang-format", vec!["-i", path.to_str().unwrap()]),
        "js" | "ts" | "jsx" | "tsx" => ("prettier", vec!["--write", path.to_str().unwrap()]),
        _ => {
            println!("{} no formatter for .{}, skipping", "info".yellow(), ext);
            return Ok(true);
        }
    };

    println!("{} formatting {}", "→".magenta(), path.display());
    let status = tokio::process::Command::new(cmd.0)
        .args(cmd.1)
        .status()
        .await;

    match status {
        Ok(s) => Ok(s.success()),
        Err(_) => {
            println!("{} {} not found", "error".red(), cmd.0);
            Ok(false)
        }
    }
}

async fn init_project() -> Result<()> {
    let cwd = std::env::current_dir()?;
    println!("{} initializing in {}", "→".green(), cwd.display());

    println!("choose a template (rs, c, go, py) [default: rs]:");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let lang = input.trim().to_lowercase();

    match lang.as_str() {
        "c" => {
            tokio::fs::write(cwd.join("main.c"), "#include <stdio.h>\n\nint main() {\n    printf(\"hello world\\n\");\n    return 0;\n}\n").await?;
            println!("{} created main.c", "ok".green());
        }
        "go" => {
            tokio::fs::write(cwd.join("main.go"), "package main\n\nimport \"fmt\"\n\nfunc main() {\n    fmt.Println(\"hello world\")\n}\n").await?;
            println!("{} created main.go", "ok".green());
        }
        "py" => {
            tokio::fs::write(cwd.join("main.py"), "print(\"hello world\")\n").await?;
            println!("{} created main.py", "ok".green());
        }
        _ => {
            tokio::fs::write(cwd.join("main.rs"), "fn main() {\n    println!(\"hello world\");\n}\n").await?;
            println!("{} created main.rs", "ok".green());
        }
    }
    
    Ok(())
}

async fn watch_mode(file: &PathBuf, args: &Args) -> Result<()> {
    use notify::{Watcher, RecursiveMode};
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::channel(1);
    let mut watcher = notify::recommended_watcher(move |res| {
        if let Ok(_) = res {
            let _ = tx.blocking_send(());
        }
    })?;

    watcher.watch(file, RecursiveMode::Recursive)?;

    println!("{} watching for changes...", "info".yellow());
    
    // initial run
    let _ = run_actions(file, args).await;

    while let Some(_) = rx.recv().await {
        // debounce slightly
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        while let Ok(_) = rx.try_recv() {} // clear channel
        
        println!("\n{} change detected, re-running...", "info".yellow());
        let _ = run_actions(file, args).await;
    }

    Ok(())
}
