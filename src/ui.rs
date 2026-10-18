use crate::clean;
use crate::cli::KeepRule;
use crate::fingerprint::{self, Fingerprint};
use crate::grouping;
use crate::mover::{self, MovePlan, MoveReport, UndoLog};
use crate::original_rule::choose_original;
use crate::paths;
use crate::scanner::{self, ScanOptions};
use crate::table::{Align, Table};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct RunConfig {
    pub root: PathBuf,
    pub dest_dupe: PathBuf,
    pub dest_keep: PathBuf,
    pub keep: KeepRule,
    pub min_size: u64,
    pub exclude: Vec<String>,
    pub follow_links: bool,
    pub dry_run: bool,
    pub yes: bool,
}

impl RunConfig {
    /// Configuración por defecto para un root (usada por el menú).
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            dest_dupe: paths::duplicate_dir(root),
            dest_keep: paths::keep_dir(root),
            keep: KeepRule::Oldest,
            min_size: 0,
            exclude: paths::generated_dirs().iter().map(|s| s.to_string()).collect(),
            follow_links: false,
            dry_run: false,
            yes: false,
        }
    }
}

pub enum Outcome {
    Applied(Box<MovePlan>),
    DryRun(Box<MovePlan>),
    Nothing,
    Cancelled,
}

pub fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

fn file_name(fp: &Fingerprint) -> String {
    fp.path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| fp.path.display().to_string())
}

fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

fn bar(total: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template("{msg} [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("=> "),
    );
    pb.set_message(msg.to_string());
    pb
}

pub fn run(cfg: &RunConfig) -> io::Result<Outcome> {
    let files = scan_stage(cfg)?;
    if files.is_empty() {
        println!("{}", style("Sin archivos para analizar.").yellow());
        return Ok(Outcome::Nothing);
    }

    let paths: Vec<PathBuf> = files.into_iter().map(|f| f.path).collect();
    let fps = hash_stage(&paths)?;
    let groups = grouping::group_duplicates(fps);
    if groups.is_empty() {
        println!("{}", style("Sin duplicados.").green());
        return Ok(Outcome::Nothing);
    }

    let Some(keep) = decide_keep(cfg, &groups)? else {
        return Ok(Outcome::Cancelled);
    };

    let plan = mover::build_moves(&groups, &keep, &cfg.dest_dupe, &cfg.dest_keep);
    if plan.moves.is_empty() {
        println!("{}", style("Sin cambios.").yellow());
        return Ok(Outcome::Nothing);
    }

    if cfg.dry_run {
        print_plan(&plan, "Simulación (dry-run)");
        return Ok(Outcome::DryRun(Box::new(plan)));
    }

    print_plan(&plan, "Se aplicará");
    print_totals(&plan);

    if !cfg.yes && !confirm()? {
        return Ok(Outcome::Cancelled);
    }

    let stamp = mover::timestamp();
    let log_path = paths::history_dir().join(format!("duplix-undo-{stamp}.json"));
    let log = UndoLog {
        fecha: stamp,
        revertido_en: None,
        carpeta_escaneada: cfg.root.clone(),
        carpeta_duplicados: cfg.dest_dupe.clone(),
        movimientos: plan.moves.clone(),
    };
    let report = exec_stage(&log, &log_path)?;
    println!(
        "{} {}",
        style("Historial:").dim(),
        style(log_path.display()).dim()
    );
    print_errors(&report);
    Ok(Outcome::Applied(Box::new(plan)))
}

pub fn scan_stage(cfg: &RunConfig) -> io::Result<Vec<scanner::FileInfo>> {
    let pb = spinner("escaneando…");
    let counter = Arc::new(AtomicUsize::new(0));
    let files = scanner::scan(
        &cfg.root,
        ScanOptions {
            min_size: cfg.min_size,
            exclude: cfg.exclude.clone(),
            follow_links: cfg.follow_links,
        },
        counter,
    );
    pb.finish_with_message(format!("escaneados {} archivos", files.len()));
    Ok(files)
}

pub fn hash_stage(paths: &[PathBuf]) -> io::Result<Vec<Fingerprint>> {
    let pb = bar(paths.len() as u64, "comparando");
    let counter = Arc::new(AtomicUsize::new(0));
    let done = counter.clone();
    let handle = std::thread::spawn({
        let paths = paths.to_vec();
        move || fingerprint::fingerprint_files(&paths, Some(done))
    });
    while !handle.is_finished() {
        pb.set_position(counter.load(Ordering::Relaxed) as u64);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let fps = handle.join().unwrap_or_default();
    pb.finish_and_clear();
    println!("{}", style(format!("comparados {} archivos", fps.len())).dim());
    Ok(fps)
}

/// Devuelve el índice a conservar por grupo. Con `--yes` usa la regla sin
/// preguntar; en terminal, revisa grupo a grupo.
fn decide_keep(cfg: &RunConfig, groups: &[Vec<Fingerprint>]) -> io::Result<Option<Vec<usize>>> {
    if cfg.yes || !io::stdin().is_terminal() {
        return Ok(Some(
            groups.iter().map(|g| choose_original(g, &cfg.keep)).collect(),
        ));
    }

    let theme = ColorfulTheme::default();
    let mut keep = Vec::with_capacity(groups.len());
    for (gi, group) in groups.iter().enumerate() {
        let default = choose_original(group, &cfg.keep);
        // El sugerido va primero para que Enter avance de inmediato.
        let mut order: Vec<usize> = vec![default];
        order.extend((0..group.len()).filter(|&i| i != default));

        let mut items: Vec<String> = order.iter().map(|&i| file_name(&group[i])).collect();
        items.push("Saltar este grupo (no mover nada)".to_string());
        items.push("Salir sin mover nada".to_string());

        let prompt = format!(
            "Grupo {}/{} · {} copias · {}",
            gi + 1,
            groups.len(),
            group.len(),
            human(group[0].size)
        );
        let pick = Select::with_theme(&theme)
            .with_prompt(prompt)
            .items(&items)
            .default(0)
            .interact()
            .map_err(|e| io::Error::other(e.to_string()))?;

        if pick == group.len() + 1 {
            return Ok(None);
        } else if pick == group.len() {
            keep.push(usize::MAX);
        } else {
            keep.push(order[pick]);
        }
    }
    Ok(Some(keep))
}

fn confirm() -> io::Result<bool> {
    let ans = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "¿Mover a {} y {}?",
            style(paths::DUPLICATE_DIR).cyan(),
            style(paths::KEEP_DIR).cyan()
        ))
        .default(true)
        .interact()
        .map_err(|e| io::Error::other(e.to_string()))?;
    if !ans {
        println!("{}", style("Cancelado. No se movió nada.").yellow());
    }
    Ok(ans)
}

