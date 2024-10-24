use crate::source::{Animation, TransitionExt, TransitionType};
use crate::RegexOr;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AnimationPatchBody {
    Add(AddAnimation),
    Delete(DeleteAnimation),
    Update(UpdateAnimation),
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TransitionPatchBody {
    Add(AddTransition),
    Delete(DeleteTransition),
    Update(UpdateTransition),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Transition;
    use indoc::indoc;

    #[test]
    fn test_patchfile_ser() {
        let data = PatchFile {
            anims: vec![
                AnimationPatch {
                    body: AnimationPatchBody::Add(AddAnimation {
                        id: 0,
                        path: "foo/bar".to_string(),
                        index: 0,
                        trans: vec![Transition {
                            id: 1,
                            type_: TransitionType::Blend,
                            ext: None,
                        }],
                    }),
                },
                AnimationPatch {
                    body: AnimationPatchBody::Update(UpdateAnimation {
                        id: RegexOr::Other(1),
                        path: None,
                        index: Some(2),
                        trans: Some(vec![
                            TransitionPatch {
                                body: TransitionPatchBody::Delete(DeleteTransition {
                                    id: RegexOr::Regex(".*".try_into().unwrap()),
                                }),
                            },
                            TransitionPatch {
                                body: TransitionPatchBody::Add(AddTransition {
                                    id: RegexOr::Other(3),
                                    type_: TransitionType::ChainAnimation,
                                    ext: None,
                                }),
                            },
                        ]),
                    }),
                },
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

        let actual = serde_yaml::to_string(&data).unwrap();
        assert_eq!(expected, actual);
    }
}
