pub mod kfm;
pub mod patch;
pub mod regex_or;
pub mod source;

pub use regex_or::RegexOr;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use patch::PatchFile;
use patch::{AddAnimation, AnimationPatchBody, DeleteAnimation, UpdateAnimation};
use patch::{AddTransition, DeleteTransition, TransitionPatchBody, UpdateTransition};
use source::SourceFile;
use source::{MappedSource, MappedTransition};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Converts a source file's format (inferred from file extensions)
    Convert {
        /// Input source file
        #[arg(long, short)]
        input: PathBuf,

        /// Output source file
        #[arg(long, short)]
        output: PathBuf,
    },

    /// Applies a patch to a given source file
    Patch {
        /// Source file
        src: PathBuf,

        /// Patch file
        patch: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Convert { input, output } => on_convert(input, output),
        Commands::Patch { src, patch } => on_patch(src, patch),
    }
}

pub fn on_convert(input_path: PathBuf, output_path: PathBuf) -> Result<()> {
    let input_file = SourceFile::load(&input_path).context("load input file")?;
    input_file.save(output_path).context("save output file")
}

pub fn on_patch(src_path: PathBuf, patch_path: PathBuf) -> Result<()> {
    let src_file = SourceFile::load(&src_path).context("load source file")?;
    let patch_file = PatchFile::load(patch_path).context("load patch file")?;

    // Map source for more efficient edits
    let mut m_src = MappedSource::try_from(src_file.body)?;

    for anim_patch in patch_file.anims.into_iter() {
        match anim_patch.body {
            AnimationPatchBody::Add(a) => on_add_anim(&mut m_src, a)?,
            AnimationPatchBody::Delete(d) => on_delete_anim(&mut m_src, d)?,
            AnimationPatchBody::Update(u) => on_update_anim(&mut m_src, u)?,
        }
    }

    // Unmap then save source
    let new_src_file = SourceFile {
        header: src_file.header,
        body: m_src.into(),
    };
    new_src_file.save(src_path).context("save source file")?;

    Ok(())
}

fn on_add_anim(m_src: &mut MappedSource, add: AddAnimation) -> Result<()> {
    let (m_id, m_anim) = add.try_into().context("map anim")?;

    // If an animation of the same id already existed, fail
    let old = m_src.anims.insert(m_id, m_anim);
    if old.is_some() {
        bail!("anim `{}` already exists", m_id);
    }

    Ok(())
}

fn on_delete_anim(m_src: &mut MappedSource, delete: DeleteAnimation) -> Result<()> {
    let all_ids = m_src.anims.keys().cloned();
    let delete_ids: HashSet<_> = collect_matching_ids(all_ids, delete.id);

    // Delete matching animations
    m_src.anims.retain(|id, _| !delete_ids.contains(id));

    // Delete transitions to matching animations
    for anim in m_src.anims.values_mut() {
        anim.trans.retain(|id, _| !delete_ids.contains(id));
    }

    Ok(())
}

fn on_update_anim(m_src: &mut MappedSource, update: UpdateAnimation) -> Result<()> {
    let all_ids = m_src.anims.keys().cloned();
    let update_ids: HashSet<_> = collect_matching_ids(all_ids, update.id);

    for update_id in update_ids.iter() {
        let anim = match m_src.anims.get_mut(update_id) {
            Some(a) => a,
            None => bail!("get anim `{}`", update_id),
        };

        // Update animation path
        if let Some(path) = &update.path {
            anim.path = path.clone();
        }

        // Update animation index
        if let Some(index) = update.index {
            anim.index = index;
        }

        // Update animation transitions
        if let Some(trans) = &update.trans {
            for tran in trans.iter() {
                match &tran.body {
                    TransitionPatchBody::Add(a) => on_add_tran(m_src, *update_id, a)?,
                    TransitionPatchBody::Delete(d) => on_delete_tran(m_src, *update_id, d)?,
                    TransitionPatchBody::Update(u) => on_update_tran(m_src, *update_id, u)?,
                }
            }
        }
    }

    todo!();
}

fn on_add_tran(m_src: &mut MappedSource, parent_anim_id: u32, add: &AddTransition) -> Result<()> {
    // Find all transition ids to add to the parent animation
    let all_anim_ids = m_src.anims.keys().cloned();
    let add_tran_ids: HashSet<_> = collect_matching_ids(all_anim_ids, add.id.clone());

    let parent_anim = match m_src.anims.get_mut(&parent_anim_id) {
        Some(a) => a,
        None => bail!("get parent anim `{}`", parent_anim_id),
    };

    for tran_id in add_tran_ids.into_iter() {
        let tran = MappedTransition {
            type_: add.type_,
            ext: add.ext.clone(),
        };

        // Add transition to parent animation
        let old = parent_anim.trans.insert(tran_id, tran);
        if old.is_some() {
            bail!(
                "anim `{}` already has trans to `{}`",
                parent_anim_id,
                tran_id
            );
        }
    }

    Ok(())
}

fn on_delete_tran(
    m_src: &mut MappedSource,
    parent_anim_id: u32,
    delete: &DeleteTransition,
) -> Result<()> {
    let parent_anim = match m_src.anims.get_mut(&parent_anim_id) {
        Some(a) => a,
        None => bail!("get parent anim `{}`", parent_anim_id),
    };

    // Find all transition ids to remove from the parent animation
    let all_tran_ids = parent_anim.trans.keys().cloned();
    let delete_tran_ids: HashSet<_> = collect_matching_ids(all_tran_ids, delete.id.clone());

    for tran_id in delete_tran_ids.into_iter() {
        // Remove transition from parent animation
        let old = parent_anim.trans.remove(&tran_id);
        if old.is_none() {
            bail!(
                "anim `{}` did not have a trans to `{}`",
                parent_anim_id,
                tran_id
            );
        }
    }

    Ok(())
}

fn on_update_tran(
    m_src: &mut MappedSource,
    parent_anim_id: u32,
    update: &UpdateTransition,
) -> Result<()> {
    let parent_anim = match m_src.anims.get_mut(&parent_anim_id) {
        Some(a) => a,
        None => bail!("get parent anim `{}`", parent_anim_id),
    };

    // Find all transition ids to update from the parent animation
    let all_tran_ids = parent_anim.trans.keys().cloned();
    let update_tran_ids: HashSet<_> = collect_matching_ids(all_tran_ids, update.id.clone());

    for tran_id in update_tran_ids.into_iter() {
        let tran = match parent_anim.trans.get_mut(&tran_id) {
            Some(t) => t,
            None => bail!("get tran `{}`", tran_id),
        };

        // Update transition type of parent animation
        if let Some(type_) = update.type_ {
            tran.type_ = type_;
        }

        // Update transition ext of parent animation
        tran.ext = update.ext.clone();
    }

    Ok(())
}

fn collect_matching_ids<I, T>(iter: I, value: RegexOr<u32>) -> T
where
    I: Iterator<Item = u32>,
    T: FromIterator<u32>,
{
    match value {
        RegexOr::Regex(re) => iter.filter(|i| re.is_match(&i.to_string())).collect(),
        RegexOr::Other(o) => iter.filter(|i| *i == o).collect(),
    }
}
