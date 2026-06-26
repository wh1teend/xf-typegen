use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const EXTRACT_PHP: &str = include_str!("../extract.php");

pub struct ExtractOptions<'a> {
    pub root: Option<&'a str>,
    pub addon: Option<&'a str>,
    pub minify: bool,
    pub mixin: Option<&'a str>,
    pub php_cmd: &'a str,
}

pub fn run(opts: &ExtractOptions) -> Result<String> {
    let mut parts = opts.php_cmd.split_whitespace();
    let program = parts
        .next()
        .context("--php-cmd is empty; expected something like 'php'")?;

    let mut command = Command::new(program);
    command.args(parts);

    // Read the script from stdin: `php -- <args>` leaves $argv[0] as a stdin
    // placeholder, which extract.php already skips via array_slice($argv, 1).
    command.arg("--");
    if let Some(root) = opts.root {
        command.arg(root);
    }
    if let Some(addon) = opts.addon {
        command.arg(format!("--addon={addon}"));
    }
    if opts.minify {
        command.arg("--minify");
    }
    if let Some(mixin) = opts.mixin {
        command.arg(format!("--mixin={mixin}"));
    }

    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let mut child = command
        .spawn()
        .with_context(|| format!("launching PHP via '{}'", opts.php_cmd))?;

    // Stream the script in on a separate thread so a large script can't deadlock
    // against a full stdout pipe.
    let mut stdin = child.stdin.take().expect("stdin was piped");
    let writer = std::thread::spawn(move || stdin.write_all(EXTRACT_PHP.as_bytes()));

    let output = child
        .wait_with_output()
        .with_context(|| format!("waiting on PHP via '{}'", opts.php_cmd))?;

    writer
        .join()
        .expect("stdin writer thread panicked")
        .context("piping extract.php to PHP stdin")?;

    if !output.status.success() {
        bail!(
            "extract.php failed ({}). See the PHP error output above.",
            output.status
        );
    }

    String::from_utf8(output.stdout).context("extract.php produced non-UTF-8 output")
}

pub fn run_to_file(opts: &ExtractOptions, out: &Path) -> Result<()> {
    let json = run(opts)?;
    std::fs::write(out, &json).with_context(|| format!("writing contract to {}", out.display()))?;
    eprintln!("Wrote {}", out.display());
    Ok(())
}
