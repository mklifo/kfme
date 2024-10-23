use anyhow::{bail, Context, Result};
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt, LE};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeyframeFile {
    pub version: u8,
    pub model_path: String,
    pub model_root: String,
    pub default_sync_transition_type: u32,
    pub default_non_sync_transition_type: u32,
    pub default_sync_transition_duration: f32,
    pub default_non_sync_transition_duration: f32,
    pub anims: Vec<Animation>,
    pub anim_layer_groups: Vec<AnimationLayerGroup>,
}

impl KeyframeFile {
    pub fn read<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        // Open the file and create a buffered reader
        let file = File::open(path).context("open file")?;
        let mut reader = BufReader::new(file);

        // Read the version of the file
        let version = reader.read_u8().context("read `version`")?;

        // Read and verify the magic string in the file header
        read_magic(&mut reader)?;

        // Read if the file is little endian
        let _ = reader.read_u8().context("read `is_little_endian`")?;

        // Read the model path
        let model_path = read_u32_len_str::<_, LE>(&mut reader).context("read `model_path`")?;

        // Read the model root
        let model_root = read_u32_len_str::<_, LE>(&mut reader).context("read `model_root`")?;

        // Read the default transition types and durations
        let default_sync_transition_type = reader
            .read_u32::<LE>()
            .context("read `default_sync_transition_type`")?;

        let default_non_sync_transition_type = reader
            .read_u32::<LE>()
            .context("read `default_non_sync_transition_type`")?;

        let default_sync_transition_duration = reader
            .read_f32::<LE>()
            .context("read `default_sync_transition_duration`")?;

        let default_non_sync_transition_duration = reader
            .read_f32::<LE>()
            .context("read `default_non_sync_transition_duration`")?;

        // Read the animations
        let num_anims = reader.read_u32::<LE>().context("read `num_anims`")? as usize;
        let mut anims = Vec::with_capacity(num_anims);
        for i in 0..num_anims {
            let item = Animation::read_kfm_bytes(&mut reader)
                .with_context(|| format!("read `anims[{}]`", i))?;
            anims.push(item);
        }

        // Read the animation layer groups
        let num_anim_layer_groups = reader
            .read_u32::<LE>()
            .context("read `num_anim_layer_groups`")? as usize;
        let mut anim_layer_groups = Vec::with_capacity(num_anim_layer_groups);
        for i in 0..num_anim_layer_groups {
            let item = AnimationLayerGroup::read_kfm_bytes(&mut reader)
                .with_context(|| format!("read `anim_layer_groups[{}]`", i))?;
            anim_layer_groups.push(item);
        }

        Ok(Self {
            version,
            model_path,
            model_root,
            default_sync_transition_type,
            default_non_sync_transition_type,
            default_sync_transition_duration,
            default_non_sync_transition_duration,
            anims,
            anim_layer_groups,
        })
    }

    pub fn write<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        // Create the file and wrap it with a buffered writer
        let file = File::create(path).context("create file")?;
        let mut writer = BufWriter::new(file);

        // Write the version
        writer.write_u8(self.version).context("write `version`")?;

        // Write the magic string
        writer.write_all(EXPECTED_MAGIC.as_bytes())?;

        // Write if the file is little endian
        writer.write_u8(1).context("write `is_little_endian`")?;

        // Write the model path and model root strings
        write_u32_len_str::<_, LE>(&mut writer, &self.model_path).context("write `model_path`")?;
        write_u32_len_str::<_, LE>(&mut writer, &self.model_root).context("write `model_root`")?;

        // Write the default transition types and durations
        writer
            .write_u32::<LE>(self.default_sync_transition_type)
            .context("write `default_sync_transition_type`")?;

        writer
            .write_u32::<LE>(self.default_non_sync_transition_type)
            .context("write `default_non_sync_transition_type`")?;

        writer
            .write_f32::<LE>(self.default_sync_transition_duration)
            .context("write `default_sync_transition_duration`")?;

        writer
            .write_f32::<LE>(self.default_non_sync_transition_duration)
            .context("write `default_non_sync_transition_duration`")?;

        // Write the animations count and data
        writer
            .write_u32::<LE>(self.anims.len() as u32)
            .context("write `num_anims`")?;
        for (i, anim) in self.anims.iter().enumerate() {
            anim.write_kfm_bytes(&mut writer)
                .with_context(|| format!("write `anims[{}]`", i))?;
        }

        // Write the animation layer groups count and data
        writer
            .write_u32::<LE>(self.anim_layer_groups.len() as u32)
            .context("write `num_anim_layer_groups`")?;
        for (i, anim_layer_group) in self.anim_layer_groups.iter().enumerate() {
            anim_layer_group
                .write_kfm_bytes(&mut writer)
                .with_context(|| format!("write `anim_layer_groups[{}]`", i))?;
        }

        Ok(())
    }

    pub fn read_yaml<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path)?;
        let mut yaml = String::new();
        file.read_to_string(&mut yaml)?;
        let result: Self = serde_yaml::from_str(&yaml)?;
        Ok(result)
    }

    pub fn write_yaml<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let yaml = serde_yaml::to_string(&self)?;
        let mut file = File::create(path)?;
        file.write_all(yaml.as_bytes())?;
        Ok(())
    }
}

