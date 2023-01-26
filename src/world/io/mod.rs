pub mod region;

use crate::ioext::*;

use std::io::{
	self,
	Write,
	Read,
	// BufWriter,
	// BufReader,
};

use crate::nbt::{
	io::NbtWrite,
	io::NbtRead,
};

// impl<T: NbtWrite> Writable for T {
//     fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize,crate::McError> {
//         use crate::nbt::io::*;
// 		Ok(writer.write_nbt(self)?)
//     }
// }

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

// impl<T: NbtRead> Readable for T {
//     fn read_from<R: Read>(reader: &mut R) -> Result<Self,crate::McError> {
//         use crate::nbt::io::*;
// 		Ok(reader.read_nbt()?)
//     }
// }

pub trait WriteZeroes {
	fn write_zeroes(&mut self, count: u64) -> io::Result<u64>;
}

impl<T: Write> WriteZeroes for T {
    fn write_zeroes(&mut self, count: u64) -> io::Result<u64> {
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

/// Copies bytes from a reader into a writer
pub fn copy_bytes<R: Read, W: Write>(reader: &mut R, writer: &mut W, count: u64) -> io::Result<u64> {
	std::io::copy(&mut reader.take(count), writer)
}