fn plan_table(plan: &MovePlan) -> Table {
    let mut t = Table::new(
        &["Grupo", "Copias", "Tamaño", "Se conserva", "A duplicate/"],
        &[Align::Right, Align::Right, Align::Right, Align::Left, Align::Right],
    );
    for g in &plan.groups {
        t.row(&[
            g.index.to_string(),
            g.copies.to_string(),
            human(g.size),
            g.kept.clone(),
            g.moved.to_string(),
        ]);
    }
    t
}

fn print_plan(plan: &MovePlan, title: &str) {
    println!("\n{}", style(title).bold());
    plan_table(plan).print();
}

fn print_totals(plan: &MovePlan) {
    println!(
        "{} conservados · {} duplicados movidos · {} liberados",
        style(plan.kept_count()).green(),
        style(plan.duplicates()).yellow(),
        style(human(plan.freed)).cyan(),
    );
}

fn print_errors(report: &MoveReport) {
    for e in &report.errors {
        eprintln!("{}", style(format!("  - {e}")).red());
    }
}

fn exec_stage(log: &UndoLog, log_path: &Path) -> io::Result<MoveReport> {
    let pb = bar(log.movimientos.len() as u64, "moviendo");
    let report = match mover::execute_moves(log, log_path, |count, _| {
        pb.set_position(count as u64);
    }) {
        Ok(r) => {
            pb.finish_and_clear();
            r
        }
        Err(e) => {
            pb.finish_and_clear();
            eprintln!("{}", style(format!("Error: {e}")).red());
            MoveReport::default()
        }
    };
    Ok(report)
}

/// Menú interactivo: buscar/organizar, revertir, ver historial, limpiar.
pub fn menu(root: &Path) -> io::Result<()> {
    let theme = ColorfulTheme::default();
    loop {
        let items = [
            "Buscar y organizar duplicados",
            "Revertir la última ejecución",
            "Ver historial",
            "Limpiar historial y cache",
            "Salir",
        ];
        let pick = Select::with_theme(&theme)
            .with_prompt("Duplix · menú")
            .items(&items)
            .default(0)
            .interact_opt()
            .map_err(|e| io::Error::other(e.to_string()))?;

        match pick {
            Some(0) => {
                if let Err(e) = run(&RunConfig::new(root)) {
                    eprintln!("{}", style(format!("Error: {e}")).red());
                }
            }
            Some(1) => revert_last(),
            Some(2) => show_history(),
            Some(3) => clean_report(root),
            _ => break,
        }
        println!();
    }
    Ok(())
}

fn revert_last() {
    match crate::undo::undo_latest() {
        Ok(r) if r.errors.is_empty() => {
            println!("{}", style(format!("Revertidos {} archivos.", r.restored)).green());
        }
        Ok(r) => {
            println!("{}", style(format!("Revertidos {} archivos.", r.restored)).yellow());
            for e in &r.errors {
                eprintln!("  - {e}");
            }
        }
        Err(e) => println!("{}", style(e).yellow()),
    }
}

fn show_history() {
    let entries = crate::undo::history();
    if entries.is_empty() {
        println!("{}", style("Sin historial.").dim());
        return;
    }
    let mut t = Table::new(
        &["#", "Fecha (ms)", "Archivos", "Estado"],
        &[Align::Right, Align::Left, Align::Right, Align::Left],
    );
    for (i, e) in entries.iter().enumerate() {
        let estado = if e.reverted { "revertido" } else { "activo" };
        t.row(&[
            (i + 1).to_string(),
            e.fecha.clone(),
            e.movimientos.to_string(),
            estado.to_string(),
        ]);
    }
    println!();
    t.print();
}

pub fn clean_report(root: &Path) {
    let report = clean::clean(root);
    if report.items.is_empty() && report.errors.is_empty() {
        println!("{}", style("Nada que limpiar.").dim());
        return;
    }
    let mut t = Table::new(
        &["Elemento", "Archivos", "Tamaño"],
        &[Align::Left, Align::Right, Align::Right],
    );
    for it in &report.items {
        t.row(&[it.what.clone(), it.files.to_string(), human(it.bytes)]);
    }
    println!();
    t.print();
    for e in &report.errors {
        eprintln!("  - {e}");
    }
}
