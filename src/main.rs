use clap::Parser;
use duplix::cli::{Args, Command};
use duplix::ui::{self, RunConfig};
use duplix::{paths, undo};
use std::path::PathBuf;

fn main() {
    let args = Args::parse();
    let root = args.path.clone().unwrap_or_else(|| PathBuf::from("input"));

    match args.command {
        Some(Command::Undo { log }) => return run_undo(log),
        Some(Command::Clean { path }) => {
            let target = path.unwrap_or(root);
            return run_clean(&target);
        }
        None => {}
    }

    if !root.is_dir() {
        eprintln!("No es una carpeta válida: {}", root.display());
        std::process::exit(1);
    }

    if args.menu {
        if let Err(e) = ui::menu(&root) {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
        return;
    }

    let cfg = build_config(&args, root);
    if let Err(e) = ui::run(&cfg) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn build_config(args: &Args, root: PathBuf) -> RunConfig {
    let mut exclude: Vec<String> = args
        .exclude
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default();
    for dir in paths::generated_dirs() {
        if !exclude.iter().any(|e| e == dir) {
            exclude.push(dir.to_string());
        }
    }

    RunConfig {
        dest_dupe: paths::duplicate_dir(&root),
        dest_keep: paths::keep_dir(&root),
        root,
        keep: args.keep.clone(),
        min_size: args.min_size,
        exclude,
        follow_links: args.follow_symlinks,
        dry_run: args.dry_run,
        yes: args.yes,
    }
}

fn run_undo(log: Option<PathBuf>) {
    let result = match log {
        Some(path) => undo::undo(&path),
        None => undo::undo_latest(),
    };
    match result {
        Ok(r) => {
            println!("Revertidos {} archivos.", r.restored);
            for e in r.errors {
                eprintln!("  - {e}");
            }
        }
        Err(e) => {
            eprintln!("Error al deshacer: {e}");
            std::process::exit(1);
        }
    }
}

fn run_clean(root: &std::path::Path) {
    if !root.is_dir() {
        eprintln!("No es una carpeta válida: {}", root.display());
        std::process::exit(1);
    }
    ui::clean_report(root);
}
