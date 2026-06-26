use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::time::Duration;
use xf_typegen::contract::Contract;
use xf_typegen::extract::{self, ExtractOptions};
use xf_typegen::generate::{self, EntityMode, GeneratedFile, Target};

#[derive(Parser)]
#[command(
    name = "xf-typegen",
    version,
    about = "Generate real PHP types from XenForo's magic — full IDE autocomplete, no PhpStorm required."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Extract(ExtractArgs),
    Generate(GenerateArgs),
    Watch(GenerateArgs),
}

#[derive(Parser, Clone)]
struct ExtractArgs {
    root: Option<String>,

    #[arg(short, long, default_value = "xf-typegen.json")]
    out: PathBuf,

    #[arg(long)]
    addon: Option<String>,

    #[arg(long)]
    minify: bool,

    #[arg(long, value_parser = ["apply", "remove"])]
    mixin: Option<String>,

    #[arg(long, default_value = "php")]
    php_cmd: String,
}

#[derive(Parser, Clone)]
struct GenerateArgs {
    #[arg(short, long, default_value = "xf-typegen.json")]
    input: PathBuf,

    #[arg(short, long)]
    out_dir: Option<PathBuf>,

    #[arg(short, long)]
    targets: Option<String>,

    #[arg(long)]
    entity_mode: Option<String>,

    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Extract(args) => run_extract(&args),
        Command::Generate(args) => run_generate(&args),
        Command::Watch(args) => run_watch(&args),
    }
}

fn run_extract(args: &ExtractArgs) -> Result<()> {
    let opts = ExtractOptions {
        root: args.root.as_deref(),
        addon: args.addon.as_deref(),
        minify: args.minify,
        mixin: args.mixin.as_deref(),
        php_cmd: &args.php_cmd,
    };
    extract::run_to_file(&opts, &args.out)
}

fn resolve_entity_mode(spec: &Option<String>) -> Result<EntityMode> {
    match spec {
        None => Ok(EntityMode::Redeclare),
        Some(s) => EntityMode::parse(s)
            .with_context(|| format!("unknown entity mode '{}' (expected redeclare, mixin)", s)),
    }
}

fn resolve_targets(spec: &Option<String>) -> Result<Vec<Target>> {
    match spec {
        None => Ok(Target::ALL.to_vec()),
        Some(s) => {
            let mut targets = Vec::new();
            for part in s.split(',') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                let target = Target::parse(part)
                    .with_context(|| format!("unknown target '{}' (expected ide-helper, phpstorm-meta)", part))?;
                if !targets.contains(&target) {
                    targets.push(target);
                }
            }
            if targets.is_empty() {
                anyhow::bail!("no valid targets specified");
            }
            Ok(targets)
        }
    }
}

fn parent_or_dot(path: &Path) -> PathBuf {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn out_dir_for(args: &GenerateArgs) -> PathBuf {
    args.out_dir
        .clone()
        .unwrap_or_else(|| parent_or_dot(&args.input))
}

fn run_generate(args: &GenerateArgs) -> Result<()> {
    let targets = resolve_targets(&args.targets)?;
    let mode = resolve_entity_mode(&args.entity_mode)?;
    let contract = Contract::from_path(&args.input)?;
    let out_dir = out_dir_for(args);

    eprintln!(
        "Loaded {} entities (XF {}) from {}",
        contract.entities.len(),
        contract
            .xf
            .version
            .clone()
            .unwrap_or_else(|| contract.xf.version_id.to_string()),
        args.input.display()
    );

    let files = generate::render_all(&contract, &targets, mode);
    write_files(&out_dir, &files, args.dry_run)?;
    Ok(())
}

fn write_files(out_dir: &Path, files: &[GeneratedFile], dry_run: bool) -> Result<()> {
    if !dry_run {
        std::fs::create_dir_all(out_dir)
            .with_context(|| format!("creating output directory: {}", out_dir.display()))?;
    }

    for file in files {
        let path = out_dir.join(file.filename);
        let existing = std::fs::read_to_string(&path).ok();
        let changed = existing.as_deref() != Some(file.contents.as_str());

        let status = if !changed {
            "unchanged"
        } else if existing.is_some() {
            "updated"
        } else {
            "created"
        };

        if dry_run {
            eprintln!(
                "[dry-run] {:<9} {} ({} bytes)",
                status,
                path.display(),
                file.contents.len()
            );
        } else if changed {
            std::fs::write(&path, &file.contents)
                .with_context(|| format!("writing {}", path.display()))?;
            eprintln!("{:<9} {} ({} bytes)", status, path.display(), file.contents.len());
        } else {
            eprintln!("{:<9} {}", status, path.display());
        }
    }

    Ok(())
}

fn run_watch(args: &GenerateArgs) -> Result<()> {
    use notify::RecursiveMode;
    use notify_debouncer_mini::new_debouncer;
    use std::sync::mpsc::channel;

    let _ = resolve_targets(&args.targets)?;
    let _ = resolve_entity_mode(&args.entity_mode)?;
    if let Err(e) = run_generate(args) {
        eprintln!("initial generation failed: {e:#}");
    }

    let input = args
        .input
        .canonicalize()
        .unwrap_or_else(|_| args.input.clone());
    let watch_dir = parent_or_dot(&input);
    let target_name = input.file_name().map(|n| n.to_os_string());

    let (tx, rx) = channel();
    let mut debouncer = new_debouncer(Duration::from_millis(300), tx)
        .context("creating file watcher")?;
    debouncer
        .watcher()
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .with_context(|| format!("watching {}", watch_dir.display()))?;

    eprintln!("Watching {} (Ctrl-C to stop)", input.display());

    for res in rx {
        let events = match res {
            Ok(events) => events,
            Err(e) => {
                eprintln!("watch error: {e:?}");
                continue;
            }
        };

        let relevant = events.iter().any(|ev| {
            ev.path.file_name().map(|n| n.to_os_string()) == target_name
        });
        if !relevant {
            continue;
        }

        match run_generate(args) {
            Ok(()) => {}
            Err(e) => eprintln!("regeneration failed: {e:#}"),
        }
    }

    Ok(())
}