pub trait ReadKfmBytes: Sized {
    fn read_kfm_bytes<R>(reader: &mut R) -> Result<Self>
    where
        R: Read;
}

pub trait WriteKfmBytes: Sized {
    fn write_kfm_bytes<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Animation {
    pub id: u32,
    pub path: String,
    pub index: u32,
    pub transitions: Vec<Transition>,
}

impl ReadKfmBytes for Animation {
    fn read_kfm_bytes<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        // Read the animation ID
        let id = reader.read_u32::<LE>().context("read `id`")?;

        // Read the animation path as a string
        let path = read_u32_len_str::<_, LE>(reader).context("read `path`")?;

        // Read the animation index
        let index = reader.read_u32::<LE>().context("read `index`")?;

        // Read the number of transitions and populate the vector
        let num_transitions = reader.read_u32::<LE>().context("read `num_transitions`")? as usize;
        let mut transitions = Vec::with_capacity(num_transitions);
        for i in 0..num_transitions {
            let item = Transition::read_kfm_bytes(reader)
                .with_context(|| format!("read `transitions[{}]`", i))?;
            transitions.push(item);
        }

        // Return the populated `Animation` struct
        Ok(Self {
            id,
            path,
            index,
            transitions,
        })
    }
}

impl WriteKfmBytes for Animation {
    fn write_kfm_bytes<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        // Write the animation ID
        writer.write_u32::<LE>(self.id).context("write `id`")?;

        // Write the animation path as a string
        write_u32_len_str::<_, LE>(writer, &self.path).context("write `path`")?;

        // Write the animation index
        writer
            .write_u32::<LE>(self.index)
            .context("write `index`")?;

        // Write the number of transitions and each transition
        writer
            .write_u32::<LE>(self.transitions.len() as u32)
            .context("write `num_transitions`")?;
        for (i, item) in self.transitions.iter().enumerate() {
            item.write_kfm_bytes(writer)
                .with_context(|| format!("write `transitions[{}]`", i))?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transition {
    pub id: u32,

    #[serde(rename = "type")]
    pub type_: TransitionType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<TransitionExt>,
}

impl ReadKfmBytes for Transition {
    fn read_kfm_bytes<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        // Read the transition ID
        let id = reader.read_u32::<LE>().context("read `id`")?;

        // Read the transition type
        let type_ = TransitionType::read_kfm_bytes(reader).context("read `type`")?;

        // Depending on the type, read the optional extension
        let ext = match type_ {
            TransitionType::DefaultSync | TransitionType::DefaultNonSync => None,
            _ => Some(TransitionExt::read_kfm_bytes(reader).context("read `ext`")?),
        };

        Ok(Self { id, type_, ext })
    }
}

impl WriteKfmBytes for Transition {
    fn write_kfm_bytes<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        // Write the transition ID
        writer.write_u32::<LE>(self.id).context("write `id`")?;

