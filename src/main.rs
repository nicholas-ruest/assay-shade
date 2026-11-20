use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use assay_shade::{AuditReport, BedReader, audit, mask};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Quantify how much of each target is hidden by blind regions.
    Audit {
        /// BED file containing target intervals.
        #[arg(short, long)]
        targets: PathBuf,
        /// BED file containing assay blind regions.
        #[arg(short, long)]
        blind: PathBuf,
        /// Report encoding written to stdout.
        #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
        format: OutputFormat,
        /// Exit with status 2 when the global blind percentage exceeds this value.
        #[arg(long, value_parser = parse_percentage)]
        fail_above: Option<f64>,
    },
    /// Emit the target fragments that do not overlap blind regions.
    Mask {
        /// BED file containing target intervals.
        #[arg(short, long)]
        targets: PathBuf,
        /// BED file containing assay blind regions.
        #[arg(short, long)]
        blind: PathBuf,
        /// Output BED path, or '-' for stdout.
        #[arg(short, long, default_value = "-")]
        output: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Table,
    Json,
}

fn parse_percentage(raw: &str) -> Result<f64, String> {
    let value = raw
        .parse::<f64>()
        .map_err(|_| format!("'{raw}' is not a number"))?;
    if (0.0..=100.0).contains(&value) {
        Ok(value)
    } else {
        Err("percentage must be between 0 and 100".to_owned())
    }
}

fn load(path: &Path) -> Result<Vec<assay_shade::Interval>> {
    let file = File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    BedReader::new(file)
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("could not parse {}", path.display()))
}

fn print_report(report: &AuditReport, format: OutputFormat) -> Result<()> {
    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    match format {
        OutputFormat::Table => report.write_table(&mut output)?,
        OutputFormat::Json => serde_json::to_writer_pretty(&mut output, report)?,
    }
    writeln!(output)?;
    Ok(())
}

fn output_writer(path: &Path) -> Result<Box<dyn Write>> {
    if path == Path::new("-") {
        Ok(Box::new(BufWriter::new(io::stdout())))
    } else {
        let file =
            File::create(path).with_context(|| format!("could not create {}", path.display()))?;
        Ok(Box::new(BufWriter::new(file)))
    }
}

fn run() -> Result<i32> {
    match Cli::parse().command {
        Command::Audit {
            targets,
            blind,
            format,
            fail_above,
        } => {
            let report = audit(&load(&targets)?, &load(&blind)?);
            print_report(&report, format)?;
            if fail_above.is_some_and(|limit| report.summary.blind_percent > limit) {
                Ok(2)
            } else {
                Ok(0)
            }
        }
        Command::Mask {
            targets,
            blind,
            output,
        } => {
            let fragments = mask(&load(&targets)?, &load(&blind)?);
            let mut writer = output_writer(&output)?;
            for fragment in fragments {
                writeln!(writer, "{}", fragment.to_bed_line())?;
            }
            Ok(0)
        }
    }
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("Error: {error:#}");
            std::process::exit(1);
        }
    }
}
