use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Keen, the universal linter and runner!", long_about = None)]
pub struct Args {
    pub file: Option<PathBuf>,
    #[arg(short, long)]
    pub check: bool,
    #[arg(short, long)]
    pub output: bool,
    #[arg(short, long)]
    pub proceed: bool,
    #[arg(short = 'P', long)]
    pub project: bool,
    #[arg(long)]
    pub install: bool,
    #[arg(short, long)]
    pub watch: bool,
    #[arg(long)]
    pub fmt: bool,
    #[arg(long)]
    pub init: bool,
    #[arg(last = true)]
    pub trailing: Vec<String>,
}
