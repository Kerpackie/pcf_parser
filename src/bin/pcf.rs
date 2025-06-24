use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use anyhow::{Context, Result};

use pcf_parser::{
    parse_pcf_file, write_pcf_file,
    hex_dump_file, diff_files, diff_blocks, PatternFileData,
};

/// PCF – pattern-file command-line toolkit
#[derive(Parser)]
#[command(name = "pcf", version, about = "Read, diff and write .pcf files", arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Pretty-prints the header / metadata
    Parse {
        /// Path to the .pcf file
        file: PathBuf,

        /// Emit as JSON
        #[arg(long)]
        json: bool,
    },

    /// Hex-dumps the entire file
    Dump {
        /// Path to the .pcf file
        file: PathBuf,

        /// Bytes per line
        #[arg(long, default_value_t = 16, value_parser = parse_byte_range)]
        bytes: usize,
    },

    /// Byte-by-byte diff
    Diff {
        file_a: PathBuf,
        file_b: PathBuf,

        /// Show N bytes before/after mismatch
        #[arg(long, default_value_t = 8)]
        context: usize,
    },

    /// Block diff (18-byte rows)
    DiffBlocks {
        file_a: PathBuf,
        file_b: PathBuf,

        /// Bytes per block (default 18 for PCF pattern row)
        #[arg(long, default_value_t = 18)]
        block: usize,

        /// Max mismatched blocks to show
        #[arg(long, default_value_t = 10)]
        max: usize,
    },

    /// Rewrite: JSON → PCF (for round-trip experiments)
    Write {
        /// Path to .json input file
        json_in: PathBuf,

        /// Path to output .pcf file
        pcf_out: PathBuf,
    },
}

/// Accepts a string, parses to usize, and enforces 1..=64
fn parse_byte_range(s: &str) -> Result<usize, String> {
    let val: usize = s
        .parse()
        .map_err(|_| format!("`{}` isn’t a number", s))?;
    if (1..=64).contains(&val) {
        Ok(val)
    } else {
        Err(format!("must be in range 1..=64 (got {})", val))
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::Parse { file, json } => {
            let data = parse_pcf_file(&file)
                .with_context(|| format!("Failed to parse {:?}", file))?;

            if json {
                let output = serde_json::to_string_pretty(&data)?;
                println!("{output}");
            } else {
                println!("{:#?}", data);
            }
        }

        Command::Dump { file, bytes } => {
            hex_dump_file(&file, bytes)?;
        }

        Command::Diff { file_a, file_b, context } => {
            diff_files(&file_a, &file_b, context)?;
        }

        Command::DiffBlocks { file_a, file_b, block, max } => {
            diff_blocks(&file_a, &file_b, block, max)?;
        }

        Command::Write { json_in, pcf_out } => {
            let text = std::fs::read_to_string(&json_in)
                .with_context(|| format!("Reading {:?}", json_in))?;

            let data: PatternFileData = serde_json::from_str(&text)
                .with_context(|| "Failed to deserialize JSON")?;

            if data.clk_sources.len() != 65 {
                anyhow::bail!(
                    "clk_sources must have exactly 65 elements (found {})",
                    data.clk_sources.len()
                );
            }

            write_pcf_file(&pcf_out, &data)
                .with_context(|| format!("Writing {:?}", pcf_out))?;

            println!("{}", "Wrote PCF file".green());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parse_command() {
        let args = ["pcf", "parse", "file.pcf"];
        let cli = Cli::parse_from(&args);
        match cli.cmd {
            Command::Parse { file, json } => {
                assert_eq!(file, PathBuf::from("file.pcf"));
                assert!(!json);
            },
            _ => panic!("Expected Parse command"),
        }
    }

    #[test]
    fn test_cli_parse_command_with_json() {
        let args = ["pcf", "parse", "file.pcf", "--json"];
        let cli = Cli::parse_from(&args);
        match cli.cmd {
            Command::Parse { file, json } => {
                assert_eq!(file, PathBuf::from("file.pcf"));
                assert!(json);
            },
            _ => panic!("Expected Parse command with --json"),
        }
    }

    #[test]
    fn test_cli_dump_command() {
        let args = ["pcf", "dump", "file.pcf", "--bytes", "32"];
        let cli = Cli::parse_from(&args);
        match cli.cmd {
            Command::Dump { file, bytes } => {
                assert_eq!(file, PathBuf::from("file.pcf"));
                assert_eq!(bytes, 32);
            },
            _ => panic!("Expected Dump command"),
        }
    }

    #[test]
    fn test_cli_diff_command() {
        let args = ["pcf", "diff", "a.pcf", "b.pcf", "--context", "4"];
        let cli = Cli::parse_from(&args);
        match cli.cmd {
            Command::Diff { file_a, file_b, context } => {
                assert_eq!(file_a, PathBuf::from("a.pcf"));
                assert_eq!(file_b, PathBuf::from("b.pcf"));
                assert_eq!(context, 4);
            },
            _ => panic!("Expected Diff command"),
        }
    }

    #[test]
    fn test_cli_diffblocks_command() {
        let args = ["pcf", "diff-blocks", "a.pcf", "b.pcf", "--block", "20", "--max", "2"];
        let cli = Cli::parse_from(&args);
        match cli.cmd {
            Command::DiffBlocks { file_a, file_b, block, max } => {
                assert_eq!(file_a, PathBuf::from("a.pcf"));
                assert_eq!(file_b, PathBuf::from("b.pcf"));
                assert_eq!(block, 20);
                assert_eq!(max, 2);
            },
            _ => panic!("Expected DiffBlocks command"),
        }
    }

    #[test]
    fn test_cli_write_command() {
        let args = ["pcf", "write", "input.json", "output.pcf"];
        let cli = Cli::parse_from(&args);
        match cli.cmd {
            Command::Write { json_in, pcf_out } => {
                assert_eq!(json_in, PathBuf::from("input.json"));
                assert_eq!(pcf_out, PathBuf::from("output.pcf"));
            },
            _ => panic!("Expected Write command"),
        }
    }
}
