use anyhow::{bail, Context, Result};
use byteorder::{ByteOrder, WriteBytesExt, ReadBytesExt};
use std::io::{Read, Write};

/// Defines how an object can be encoded into KFM format.
pub trait Encode {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder;
}

impl<T> Encode for [T]
where
    T: Encode,
{
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        let num_items = self.len() as u32;
        writer
            .write_u32::<O>(num_items)
            .context("write length specifier")?;

        for (i, item) in self.iter().enumerate() {
            writer
                .write_value::<T, O>(item)
                .with_context(|| format!("write item at index {}", i))?;
        }

        Ok(())
    }
}

impl Encode for String {
    fn encode_kfm<W, O>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        if !self.is_ascii() {
            bail!("`{}` is not ascii", self);
        }

        writer.write_u32::<O>(self.len() as u32)?;
        writer.write_all(self.as_bytes())?;

        Ok(())
    }
}

/// Provides functionality for writing KFM-encodable data.
pub trait WriteValueExt: Write {
    fn write_value<T, O>(&mut self, data: &T) -> Result<()>
    where
        T: Encode + ?Sized,
        O: ByteOrder;
}

impl<W> WriteValueExt for W
where
    W: Write,
{
    fn write_value<T, O>(&mut self, data: &T) -> Result<()>
    where
        T: Encode + ?Sized,
        O: ByteOrder,
    {
        data.encode_kfm::<_, O>(self)
    }
}


/// Defines how an object can be decoded from KFM format.
pub trait Decode: Sized {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder;
}

impl<T> Decode for Vec<T>
where
    T: Decode,
{
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
    where
        R: Read,
        O: ByteOrder,
    {
        let num_items = reader.read_u32::<O>().context("read length specifier")? as usize;

        let mut items = Vec::with_capacity(num_items);
        for i in 0..num_items {
            let item = reader
                .read_value::<T, O>()
                .with_context(|| format!("read item at index {}", i))?;
            items.push(item);
        }

        Ok(items)
    }
}

impl Decode for String {
    fn decode_kfm<R, O>(reader: &mut R) -> Result<Self>
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
}

/// Provides functionality for reading KFM-decodable data.
pub trait ReadValueExt: Read + Sized {
    fn read_value<T, O>(&mut self) -> Result<T>
    where
        T: Decode,
        O: ByteOrder;
}

impl<R> ReadValueExt for R
where
    R: Read,
{
    fn read_value<T, O>(&mut self) -> Result<T>
    where
        T: Decode,
        O: ByteOrder,
    {
        T::decode_kfm::<_, O>(self)
    }
}
