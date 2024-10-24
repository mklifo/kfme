use super::kfm::{Decode, ReadValueExt};
use super::kfm::{Encode, WriteValueExt};
use anyhow::{bail, Context, Error, Result};
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

/// Represents a source file that can be loaded and parsed from various formats.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceFile {
    pub header: SourceFileHeader,
    pub body: SourceFileBody,
}

impl SourceFile {
    /// Loads a `SourceFile` from a given file path, inferring the format based on the file extension.
    ///
    /// Supported extensions include `.kfm`, `.yaml`, and `.yml`. Returns an `Err` if the file
    /// extension is not recognized or the file cannot be read.
    pub fn load<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .context("extension unreadable")?;

        let file = File::open(path).context("open file")?;
        let reader = BufReader::new(file);

        match extension {
            "kfm" => Self::from_kfm_reader(reader),
            "yaml" | "yml" => Self::from_yaml_reader(reader),
            _ => bail!("unknown extension"),
        }
    }

    /// Saves a `SourceFile` to a given file path, inferring the format based on the file extension.
    ///
    /// Supported extensions include `.kfm`, `.yaml`, and `.yml`. Returns an `Err` if the file
    /// extension is not recognized or the file cannot be written.
    pub fn save<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .context("extension unreadable")?;

        let file = File::create(path).context("create file")?;
        let writer = BufWriter::new(file);

        match extension {
            "kfm" => self.to_kfm_writer(writer),
            "yaml" | "yml" => self.to_yaml_writer(writer),
            _ => bail!("unknown extension"),
        }
    }

    /// Creates a `SourceFile` from a KFM format reader.
    ///
    /// Reads the data from the provided reader and parses it according to the KFM format,
    /// respecting the file's endianness.
    pub fn from_kfm_reader<R>(mut reader: R) -> Result<Self>
    where
        R: Read,
    {
        let header = decode_source_file_header(&mut reader).context("read header")?;

        let body = if header.is_little_endian {
            reader
                .read_value::<SourceFileBody, LittleEndian>()
                .context("read body")?
        } else {
            reader
                .read_value::<SourceFileBody, BigEndian>()
                .context("read body")?
        };

        Ok(Self { header, body })
    }

    /// Creates a `SourceFile` from a YAML format reader.
    ///
    /// Parses the YAML data from the provided reader into the `SourceFile` structure.
    pub fn from_yaml_reader<R>(reader: R) -> Result<Self>
    where
        R: Read,
    {
        let f: Self = serde_yaml::from_reader(reader)?;
        Ok(f)
    }

    /// Writes the `SourceFile` data in KFM format to the given writer.
    ///
    /// The encoding respects the endianness specified in the file's header.
    pub fn to_kfm_writer<W>(&self, mut writer: W) -> Result<()>
    where
        W: Write,
    {
        encode_source_file_header(&mut writer, &self.header).context("encode header")?;

        if self.header.is_little_endian {
            self.body
                .encode_kfm::<_, LittleEndian>(&mut writer)
                .context("encode body")?;
        } else {
            self.body
                .encode_kfm::<_, BigEndian>(&mut writer)
                .context("encode body")?;
        }

        Ok(())
    }

    /// Writes the `SourceFile` data in YAML format to the writer.
    pub fn to_yaml_writer<W>(&self, writer: W) -> Result<()>
    where
        W: Write,
    {
        serde_yaml::to_writer(writer, self)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceFileHeader {
    pub version: u8,
    pub is_little_endian: bool,
}

/// Expected string in the header of all `.kfm` files.
const EXPECTED_MAGIC: &str = "Gamebryo KFM File Version 2.2.0.0b\n";

fn encode_source_file_header<W>(writer: &mut W, header: &SourceFileHeader) -> Result<()>
where
    W: Write,
{
    writer.write_u8(header.version).context("write `version`")?;
    writer.write_all(EXPECTED_MAGIC.as_bytes())?;
    writer.write_u8(1).context("write `is_little_endian`")?;

    Ok(())
}

fn decode_source_file_header<R>(reader: &mut R) -> Result<SourceFileHeader>
where
    R: Read,
{
    let version = reader.read_u8().context("read `version`")?;

    let magic = {
        let mut buf = vec![0u8; EXPECTED_MAGIC.len()];
        reader.read_exact(&mut buf)?;
        String::from_utf8(buf)?
    };
    if magic != EXPECTED_MAGIC {
        bail!("unexpected `magic`: `{}`", magic);
    }

    let is_little_endian = match reader.read_u8().context("read `is_little_endian`")? {
        0 => false,
        1 => true,
        v => bail!("unexpected `is_little_endian`: {}", v),
    };

    Ok(SourceFileHeader {
        version,
        is_little_endian,
    })
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceFileBody {
    pub model: Model,
    pub default_trans: DefaultTransitions,
    pub anims: Vec<Animation>,
    pub layer_groups: Vec<LayerGroup>,
}

impl Encode for SourceFileBody {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        writer
            .write_value::<_, O>(&self.model)
            .context("write `model`")?;

        writer
            .write_value::<_, O>(&self.default_trans)
            .context("write `default_trains`")?;

        writer
            .write_value::<_, O>(self.anims.as_slice())
            .context("write `anims`")?;

        writer
            .write_value::<_, O>(self.layer_groups.as_slice())
            .context("write `layer_groups`")?;

        Ok(())
    }
}

impl Decode for SourceFileBody {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let model = reader.read_value::<Model, O>().context("read `model`")?;

        let default_trans = reader
            .read_value::<DefaultTransitions, O>()
            .context("read `default_trans`")?;

        let anims = reader
            .read_value::<Vec<Animation>, O>()
            .context("read `anims`")?;

        let layer_groups = reader
            .read_value::<Vec<LayerGroup>, O>()
            .context("read `layer_groups`")?;

        Ok(Self {
            model,
            default_trans,
            anims,
            layer_groups,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Model {
    pub path: String,
    pub root: String,
}

impl Encode for Model {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        writer
            .write_value::<_, O>(&self.path)
            .context("write `path`")?;

        writer
            .write_value::<_, O>(&self.root)
            .context("write `root`")?;

        Ok(())
    }
}

impl Decode for Model {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let path = reader.read_value::<_, O>().context("read `path`")?;
        let root = reader.read_value::<_, O>().context("read `root`")?;

        Ok(Self { path, root })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DefaultTransitions {
    pub sync_type: TransitionType,
    pub sync_duration: f32,
    pub non_sync_type: TransitionType,
    pub non_sync_duration: f32,
}

impl Encode for DefaultTransitions {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        writer
            .write_value::<_, O>(&self.sync_type)
            .context("write `sync_type`")?;

        writer
            .write_value::<_, O>(&self.non_sync_type)
            .context("write `non_sync_type`")?;

        writer
            .write_f32::<O>(self.sync_duration)
            .context("write `sync_duration`")?;

        writer
            .write_f32::<O>(self.non_sync_duration)
            .context("write `non_sync_duration`")?;

        Ok(())
    }
}

impl Decode for DefaultTransitions {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let sync_type = reader.read_value::<_, O>().context("read `sync_type`")?;

        let non_sync_type = reader
            .read_value::<_, O>()
            .context("read `non_sync_type`")?;

        let sync_duration = reader.read_f32::<O>().context("read `sync_duration`")?;

        let non_sync_duration = reader.read_f32::<O>().context("read `non_sync_duration`")?;

        Ok(Self {
            sync_type,
            sync_duration,
            non_sync_type,
            non_sync_duration,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Animation {
    pub id: u32,
    pub path: String,
    pub index: u32,
    pub trans: Vec<Transition>,
}

impl Encode for Animation {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        writer.write_u32::<O>(self.id).context("write `id`")?;

        writer
            .write_value::<_, O>(&self.path)
            .context("write `path`")?;

        writer.write_u32::<O>(self.index).context("write `index`")?;

        writer
            .write_value::<_, O>(self.trans.as_slice())
            .context("write `trans`")?;

        Ok(())
    }
}

impl Decode for Animation {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let id = reader.read_u32::<O>().context("read `id`")?;
        let path = reader.read_value::<_, O>().context("read `path`")?;
        let index = reader.read_u32::<O>().context("read `index`")?;
        let trans = reader
            .read_value::<Vec<Transition>, O>()
            .context("read `trans`")?;

        Ok(Self {
            id,
            path,
            index,
            trans,
        })
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

impl Encode for Transition {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        writer.write_u32::<O>(self.id).context("write `id`")?;

        writer
            .write_value::<_, O>(&self.type_)
            .context("write `type`")?;

        if let Some(ext) = &self.ext {
            writer.write_value::<_, O>(ext).context("write `ext`")?;
        }

        Ok(())
    }
}

impl Decode for Transition {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let id = reader.read_u32::<O>().context("read `id`")?;

        let type_ = reader
            .read_value::<TransitionType, O>()
            .context("read `type`")?;

        let ext = match type_ {
            TransitionType::DefaultSync | TransitionType::DefaultNonSync => None,
            _ => Some(
                reader
                    .read_value::<TransitionExt, O>()
                    .context("read `ext`")?,
            ),
        };

        Ok(Self { id, type_, ext })
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum TransitionType {
    Blend,
    Morph,
    Crossfade,
    ChainAnimation,
    DefaultSync,
    DefaultNonSync,
}

impl Encode for TransitionType {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
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
            .write_u32::<O>(type_code)
            .context("write `type_code`")?;

        Ok(())
    }
}

impl Decode for TransitionType {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let type_code = reader.read_u32::<O>().context("read `type_code`")?;

        let type_ = match type_code {
            0 => Self::Blend,
            1 => Self::Morph,
            2 => Self::Crossfade,
            3 => Self::ChainAnimation,
            4 => Self::DefaultSync,
            5 => Self::DefaultNonSync,
            _ => bail!("unknown trans `type_code`: {}", type_code),
        };

        Ok(type_)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransitionExt {
    pub duration: f32,
    pub intermediate_anims: Vec<IntermediateAnimation>,
    pub chain_anims: Vec<ChainAnimation>,
}

impl Encode for TransitionExt {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        writer
            .write_f32::<O>(self.duration)
            .context("write `duration`")?;

        writer
            .write_value::<_, O>(self.intermediate_anims.as_slice())
            .context("write `intermediate_anims`")?;

        writer
            .write_value::<_, O>(self.chain_anims.as_slice())
            .context("write `chain_anims`")?;

        Ok(())
    }
}

impl Decode for TransitionExt {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let duration = reader.read_f32::<O>().context("read `duration`")?;

        let intermediate_anims = reader
            .read_value::<Vec<IntermediateAnimation>, O>()
            .context("read `intermediate_anims`")?;

        let chain_anims = reader
            .read_value::<Vec<ChainAnimation>, O>()
            .context("read `chain_anims`")?;

        Ok(Self {
            duration,
            intermediate_anims,
            chain_anims,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Layer {
    pub id: u32,
    pub priority: i32,
    pub weight: f32,
    pub ease_in_time: f32,
    pub ease_out_time: f32,
    pub sync_id: u32,
}

impl Encode for Layer {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        writer.write_u32::<O>(self.id).context("write `id`")?;

        writer
            .write_i32::<O>(self.priority)
            .context("write `priority`")?;

        writer
            .write_f32::<O>(self.weight)
            .context("write `weight`")?;

        writer
            .write_f32::<O>(self.ease_in_time)
            .context("write `ease_in_time`")?;

        writer
            .write_f32::<O>(self.ease_out_time)
            .context("write `ease_out_time`")?;

        writer
            .write_u32::<O>(self.sync_id)
            .context("write `sync_id`")?;

        Ok(())
    }
}

impl Decode for Layer {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let id = reader.read_u32::<O>().context("read `id`")?;
        let priority = reader.read_i32::<O>().context("read `priority`")?;
        let weight = reader.read_f32::<O>().context("read `weight`")?;
        let ease_in_time = reader.read_f32::<O>().context("read `ease_in_time`")?;
        let ease_out_time = reader.read_f32::<O>().context("read `ease_out_time`")?;
        let sync_id = reader.read_u32::<O>().context("read `sync_id`")?;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LayerGroup {
    pub id: u32,
    pub name: String,
    pub layers: Vec<Layer>,
}

impl Decode for LayerGroup {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let id = reader.read_u32::<O>().context("read `id`")?;

        let name = reader.read_value::<_, O>().context("read `name`")?;

        let layers = reader
            .read_value::<Vec<Layer>, O>()
            .context("read `layers`")?;

        Ok(Self { id, name, layers })
    }
}

impl Encode for LayerGroup {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        writer.write_u32::<O>(self.id).context("write `id`")?;

        writer
            .write_value::<_, O>(&self.name)
            .context("write `name`")?;

        writer
            .write_value::<_, O>(self.layers.as_slice())
            .context("write `layers`")?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IntermediateAnimation {
    pub start_key: String,
    pub target_key: String,
}

impl Encode for IntermediateAnimation {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        writer
            .write_value::<_, O>(&self.start_key)
            .context("write `start_key`")?;

        writer
            .write_value::<_, O>(&self.target_key)
            .context("write `target_key`")?;

        Ok(())
    }
}

impl Decode for IntermediateAnimation {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let start_key = reader.read_value::<_, O>().context("read `start_key`")?;
        let target_key = reader.read_value::<_, O>().context("read `target_key`")?;

        Ok(Self {
            start_key,
            target_key,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChainAnimation {
    pub id: u32,
    pub duration: f32,
}

impl Encode for ChainAnimation {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        writer.write_u32::<O>(self.id).context("write `id`")?;

        writer
            .write_f32::<O>(self.duration)
            .context("write `duration`")?;

        Ok(())
    }
}

impl Decode for ChainAnimation {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let id = reader.read_u32::<O>().context("read `id`")?;
        let duration = reader.read_f32::<O>().context("read `duration`")?;

        Ok(Self { id, duration })
    }
}

pub struct MappedSource {
    pub model: Model,
    pub default_trans: DefaultTransitions,
    pub anims: BTreeMap<u32, MappedAnimation>,
    pub layer_groups: Vec<LayerGroup>,
}

impl TryFrom<SourceFileBody> for MappedSource {
    type Error = Error;

    fn try_from(from: SourceFileBody) -> Result<Self> {
        let mut anims = BTreeMap::new();
        for anim in from.anims.into_iter() {
            let id = anim.id;
            let old = anims.insert(id, anim.try_into()?);
            if old.is_some() {
                bail!("duplicate anim id: `{}`", id);
            }
        }

        Ok(Self {
            model: from.model,
            default_trans: from.default_trans,
            anims,
            layer_groups: from.layer_groups,
        })
    }
}

impl From<MappedSource> for SourceFileBody {
    fn from(from: MappedSource) -> Self {
        let anims = from.anims.into_iter().map(Animation::from).collect();

        Self {
            model: from.model,
            default_trans: from.default_trans,
            anims,
            layer_groups: from.layer_groups,
        }
    }
}

pub struct MappedAnimation {
    pub path: String,
    pub index: u32,
    pub trans: BTreeMap<u32, MappedTransition>,
}

impl TryFrom<Animation> for MappedAnimation {
    type Error = Error;

    fn try_from(from: Animation) -> Result<Self> {
        let mut trans = BTreeMap::new();
        for tran in from.trans {
            let id = tran.id;
            let old = trans.insert(id, tran.into());
            if old.is_some() {
                bail!("duplicate tran id: `{}`", id);
            }
        }

        Ok(Self {
            path: from.path,
            index: from.index,
            trans,
        })
    }
}

impl TryFrom<Animation> for (u32, MappedAnimation) {
    type Error = Error;

    fn try_from(from: Animation) -> Result<Self> {
        Ok((from.id, from.try_into()?))
    }
}

impl From<(u32, MappedAnimation)> for Animation {
    fn from(from: (u32, MappedAnimation)) -> Self {
        let trans = from.1.trans.into_iter().map(Transition::from).collect();

        Self {
            id: from.0,
            path: from.1.path,
            index: from.1.index,
            trans,
        }
    }
}

pub struct MappedTransition {
    pub type_: TransitionType,
    pub ext: Option<TransitionExt>,
}

impl From<Transition> for MappedTransition {
    fn from(from: Transition) -> Self {
        Self {
            type_: from.type_,
            ext: from.ext,
        }
    }
}

impl From<Transition> for (u32, MappedTransition) {
    fn from(from: Transition) -> Self {
        (from.id, from.into())
    }
}

impl From<(u32, MappedTransition)> for Transition {
    fn from(from: (u32, MappedTransition)) -> Self {
        Self {
            id: from.0,
            type_: from.1.type_,
            ext: from.1.ext,
        }
    }
}
