use crate::regex_or::RegexOr;
use crate::source::{Animation, TransitionExt, TransitionType};
use crate::source::{MappedSource, MappedTransition};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PatchFile {
    pub anims: Vec<AnimationPatch>,
}

impl PatchFile {
    /// Loads a `PatchFile` from a given file path.
    pub fn load<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path).context("open file")?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Creates a `PatchFile` from a reader.
    pub fn from_reader<R>(reader: R) -> Result<Self>
    where
        R: Read,
    {
        let f: Self = serde_yaml::from_reader(reader)?;
        Ok(f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnimationPatch {
    #[serde(flatten)]
    pub body: AnimationPatchBody,
}

impl<T> From<T> for AnimationPatch
where
    T: Into<AnimationPatchBody>,
{
    fn from(from: T) -> Self {
        Self { body: from.into() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AnimationPatchBody {
    Add(AddAnimation),
    Delete(DeleteAnimation),
    Update(UpdateAnimation),
}

impl From<AddAnimation> for AnimationPatchBody {
    fn from(from: AddAnimation) -> Self {
        Self::Add(from)
    }
}

impl From<DeleteAnimation> for AnimationPatchBody {
    fn from(from: DeleteAnimation) -> Self {
        Self::Delete(from)
    }
}

impl From<UpdateAnimation> for AnimationPatchBody {
    fn from(from: UpdateAnimation) -> Self {
        Self::Update(from)
    }
}

pub type AddAnimation = Animation;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeleteAnimation {
    pub id: RegexOr<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateAnimation {
    pub id: RegexOr<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trans: Option<Vec<TransitionPatch>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransitionPatch {
    #[serde(flatten)]
    pub body: TransitionPatchBody,
}

impl<T> From<T> for TransitionPatch
where
    T: Into<TransitionPatchBody>,
{
    fn from(from: T) -> Self {
        Self { body: from.into() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TransitionPatchBody {
    Add(AddTransition),
    Delete(DeleteTransition),
    Update(UpdateTransition),
}

impl From<AddTransition> for TransitionPatchBody {
    fn from(from: AddTransition) -> Self {
        Self::Add(from)
    }
}

impl From<DeleteTransition> for TransitionPatchBody {
    fn from(from: DeleteTransition) -> Self {
        Self::Delete(from)
    }
}

impl From<UpdateTransition> for TransitionPatchBody {
    fn from(from: UpdateTransition) -> Self {
        Self::Update(from)
    }
}

/// An instruction to add a transition to an animation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddTransition {
    pub id: RegexOr<u32>,

    #[serde(rename = "type")]
    pub type_: TransitionType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<TransitionExt>,
}

/// An instruction to delete an existing transition of an animation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeleteTransition {
    pub id: RegexOr<u32>,
}

/// An instruction to update the data an existing transition of an animation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateTransition {
    pub id: RegexOr<u32>,

    #[serde(rename = "type")]
    pub type_: Option<TransitionType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<TransitionExt>,
}

pub fn apply(m_src: &mut MappedSource, patch_file: PatchFile) -> Result<()> {
    for anim_patch in patch_file.anims.into_iter() {
        match anim_patch.body {
            AnimationPatchBody::Add(a) => on_add_anim(m_src, a)?,
            AnimationPatchBody::Delete(d) => on_delete_anim(m_src, d)?,
            AnimationPatchBody::Update(u) => on_update_anim(m_src, u)?,
        }
    }

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

    Ok(())
}

fn on_add_tran(m_src: &mut MappedSource, parent_anim_id: u32, add: &AddTransition) -> Result<()> {
    // Find all transition ids to add to the parent animation
    let all_anim_ids = m_src.anims.keys().cloned();
    let mut add_tran_ids: HashSet<_> = collect_matching_ids(all_anim_ids, add.id.clone());
    add_tran_ids.remove(&parent_anim_id);

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
                "anim `{}` already has tran to `{}`",
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
                "anim `{}` did not have a tran to `{}`",
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

#[cfg(test)]
mod tests {
    use super::{apply, PatchFile};
    use super::{AddAnimation, UpdateAnimation};
    use super::{AddTransition, DeleteTransition};
    use crate::regex_or::RegexOr;
    use crate::source::{DefaultTransitions, Model, Transition, TransitionType};
    use crate::source::{MappedAnimation, MappedSource, MappedTransition};
    use indoc::indoc;
    use std::collections::BTreeMap;

    #[test]
    fn test_patch_file_ser() {
        let patch_file = PatchFile {
            anims: vec![
                AddAnimation {
                    id: 0,
                    path: "foo/bar".to_string(),
                    index: 0,
                    trans: vec![Transition {
                        id: 1,
                        type_: TransitionType::Blend,
                        ext: None,
                    }],
                }
                .into(),
                UpdateAnimation {
                    id: RegexOr::Other(1),
                    path: None,
                    index: Some(2),
                    trans: Some(vec![
                        DeleteTransition {
                            id: RegexOr::Regex(".*".try_into().unwrap()),
                        }
                        .into(),
                        AddTransition {
                            id: RegexOr::Other(3),
                            type_: TransitionType::ChainAnimation,
                            ext: None,
                        }
                        .into(),
                    ]),
                }
                .into(),
            ],
        };

        let expected = indoc! {"
            anims:
            - add:
                id: 0
                path: foo/bar
                index: 0
                trans:
                - id: 1
                  type: blend
            - update:
                id: 1
                index: 2
                trans:
                - delete:
                    id: /.*/
                - add:
                    id: 3
                    type: chain_animation
        "};

        let actual = serde_yaml::to_string(&patch_file).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_patch_file_apply() {
        let mut m_src = MappedSource {
            model: Model {
                path: "./../../mesh/newenemies/mech_order_darkling_1.nif".to_string(),
                root: "Accumulation_Root".to_string(),
            },
            default_trans: DefaultTransitions {
                sync_type: TransitionType::Morph,
                sync_duration: 0.25,
                non_sync_type: TransitionType::Blend,
                non_sync_duration: 0.25,
            },
            anims: BTreeMap::new(),
            layer_groups: Vec::new(),
        };
        let m_src_anim_paths = [
            "./mech/mech_gunbot_m_idle.kf",
            "./mech/mech_gunbot_m_run.kf",
            "./mech/mech_gunbot_a_attack.kf",
            "./mech/mech_gunbot_h_onhit.kf",
        ];
        for (id, path) in m_src_anim_paths.iter().enumerate() {
            // Create mapped animation from `id` and `path`
            let mut m_anim = MappedAnimation {
                path: path.to_string(),
                index: 0,
                trans: BTreeMap::new(),
            };
            // Insert transition to every other animation
            for (trans_id, _) in m_src_anim_paths.iter().enumerate() {
                if trans_id != id {
                    m_anim.trans.insert(
                        trans_id as u32,
                        MappedTransition {
                            type_: TransitionType::DefaultNonSync,
                            ext: None,
                        },
                    );
                }
            }
            m_src.anims.insert(id as u32, m_anim);
        }

        let patch_file = PatchFile {
            anims: vec![
                // Add `ondie` animation
                AddAnimation {
                    id: 4,
                    path: "./mech/mech_gunbot_h_ondie.kf".to_string(),
                    index: 0,
                    trans: Vec::new(),
                }
                .into(),
                // Add transition from every animation to `ondie`
                UpdateAnimation {
                    id: RegexOr::Regex(".*".try_into().unwrap()),
                    path: None,
                    index: None,
                    trans: Some(vec![AddTransition {
                        id: RegexOr::Other(4),
                        type_: TransitionType::DefaultNonSync,
                        ext: None,
                    }
                    .into()]),
                }
                .into(),
                // Add `spawn` animation
                AddAnimation {
                    id: 5,
                    path: "./mech/mech_gunbot_m_spawn.kf".to_string(),
                    index: 0,
                    trans: Vec::new(),
                }
                .into(),
                // Add transition from `spawn` to every other animation
                UpdateAnimation {
                    id: RegexOr::Other(5),
                    path: None,
                    index: None,
                    trans: Some(vec![AddTransition {
                        id: RegexOr::Regex(".*".try_into().unwrap()),
                        type_: TransitionType::DefaultNonSync,
                        ext: None,
                    }
                    .into()]),
                }
                .into(),
                // Delete transition from `spawn` to `ondie`
                UpdateAnimation {
                    id: RegexOr::Other(5),
                    path: None,
                    index: None,
                    trans: Some(vec![DeleteTransition {
                        id: RegexOr::Other(4),
                    }
                    .into()]),
                }
                .into(),
                // Add transition from `ondie` to `spawn`
                UpdateAnimation {
                    id: RegexOr::Other(4),
                    path: None,
                    index: None,
                    trans: Some(vec![AddTransition {
                        id: RegexOr::Other(5),
                        type_: TransitionType::DefaultNonSync,
                        ext: None,
                    }
                    .into()]),
                }
                .into(),
            ],
        };

        // Apply patch to source
        apply(&mut m_src, patch_file).unwrap();

        // Ensure all animations have expected transition ids
        assert_trans_ids_eq(&m_src, 0, &[1, 2, 3, 4]);
        assert_trans_ids_eq(&m_src, 1, &[0, 2, 3, 4]);
        assert_trans_ids_eq(&m_src, 2, &[0, 1, 3, 4]);
        assert_trans_ids_eq(&m_src, 3, &[0, 1, 2, 4]);
        assert_trans_ids_eq(&m_src, 4, &[5]);
        assert_trans_ids_eq(&m_src, 5, &[0, 1, 2, 3]);
    }

    fn assert_trans_ids_eq(m_src: &MappedSource, anim_id: u32, expected_ids: &[u32]) {
        let anim = match m_src.anims.get(&anim_id) {
            Some(a) => a,
            None => panic!("anim `{}` not found", anim_id),
        };

        let actual_ids: Vec<u32> = anim.trans.keys().cloned().collect();
        if actual_ids != expected_ids {
            panic!(
                "mismatch in trans ids for anim `{}`.\n\
                 expected: `{:?}`\n\
                 actual: `{:?}`",
                anim_id, expected_ids, actual_ids
            );
        }
    }
}