        // Write the transition type
        self.type_.write_kfm_bytes(writer).context("write `type`")?;

        // If there's an extension, write it
        if let Some(ext) = &self.ext {
            ext.write_kfm_bytes(writer).context("write `ext`")?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransitionExt {
    pub duration: f32,
    pub intermediate_anims: Vec<IntermediateAnimation>,
    pub chain_anims: Vec<ChainAnimation>,
}

impl ReadKfmBytes for TransitionExt {
    fn read_kfm_bytes<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        // Read the duration
        let duration = reader.read_f32::<LE>().context("read `duration`")?;

        // Read the number of intermediate animations and populate the vector
        let num_intermediate_anims = reader
            .read_u32::<LE>()
            .context("read `num_intermediate_anims`")?
            as usize;
        let mut intermediate_anims = Vec::with_capacity(num_intermediate_anims);
        for i in 0..num_intermediate_anims {
            let item = IntermediateAnimation::read_kfm_bytes(reader)
                .with_context(|| format!("read `intermediate_anims[{}]`", i))?;
            intermediate_anims.push(item);
        }

        // Read the number of chain animations and populate the vector
        let num_chain_anims = reader.read_u32::<LE>().context("read `num_chain_anims`")? as usize;
        let mut chain_anims = Vec::with_capacity(num_chain_anims);
        for i in 0..num_chain_anims {
            let item = ChainAnimation::read_kfm_bytes(reader)
                .with_context(|| format!("read `chain_anims[{}]`", i))?;
            chain_anims.push(item);
        }

        // Return the populated `TransitionExt` struct
        Ok(Self {
            duration,
            intermediate_anims,
            chain_anims,
        })
    }
}

impl WriteKfmBytes for TransitionExt {
    fn write_kfm_bytes<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        // Write the duration
        writer
            .write_f32::<LE>(self.duration)
            .context("write `duration`")?;

        // Write the number of intermediate animations and each item
        writer
            .write_u32::<LE>(self.intermediate_anims.len() as u32)
            .context("write `num_intermediate_anims`")?;
        for (i, item) in self.intermediate_anims.iter().enumerate() {
            item.write_kfm_bytes(writer)
                .with_context(|| format!("write `intermediate_anims[{}]`", i))?;
        }

        // Write the number of chain animations and each item
        writer
            .write_u32::<LE>(self.chain_anims.len() as u32)
            .context("write `num_chain_anims`")?;
        for (i, item) in self.chain_anims.iter().enumerate() {
            item.write_kfm_bytes(writer)
                .with_context(|| format!("write `chain_anims[{}]`", i))?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TransitionType {
    Blend,
    Morph,
    Crossfade,
    ChainAnimation,
    DefaultSync,
    DefaultNonSync,
}

impl ReadKfmBytes for TransitionType {
    fn read_kfm_bytes<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        let type_code = reader.read_u32::<LE>().context("read `type_code`")?;

        let type_ = match type_code {
            0 => Self::Blend,
            1 => Self::Morph,
            2 => Self::Crossfade,
            3 => Self::ChainAnimation,
            4 => Self::DefaultSync,
            5 => Self::DefaultNonSync,
            _ => bail!("unknown transition `type_code` `{}`", type_code),
        };

        Ok(type_)
    }
}

impl WriteKfmBytes for TransitionType {
    fn write_kfm_bytes<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        let type_code = match self {
            Self::Blend => 0,
            Self::Morph => 1,
            Self::Crossfade => 2,
            Self::ChainAnimation => 3,
            Self::DefaultSync => 4,
            Self::DefaultNonSync => 5,
        };

        writer
            .write_u32::<LE>(type_code)
            .context("write `type_code`")?;

