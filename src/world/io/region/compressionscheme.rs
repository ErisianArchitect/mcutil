use std::io::{Read, Write};
use crate::{
    McResult, McError,
    ioext::*,
};

/// Compression scheme used for writing or reading.
#[repr(u8)]
pub enum CompressionScheme {
    /// GZip compression is used.
    GZip = 1,
    /// ZLib compression is used.
    ZLib = 2,
    /// Data is uncompressed.
    Uncompressed = 3,
}

impl Writable for CompressionScheme {
    fn write_to<W: Write>(&self, writer: &mut W) -> McResult<usize> {
        match self {
            CompressionScheme::GZip => writer.write_value(1u8),
            CompressionScheme::ZLib => writer.write_value(2u8),
            CompressionScheme::Uncompressed => writer.write_value(3u8),
        }
    }
}

impl Readable for CompressionScheme {
    fn read_from<R: Read>(reader: &mut R) -> McResult<Self> {
        match reader.read_value::<u8>()? {
            1 => Ok(Self::GZip),
            2 => Ok(Self::ZLib),
            3 => Ok(Self::Uncompressed),
            unexpected => Err(McError::InvalidCompressionScheme(unexpected)),
        }
    }
}