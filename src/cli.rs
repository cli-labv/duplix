use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "start", version, about = "Duplix · organiza duplicados en input/")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Carpeta a escanear. Por defecto usa <proyecto>/input
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Regla para elegir el original: oldest, newest, shortest-path, first-alpha
    #[arg(long, default_value = "oldest")]
    pub keep: KeepRule,

    /// Aplica la regla a todos los grupos sin preguntar
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Abre el menú interactivo (buscar, revertir, historial, limpiar)
    #[arg(long)]
    pub menu: bool,

    /// Modo automático: conserva el sugerido de cada grupo y mueve el resto
    /// de una sola pasada (masivo), sin preguntar nada
    #[arg(long)]
    pub auto: bool,

    /// Muestra qué se movería sin mover nada
    #[arg(long)]
    pub dry_run: bool,

    /// Tamaño mínimo en bytes para considerar archivos
    #[arg(long, default_value_t = 0)]
    pub min_size: u64,

    /// Nombres de carpeta a excluir (separados por coma)
    #[arg(long)]
    pub exclude: Option<String>,

    /// Seguir symlinks a propósito (por defecto no se siguen)
    #[arg(long)]
    pub follow_symlinks: bool,

    /// Número de hilos para hashing/escaneo. 0 = auto. En HDD usa 1-2.
    #[arg(long, default_value_t = 0)]
    pub threads: usize,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Revierte una ejecución (la última si no se indica un log)
    Undo {
        /// Ruta del log JSON. Si se omite, revierte la última ejecución
        #[arg(value_name = "LOG")]
        log: Option<PathBuf>,
    },
    /// Limpia el historial JSON y los restos (carpetas vacías) de la app
    Clean {
        /// Carpeta escaneada (por defecto input)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
    },
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum KeepRule {
    Oldest,
    Newest,
    ShortestPath,
    FirstAlpha,
}