        Ok(())
    }
}

/// Animation info.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnimationLayer {
    pub id: u32,
    pub priority: i32,
    pub weight: f32,
    pub ease_in_time: f32,
    pub ease_out_time: f32,
    pub sync_id: u32,
}

impl ReadKfmBytes for AnimationLayer {
    fn read_kfm_bytes<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        let id = reader.read_u32::<LE>().context("read `id`")?;
        let priority = reader.read_i32::<LE>().context("read `priority`")?;
        let weight = reader.read_f32::<LE>().context("read `weight`")?;
        let ease_in_time = reader.read_f32::<LE>().context("read `ease_in_time`")?;
        let ease_out_time = reader.read_f32::<LE>().context("read `ease_out_time`")?;
        let sync_id = reader.read_u32::<LE>().context("read `sync_id`")?;

        Ok(Self {
            id,
            priority,
            weight,
            ease_in_time,
            ease_out_time,
            sync_id,
        })
    }
}

impl WriteKfmBytes for AnimationLayer {
    fn write_kfm_bytes<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        writer.write_u32::<LE>(self.id).context("write `id`")?;

        writer
            .write_i32::<LE>(self.priority)
            .context("write `priority`")?;

        writer
            .write_f32::<LE>(self.weight)
            .context("write `weight`")?;

        writer
            .write_f32::<LE>(self.ease_in_time)
            .context("write `ease_in_time`")?;

        writer
            .write_f32::<LE>(self.ease_out_time)
            .context("write `ease_out_time`")?;

        writer
            .write_u32::<LE>(self.sync_id)
            .context("write `sync_id`")?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnimationLayerGroup {
    pub id: u32,
    pub name: String,
    pub anim_layers: Vec<AnimationLayer>,
}

impl ReadKfmBytes for AnimationLayerGroup {
    fn read_kfm_bytes<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        // Read the group ID
        let id = reader.read_u32::<LE>().context("read `id`")?;

        // Read the group name as a string
        let name = read_u32_len_str::<_, LE>(reader).context("read `name`")?;

        // Read the number of animation layers and populate the vector
        let num_anim_layers = reader.read_u32::<LE>().context("read `num_anim_layers`")? as usize;
        let mut anim_layers = Vec::with_capacity(num_anim_layers);
        for i in 0..num_anim_layers {
            let item = AnimationLayer::read_kfm_bytes(reader)
                .with_context(|| format!("read `anim_layers[{}]`", i))?;
            anim_layers.push(item);
        }

        // Return the populated `AnimationLayerGroup` struct
        Ok(Self {
            id,
            name,
            anim_layers,
        })
    }
}

impl WriteKfmBytes for AnimationLayerGroup {
    fn write_kfm_bytes<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        // Write the group ID
        writer.write_u32::<LE>(self.id).context("write `id`")?;

        // Write the group name as a string
        write_u32_len_str::<_, LE>(writer, &self.name).context("write `name`")?;

        // Write the number of animation layers and each layer
        writer
            .write_u32::<LE>(self.anim_layers.len() as u32)
            .context("write `num_anim_layers`")?;
        for (i, item) in self.anim_layers.iter().enumerate() {
            item.write_kfm_bytes(writer)
                .with_context(|| format!("write `anim_layers[{}]`", i))?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IntermediateAnimation {
    pub start_key: String,
    pub target_key: String,
}

impl ReadKfmBytes for IntermediateAnimation {
    fn read_kfm_bytes<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        let start_key = read_u32_len_str::<_, LE>(reader).context("read `start_key`")?;
        let target_key = read_u32_len_str::<_, LE>(reader).context("read `target_key`")?;
        Ok(Self {
            start_key,
            target_key,
        })
    }
}

impl WriteKfmBytes for IntermediateAnimation {
    fn write_kfm_bytes<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        write_u32_len_str::<_, LE>(writer, &self.start_key).context("write `start_key`")?;
        write_u32_len_str::<_, LE>(writer, &self.target_key).context("write `target_key`")?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChainAnimation {
    pub id: u32,
    pub duration: f32,
}

impl ReadKfmBytes for ChainAnimation {
    fn read_kfm_bytes<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        let id = reader.read_u32::<LE>().context("read `id`")?;
        let duration = reader.read_f32::<LE>().context("read `duration`")?;
        Ok(Self { id, duration })
    }
}

impl WriteKfmBytes for ChainAnimation {
    fn write_kfm_bytes<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        writer.write_u32::<LE>(self.id).context("write `id`")?;
        writer
            .write_f32::<LE>(self.duration)
            .context("write `duration`")?;
        Ok(())
    }
}

