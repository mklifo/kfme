pub mod kfm;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use kfm::KeyframeFile;
use std::path::{Path, PathBuf};

#[derive(Parser)]
struct Cli {
    /// The conversion type
    conversion: Conversion,

    /// The input file (either KFM or YAML depending on the conversion type)
    #[arg(short, long, value_name = "INPUT_FILE")]
    input: PathBuf,

    /// The output file (optional, will be auto-generated if not provided)
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    output: Option<PathBuf>,
}

#[derive(ValueEnum, Clone)]
enum Conversion {
    ToYaml,
    FromYaml,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.conversion {
        Conversion::ToYaml => {
            let output_path = cli
                .output
                .clone()
                .unwrap_or_else(|| change_extension(&cli.input, "yaml"));
            let file = KeyframeFile::read(&cli.input).context("failed to read kfm file")?;
            file.write_yaml(&output_path)
                .context("failed to write yaml file")?;
            println!("converted to YAML and saved to {:?}", output_path);
        }
        Conversion::FromYaml => {
            let output_path = cli
                .output
                .clone()
                .unwrap_or_else(|| change_extension(&cli.input, "kfm"));
            let file = KeyframeFile::read_yaml(&cli.input).context("Failed to read YAML file")?;
            file.write(&output_path)
                .context("failed to write kfm file")?;
            println!("converted to KFM and saved to {:?}", output_path);
        }
    }

    Ok(())
}

/// Helper function to change the extension of a file path
fn change_extension(input: &Path, new_extension: &str) -> PathBuf {
    let mut new_path = input.to_path_buf();
    new_path.set_extension(new_extension);
    new_path
}
