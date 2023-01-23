use std::io::{
	self,
	Write,
	Read,
	// BufWriter,
	// BufReader,
};

use crate::nbt::{
	NbtError,
	io::NbtWrite,
};

pub trait Writable {
	fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize,NbtError>;
}

impl<T: NbtWrite> Writable for T {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize,NbtError> {
        use crate::nbt::io::*;
		writer.write_nbt(self)
    }
}

/// Writes zeroes to a writer.
pub fn write_zeroes<W: Write>(writer: &mut W, count: u64) -> io::Result<u64> {
	const ZEROES: &'static [u8; 4096] = &[0u8; 4096];
	let mut remainder = count;
	while remainder >= ZEROES.len() as u64 {
		writer.write_all(ZEROES)?;
		remainder -= ZEROES.len() as u64;
	}
	if remainder != 0 {
		writer.write_all(&ZEROES[0..remainder as usize])?;
	}
	Ok(count)
}

/// Copies bytes from a reader into a writer
pub fn copy_bytes<R: Read, W: Write>(reader: &mut R, writer: &mut W, count: u64) -> io::Result<u64> {
	let buffer_size = _highest_power_of_two(count).min(4096);
	let mut buffer = vec![0u8; buffer_size as usize];
	let mut remainder = count;
	while remainder >= buffer_size {
		reader.read_exact(&mut buffer)?;
		writer.write_all(&buffer)?;
		remainder = remainder - buffer_size;
	}
	if remainder != 0 {
		reader.read_exact(&mut buffer[0..remainder as usize])?;
		writer.write_all(&buffer[0..remainder as usize])?;
	}
	Ok(count)
}

fn _highest_power_of_two(value: u64) -> u64 {
	if value == 0 {
		return 0;
	}
	let mut highest = 1u64 << 63;
	while value < highest && highest > 0 {
		highest = highest.overflowing_shr(1).0;
	}
	highest
}

pub struct WriteNothing;

impl Writable for WriteNothing {
    fn write_to<W: Write>(&self, _: &mut W) -> Result<usize,NbtError> {
        Ok(0)
    }
}