/// Expected string in the header of all `.kfm` files.
const EXPECTED_MAGIC: &str = "Gamebryo KFM File Version 2.2.0.0b\n";

/// Reads and verifies the magic string from the given reader.
fn read_magic<R>(reader: &mut R) -> Result<()>
where
    R: Read,
{
    let mut buffer = vec![0u8; EXPECTED_MAGIC.len()];
    reader.read_exact(&mut buffer)?;

    let magic = String::from_utf8(buffer)?;
    if magic == EXPECTED_MAGIC {
        Ok(())
    } else {
        bail!("unexpected magic `{}`", magic);
    }
}

/// Reads an ASCII string with a prefixed `u32` length specifier.
fn read_u32_len_str<R, O>(reader: &mut R) -> Result<String>
where
    R: Read,
    O: ByteOrder,
{
    let len = reader.read_u32::<O>()? as usize;

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;

    let s = String::from_utf8(buf)?;
    Ok(s)
}

/// Writes an ASCII string with a prefixed `u32` length specifier.
fn write_u32_len_str<W, O>(writer: &mut W, s: &str) -> Result<()>
where
    W: Write,
    O: ByteOrder,
{
    if !s.is_ascii() {
        bail!("`{s}` is not ascii");
    }

    writer.write_u32::<O>(s.len() as u32)?;
    writer.write_all(s.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{read_u32_len_str, write_u32_len_str};
    use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
    use std::io::{Cursor, Read, Write};

    #[test]
    fn test_read_u32_len_str_with_valid_string() {
        // Ensure success when reading valid input
        {
            let input = "testing";
            let mut cursor = Cursor::new(Vec::new());

            cursor
                .write_u32::<LittleEndian>(input.len() as u32)
                .expect("failed to write string length");
            cursor
                .write_all(input.as_bytes())
                .expect("failed to write string bytes");
            cursor.set_position(0);

            let result = read_u32_len_str::<_, LittleEndian>(&mut cursor)
                .expect("expected `read_u32_len_str` to succeed with a valid string");
            assert_eq!(result, input, "string content mismatch");
        }

        // Ensure failure when length exceeds the available data
        {
            let mut cursor = Cursor::new(vec![0, 0, 0, 5]);
            let result = read_u32_len_str::<_, LittleEndian>(&mut cursor);
            assert!(
                result.is_err(),
                "expected `read_u32_len_str` to fail with insufficient data"
            );
        }
    }

    #[test]
    fn test_write_u32_len_str() {
        // Ensure success writing valid input
        {
            let input = "testing";
            let mut cursor = Cursor::new(Vec::new());

            write_u32_len_str::<_, LittleEndian>(&mut cursor, input)
                .expect("expected `write_u32_len_str` to succeed with a valid ASCII string");

            cursor.set_position(0);
            let length = cursor
                .read_u32::<LittleEndian>()
                .expect("failed to read length");
            assert_eq!(length as usize, input.len(), "length mismatch");

            let mut buffer = vec![0u8; input.len()];
            cursor
                .read_exact(&mut buffer)
                .expect("failed to read string data");
            let actual = String::from_utf8(buffer).expect("invalid UTF-8 sequence");
            assert_eq!(input, actual, "string content mismatch");
        }

        // Ensure failure writing non-ASCII characters
        {
            let input = "测试字符串";
            let mut cursor = Cursor::new(Vec::new());

            let result = write_u32_len_str::<_, LittleEndian>(&mut cursor, input);
            assert!(
                result.is_err(),
                "expected `write_u32_len_str` to fail with non-ASCII string"
            );
        }
    }
}
