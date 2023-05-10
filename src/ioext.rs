use std::io::{
	Write, Read,
	Seek, SeekFrom,
};

use crate::{
	McResult,
};

pub const BUFFERSIZE: usize = 8192;

/// For types that can be written to a writer.
pub trait Writable {
	fn write_to<W: Write>(&self, writer: &mut W) -> McResult<usize>;
}

/// For types that can be read from a reader.
pub trait Readable: Sized {
	fn read_from<R: Read>(reader: &mut R) -> McResult<Self>;
}

/// For types that represent a seekable file offset.
pub trait Seekable: Sized {
	fn seek_to<S: Seek>(&self, seeker: &mut S) -> McResult<u64> {
		Ok(seeker.seek(self.seeker())?)
	}

	fn seeker(&self) -> SeekFrom;
}

pub trait WriteExt: Write + Sized {
	fn write_value<T: Writable>(&mut self, value: T) -> McResult<usize>;
}

pub trait ReadExt: Read + Sized {
	fn read_value<T: Readable>(&mut self) -> McResult<T>;
}

impl<W: Write + Sized> WriteExt for W {
    fn write_value<T: Writable>(&mut self, value: T) -> McResult<usize> {
		value.write_to(self)
	}
}

impl<R: Read + Sized> ReadExt for R {
    fn read_value<T: Readable>(&mut self) -> McResult<T> {
		T::read_from(self)
	}
}

pub trait SeekExt: Seek + Sized {
	fn seek_to<S: Seekable>(&mut self, seek_offset: &S) -> McResult<u64>;

	fn seek_return(&mut self) -> Result<SeekFrom,crate::McError>;
}

impl<T: Seek + Sized> SeekExt for T {
    fn seek_to<S: Seekable>(&mut self, seek_offset: &S) -> McResult<u64> {
		seek_offset.seek_to(self)
	}
	/// Returns a [SeekFrom] that  points to the current position in the stream.
    fn seek_return(&mut self) -> McResult<SeekFrom> {
		Ok(SeekFrom::Start(self.stream_position()?))
	}
}

/// Copies bytes from a reader into a writer
pub fn copy_bytes<R: Read, W: Write>(reader: &mut R, writer: &mut W, count: u64) -> std::io::Result<u64> {
	std::io::copy(&mut reader.take(count), writer)
}


pub trait WriteZeroes {
	fn write_zeroes(&mut self, count: u64) -> std::io::Result<u64>;
}

impl<T: Write> WriteZeroes for T {
    fn write_zeroes(&mut self, count: u64) -> std::io::Result<u64> {
		const ZEROES: &'static [u8; 4096] = &[0u8; 4096];
		let mut remainder = count;
		while remainder >= ZEROES.len() as u64 {
			self.write_all(ZEROES)?;
			remainder -= ZEROES.len() as u64;
		}
		if remainder != 0 {
			self.write_all(&ZEROES[0..remainder as usize])?;
		}
		Ok(count)
    }
}

/// A `Writable` struct that writes nothing to the writer.
/// This is useful when you need to provide a Writable type to a function
/// but do not want to write anything.
/// The specific purpose that this was created for was for deleting chunks
/// from a region file using its edit_chunks function.
pub struct WriteNothing;

impl Writable for WriteNothing {
    fn write_to<W: Write>(&self, _: &mut W) -> Result<usize,crate::McError> {
        Ok(0)
    }
}