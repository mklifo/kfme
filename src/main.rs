pub mod bin;
pub mod header;
pub mod patch;
pub mod regex_or;
pub mod source;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use header::make_header;
use patch::PatchFile;
use source::MappedSource;
use source::SourceFile;
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Applies a patch to the given source file
    Patch {
        #[arg(long, short)]
        src: PathBuf,

        #[arg(long, short)]
        patch: PathBuf,
    },

    /// Converts the format of a given source file
    Convert {
        #[arg(long, short)]
        input: PathBuf,

        #[arg(long, short)]
        output: Option<PathBuf>,
    },

    /// Builds a binary and a corresponding header file from the given source file
    Build {
        #[arg(long, short)]
        input: PathBuf,

        #[arg(long, short)]
        output_dir: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Patch { src, patch } => on_patch(src, patch),
        Commands::Convert { input, output } => on_convert(input, output),
        Commands::Build { input, output_dir } => on_build(input, output_dir),
    }
}

fn on_patch(src_path: PathBuf, patch_path: PathBuf) -> Result<()> {
    let src_file = SourceFile::load(&src_path).context("load source file")?;
    let patch_file = PatchFile::load(patch_path).context("load patch file")?;

    // Map source for more efficient edits
    let mut m_src = MappedSource::try_from(src_file.body)?;

    // Apply patch
    patch::apply(&mut m_src, patch_file).context("apply patch")?;

    // Unmap source to embed in file
    let new_src_file = SourceFile {
        header: src_file.header,
        body: m_src.into(),
    };

    // Save source file
    new_src_file.save(src_path).context("save source file")?;

    Ok(())
}

fn on_convert(input_path: PathBuf, maybe_output_path: Option<PathBuf>) -> Result<()> {
    let input_file = SourceFile::load(&input_path).context("load input file")?;
    let output_file = input_file;

    // Determine the output file path.
    // If a path is provided, use it; otherwise, derive it from the input file path.
    let output_file_path = match maybe_output_path {
        Some(p) => p,
        None => {
            let mut p = input_path.clone();
            let p_old_ext = p
                .extension()
                .and_then(|s| s.to_str())
                .context("extension unreadable")?;
            let p_new_ext = match p_old_ext {
                "yaml" | "yml" => "kfm",
                "kfm" => "yaml",
                _ => bail!("unsupported extension"),
            };
            p.set_extension(p_new_ext);
            p
        }
    };

    // Save source file
    output_file
        .save(output_file_path)
        .context("save output file")
}

fn on_build(input_path: PathBuf, maybe_output_dir_path: Option<PathBuf>) -> Result<()> {
    let input_file = SourceFile::load(&input_path).context("load input file")?;

    let output_src_file_stem = input_path
        .file_stem()
        .context("file stem")?
        .to_string_lossy()
        .to_string();

    // Determine the output directory path.
    // If a path is provided, use it; otherwise, derive it from the input file path.
    let output_dir_path = match maybe_output_dir_path {
        Some(p) => {
            if !p.is_dir() {
                bail!("path `{:?}` is not a directory", p);
            } else {
                p
            }
        }
        None => input_path
            .ancestors()
            .nth(1)
            .unwrap_or(&input_path)
            .to_path_buf(),
    };

    // Make binary source file.
    let output_src_file = input_file;

    // Make binary source file path.
    let mut output_src_path = output_dir_path.join(&output_src_file_stem);
    output_src_path.set_extension("kfm");

    // Save binary source file.
    output_src_file
        .save(output_src_path)
        .context("save output src file")?;

    // Make header file contents.
    let output_header_file_contents =
        make_header(&output_src_file_stem, &output_src_file.body.anims)?;

    // Make header file path.
    let mut output_header_file_path = output_dir_path.join(&output_src_file_stem);
    output_header_file_path.set_extension("h");

    // Save header file.
    std::fs::write(output_header_file_path, output_header_file_contents)
        .context("write output header file")?;

    Ok(())
}
