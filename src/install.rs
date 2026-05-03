use anyhow::{Context, Result};
use colored::*;
use std::path::PathBuf;

pub async fn install_keen() -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let home = std::env::var("HOME").context("Could not find HOME directory")?;
    let local_bin = PathBuf::from(&home).join(".local/bin");

    if !local_bin.exists() {
        std::fs::create_dir_all(&local_bin)?;
    }

    let target = local_bin.join("keen");
    std::fs::copy(&current_exe, &target)?;

    println!(
        "{} installed to {}",
        "ok".green().bold(),
        target.display().to_string().cyan()
    );

    let path_env = std::env::var("PATH").unwrap_or_default();
    if !path_env.contains(local_bin.to_str().unwrap()) {
        println!(
            "{} {} is not in your $PATH",
            "warn".yellow(),
            local_bin.display()
        );

        let shell = std::env::var("SHELL").unwrap_or_default();
        let (config_file, cmd) = if shell.contains("zsh") {
            (
                Some(".zshrc"),
                format!("echo 'export PATH=\"$PATH:{}\"' >> ~/.zshrc", local_bin.display()),
            )
        } else if shell.contains("bash") {
            (
                Some(".bashrc"),
                format!("echo 'export PATH=\"$PATH:{}\"' >> ~/.bashrc", local_bin.display()),
            )
        } else if shell.contains("fish") {
            (
                Some(".config/fish/config.fish"),
                format!("fish_add_path {}", local_bin.display()),
            )
        } else {
            (None, String::new())
        };

        if let Some(_cfg) = config_file {
            println!("{} to finish, run this:", "info".cyan());
            println!("  {}", cmd);
        }
    }

    Ok(())
}
