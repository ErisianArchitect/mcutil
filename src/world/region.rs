// TODO: Remove this eventually.
#![allow(unused)]

use core::num;
use std::{
	path::{
		Path,
		PathBuf,
	},
	io::{
		Read,      Write,
		BufReader, BufWriter,
		Cursor, Seek, SeekFrom
	},
	fs::{
		File, copy,
	},
	ops::{BitOr}, env::current_exe, marker::PhantomData, ptr::read
};

use std::io::prelude::*;
use chumsky::primitive::todo;
use flate2::{read::GzDecoder, write::ZlibEncoder, Compression, Compress};
use flate2::read::ZlibDecoder;

use chrono::prelude::*;

use crate::{
	nbt::{
		tag::*,
		io::{
			ReadNbt,
			WriteNbt
		},
		io::{
			NbtRead,
			NbtWrite,
			NbtSize
		}
	},
	ioext::{*, self}, McError,
};

/*
Layout of region module:
	Functions:
		create_region_file<P: AsRef<Path>>(path: P) -> 
*/


/// 
#[derive(Clone, Copy,PartialEq, Eq, PartialOrd, Ord)]
pub struct RegionCoord(u16);

impl RegionCoord {
	pub fn new(x: i32, z: i32) -> Self {
		let xmod = (x & 31) as u16;
		let zmod = (z & 31) as u16;
		Self(xmod | zmod.overflowing_shl(5).0)
	}

	pub fn at_index(index: usize) -> Self {
		Self(index as u16)
	}

	pub fn index(&self) -> usize {
		self.0 as usize
	}

	pub fn x(&self) -> i32 {
		(self.0 & 31) as i32
	}

	pub fn z(&self) -> i32 {
		(self.0.overflowing_shr(5).0 & 31) as i32
	}

	pub fn tuple(&self) -> (i32, i32) {
		self.clone().into()
	}

}

impl From<(i32, i32)> for RegionCoord {
    fn from(value: (i32, i32)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<(usize, usize)> for RegionCoord {
    fn from(value: (usize, usize)) -> Self {
        Self::new(value.0 as i32, value.1 as i32)
    }
}

impl From<u64> for RegionCoord {
    fn from(value: u64) -> Self {
		Self(value as u16)
    }
}

impl From<i32> for RegionCoord {
    fn from(value: i32) -> Self {
        Self(value as u16)
    }
}

impl From<usize> for RegionCoord {
    fn from(value: usize) -> Self {
        Self(value as u16)
    }
}

impl From<RegionCoord> for (i32, i32) {
    fn from(value: RegionCoord) -> Self {
		let bits = value.0 as i32;
		(bits & 31, bits.overflowing_shr(5).0 & 31)
    }
}

/// Offset and size are packed together.
/// Having these two values packed together saves 4KiB per RegionFile.
/// It just seems a little wasteful to use more memory than is necessary.
/// |Offset:3|Size:1|
#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct ChunkOffset(u32);

impl ChunkOffset {
	pub fn new(offset: u32, size: u8) -> Self {
		Self(offset.overflowing_shl(8).0.bitor(size as u32))
	}

	pub fn offset(&self) -> u64 {
		self.sector_offset() * 4096
	}

	pub fn seeker(&self) -> SeekFrom {
		SeekFrom::Start(self.offset())
	}

	pub fn seek<R: Seek>(&self, reader: &mut R) -> std::io::Result<u64> {
		reader.seek(self.seeker())
	}

	pub fn size(&self) -> u64 {
		self.sector_count() * 4096
	}

	pub fn sector_offset(&self) -> u64 {
		self.0.overflowing_shr(8).0 as u64
	}

	pub fn sector_count(&self) -> u64 {
		(self.0 & 0xFF) as u64
	}

	pub fn empty(&self) -> bool {
		self.0 == 0
	}

	pub fn read<R: Read>(mut reader: R) -> std::io::Result<Self> {
		let mut buffer = [0u8; 4];
		reader.read_exact(&mut buffer[1..4])?;
		let offset = u32::from_be_bytes(buffer);
		reader.read_exact(&mut buffer[..1])?;
		Ok(
			ChunkOffset::new(offset, buffer[0])
		)
	}

}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Debug)]
pub struct Timestamp(pub u32);

impl TryFrom<Timestamp> for DateTime<Utc> {
    type Error = ();

    fn try_from(value: Timestamp) -> Result<Self, Self::Error> {
        let naive = NaiveDateTime::from_timestamp_opt(value.0 as i64, 0);
		if let Some(naive) = naive {
			Ok(DateTime::<Utc>::from_utc(naive, Utc))
		} else {
			Err(())
		}
    }
}

impl Timestamp {

	pub fn to_datetime(&self) -> Option<DateTime<Utc>> {
		if let Ok(result) = DateTime::<Utc>::try_from(*self) {
			Some(result)
		} else {
			None
		}
	}

	pub fn now() -> Timestamp {
		Timestamp(
			Utc::now().timestamp() as u32
		)
	}
	
	pub fn read<R: Read>(mut reader: R) -> std::io::Result<Self> {
		let mut buffer = [0u8; 4];
		reader.read_exact(&mut buffer)?;
		Ok(Timestamp(u32::from_be_bytes(buffer)))
	}

}

impl From<DateTime<Utc>> for Timestamp {
    fn from(value: DateTime<Utc>) -> Self {
        Timestamp(value.timestamp() as u32)
    }
}

// TODO: Add other info, such as metadata.
//		 Ideas:
//		 - File Creation Time
//		 - File Modification Time
//		 - File Size
pub struct RegionFileInfo {
	pub(crate) path: PathBuf,
	pub(crate) metadata: std::fs::Metadata,
	pub(crate) offsets: Vec<ChunkOffset>,
	pub(crate) timestamps: Vec<Timestamp>,
	pub(crate) present_bits: Box<[u64; 16]>,
}

const fn set_bit(mut value: u64, index: usize, on: bool) -> u64 {
	if on {
		value | (1 << index)
	} else {
		value & !(1 << index)
	}
}

const fn get_bit(value: u64, index: usize) -> bool {
	value & (1 << index) != 0
}

impl RegionFileInfo {

	pub fn load<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
		let file = File::open(path.as_ref())?;
		let metadata = std::fs::metadata(path.as_ref())?;
		let mut reader = BufReader::with_capacity(4096, file);
		// Read the Chunk Offsets (32x32)
		// 		The Chunk Offsets tell us the location within the file and size of chunks.
		let offsets: Vec<ChunkOffset> = (0..32*32).map(|_|
			ChunkOffset::read(&mut reader)
		).collect::<std::io::Result<Vec<ChunkOffset>>>()?;
		// Read the timestamps (32x32)
		let timestamps: Vec<Timestamp> = (0..32*32).map(|_|
			Timestamp::read(&mut reader)
		).collect::<std::io::Result<Vec<Timestamp>>>()?;
		let mut bits: Box<[u64; 16]> = Box::new([0; 16]);
		let mut buffer: [u8; 4] = [0; 4];
		let counter = 0;
		for i in 0..1024 {
			if !offsets[i].empty() {
				reader.seek(SeekFrom::Start(offsets[i].offset()))?;
				reader.read_exact(&mut buffer)?;
				let length = u32::from_be_bytes(buffer);
				if length != 0 {
					let bitword_index = i.div_euclid(64);
					bits[bitword_index] = set_bit(bits[bitword_index], i.rem_euclid(64), true);
				}
			}
		}
		Ok(Self {
			path: PathBuf::from(path.as_ref()),
			metadata,
			offsets,
			timestamps,
			present_bits: bits,
		})
	}

	pub fn open(&self) -> std::io::Result<File> {
		File::open(&self.path)
	}

	pub fn path(&self) -> &Path {
		&self.path
	}

	pub fn metadata(&self) -> std::fs::Metadata {
		self.metadata.clone()
	}

	pub fn get_offset(&self, x: i32, z: i32) -> ChunkOffset {
		self.offsets[RegionFile::get_index(x,z)]
	}

	pub fn get_timestamp(&self, x: i32, z: i32) -> Timestamp {
		self.timestamps[RegionFile::get_index(x,z)]
	}

	pub fn has_chunk(&self, x: i32, z: i32) -> bool {
		let index = RegionFile::get_index(x, z);
		let bitword_index = index.div_euclid(64);
		let bit_index = index.rem_euclid(64);
		get_bit(self.present_bits[bitword_index], bit_index)
	}

	pub fn creation_time(&self) -> std::io::Result<std::time::SystemTime> {
		self.metadata.created()
	}

	pub fn modified_time(&self) -> std::io::Result<std::time::SystemTime> {
		self.metadata.modified()
	}

	pub fn accessed_time(&self) -> std::io::Result<std::time::SystemTime> {
		self.metadata.accessed()
	}

	/// Returns the size of the region file.
	pub fn size(&self) -> u64 {
		self.metadata.len()
	}

}

/// Handles abstractions for dealing with Region Files.
pub struct RegionFile {
	// 8KiB header
	// The first 4KiB containing offsets.
	// The second 4KiB containing timestamps.
	// pub offsets: Box<[ChunkLocation]>,
	// pub timestamps: Box<[Timestamp]>,
	// The path to the RegionFile is important because it's how reading happens.
	path: PathBuf,
}

impl RegionFile {
	/*
	Some notes about the functions in this implementation:
		Coordinates (x and z) are not relative.
		So if you want the chunk at (0,0) in the region, you could use (0,0), or (32, 32), or (0, 32), etc.
		The formula is (x % 32, z % 32).

	*/

	/// This will create an empty region file at `path`.
	/// An empty region file is simply 8KiB of zeroes, which is
	/// a 1024 entry offset table and a 1024 entry timestamp table.
	/// Each offset is a 24-bit sector offset as well as an 8-bit size
	pub fn create<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
		let mut file = File::create(path.as_ref())?;
		let mut writer = BufWriter::with_capacity(4096, file);
		_write_blank_region_header(&mut writer)?;
		Ok(Self::at_path(path))
	}

	/// 
	//pub fn edit_chunks<'a,T: Writable + 'a,  It: IntoIterator<Item = ((i32, i32), Option<&'a T>)>>(&self, it: It, compression: Compression) -> Result<(), RegionError> {
	pub fn create_with<'a, P, T, It>(path: P, compression: Compression, it: It) -> Result<Self,crate::McError>
	where
		P: AsRef<Path>,
		T: Writable + 'a,
		It: IntoIterator<Item = ((i32, i32), &'a T)> {
			let region = Self::create(path)?;
			region.edit_chunks(
				compression,
				it.into_iter().map(_map_key_value_pair_to_some),
			)?;
			Ok(region)
	}

	/// This function gets the flat index (in an array of 1024 elements) from a 32x32 grid.
	/// This is a helper function to help find the offset in a region file where certain information is stored.
	pub(crate) const fn get_index(x: i32, z: i32) -> usize {
		(x.rem_euclid(32) + (z.rem_euclid(32) * 32)) as usize
	}

	/// Creates a RegionFile object with the specified path.
	pub fn at_path<P: AsRef<Path>>(path: P) -> Self {
		Self {
			path: PathBuf::from(path.as_ref()),
		}
	}

	pub fn info(&self) -> std::io::Result<RegionFileInfo> {
		RegionFileInfo::load(&self.path)
	}

	pub(crate) fn read_chunk_offset<R: Read + Seek>(reader: &mut R, x: i32, z: i32) -> Result<ChunkOffset,McError> {
		let offset = RegionFile::get_index(x, z) * 4;
		reader.seek(SeekFrom::Start(offset as u64))?;
		Ok(ChunkOffset::read(reader)?)
	}

	pub(crate) fn read_timestamp<R: Read + Seek>(reader: &mut R, x: i32, z: i32) -> Result<Timestamp,McError> {
		let offset = RegionFile::get_index(x, z) * 4;
		let mut buffer = [0u8; 4];
		reader.seek(SeekFrom::Start(offset as u64 + 4096))?;
		reader.read_exact(&mut buffer)?;
		Ok(Timestamp(u32::from_be_bytes(buffer)))
	}

	pub fn path(&self) -> &Path {
		&self.path
	}

	/// Open the region file.
	pub fn open(&self) -> std::io::Result<File> {
		File::open(&self.path)
	}

	pub fn get_timestamp(&self, x: i32, z: i32) -> Result<Timestamp, McError> {
		let mut file = self.open()?;
		let mut reader = BufReader::with_capacity(4096, file);
		RegionFile::read_timestamp(&mut reader, x, z)
	}

	/// This function will open the region file, find the chunk offset, then seek to that offset.
	pub fn chunk_present(&self, x: i32, z: i32) -> Result<bool, McError> {
		let mut file = self.open()?;
		let mut reader = BufReader::with_capacity(4096, file);
		let offset = RegionFile::read_chunk_offset(&mut reader, x, z)?;
		if offset.empty() {
			return Ok(false)
		}
		reader.seek(SeekFrom::Start(offset.offset()))?;
		// 4 byte buffer for reading the length of the chunk.
		// If the length of the chunk is 0, there is no chunk present.
		let mut buffer = [0u8; 4];
		reader.read_exact(&mut buffer)?;
		let length = (u32::from_be_bytes(buffer) as u64);
		dbg!("Offset not empty, {}", length);
		Ok(length > 0)
	}

	pub fn delete_chunk(&self, x: i32, z: i32) -> Result<(), crate::McError> {
		self.edit_chunk::<WriteNothing>(x, z, None, Compression::none())
	}

	pub fn set_chunk<T: Writable>(&self, x: i32, z: i32, chunk: &T, compression: Compression) -> Result<(), crate::McError> {
		self.edit_chunk(x, z, Some(chunk), compression)
	}

	/// Either replace or delete a chunk in a region file.
	pub(crate) fn edit_chunk<T: Writable>(&self, x: i32, z: i32, chunk: Option<&T>, compression: Compression) -> Result<(), crate::McError> {
		/*
		The process for injecting a chunk is fairly difficult.
		It involves rewriting the file completely. So a temporary file will need to be created.
		Then each chunk present in the file is systematically copied over to the new file and
		a new timestamp and chunk offset table is built.
		*/
		self.edit_chunks(compression, [((x, z), chunk)])
	}

	/// Rebuilds a region files.
	/// If a None value is provided, effectively deletes the chunk.
	/// Otherwise, if a Some value is provided, that value is written
	/// to the file.
	/// The coordinate for each provided chunk will be normalized to the 32x32 range.
	/// That means that if it is outside of the range of 0-31, It will be forced into
	/// that range. Does that make sense? No? Maybe this will:
	/// ```rs
	/// value = value.rem_euclid(32);
	/// ```
	pub fn edit_chunks<'a,T,  It>(&self, compression: Compression, it: It) -> Result<(), crate::McError>
	where 
	T: Writable + 'a,
	It: IntoIterator<Item = ((i32, i32), Option<&'a T>)> {
		let map = {
			let mut map: Vec<Option<Option<&'a T>>> = vec![None; 1024];
			it.into_iter().try_for_each(|((x, z), tag)| {
				let index = RegionFile::get_index(x, z);
				// Check if the chunk has already been assigned. This means that we are trying to save
				// two chunks to the same location, which is an error.
				if std::mem::replace(&mut map[index], Some(tag)).is_none() {
					Ok(())
				} else {
					Err(McError::DuplicateChunk)
				}
			})?;
			map
		};

		let info = self.info()?;
		// We open the region file that we plan to inject into.
		let input: File = self.open()?;
		// Open a temporary file to inject into.
		let outputfile = tempfile::NamedTempFile::new()?;
		let mut writer = BufWriter::with_capacity(4096, outputfile);
		let mut reader = BufReader::with_capacity(4096, input);
		
		_write_blank_region_header(&mut writer)?;
		let edit_time = Timestamp::now();

		let mut current_sector = 2;
		for i in 0..1024 {
			match map[i] {
				Some(Some(chunk)) => {
					current_sector = _write_chunk_data(
						current_sector,
						&mut writer,
						i,
						chunk,
						edit_time,
						compression,
					)?;
				},
				None => {
					current_sector = _copy_chunk_data(
						current_sector,
						&mut reader,
						&mut writer,
						i,
						info.offsets[i],
						info.timestamps[i],
					)?;
				},
				_ => (),
			}
		}
		let output = writer.get_ref().path();
		std::fs::copy(output, self.path())?;
		Ok(())
	}

	/// Counts the number of 4KB sectors required to accomodate `size` bytes.
	fn required_sectors(size: u32) -> u32 {
		if size == 0 {
			return 0;
		}
		if size < 4096 {
			return 1;
		}
		let sub = size / 4096;
		let overflow = if size.rem_euclid(4096) != 0 { 1 } else { 0 };
		sub + overflow
	}

}



pub trait ChunkManager<T> {
	fn get(&self, x: i32, z: i32) -> Option<&T>;
	fn get_mut(&mut self, x: i32, z: i32) -> Option<&mut T>;
	fn set(&mut self, x: i32, z: i32, value: T) -> Option<T>;
}

pub trait ChunkProvider {

	type Error;

	fn get_chunk_nbt(&self, x: i32, z: i32) -> Result<NamedTag, Self::Error>;

}

impl ChunkProvider for RegionFile {
	type Error = McError;

	fn get_chunk_nbt(&self, x: i32, z: i32) -> Result<NamedTag, Self::Error> {
        let mut file = self.open()?;
		let mut reader = BufReader::with_capacity(4 << 10, file);
		let offset = RegionFile::read_chunk_offset(&mut reader, x, z)?;
		if offset.empty() {
			return Err(McError::ChunkNotFound);
		}

		offset.seek(&mut reader)?;
		_read_from_region_sectors(&mut reader)
    }

}

/// This function will read a value from a reader that is an open region
/// file. The reader is expected to be at the beginning of a 4KiB sector
/// within the file. This function does not perform that check. It will
/// read a 32-bit length, an 8-bit compression scheme (1, 2, or 3), then
/// if will create the appropriate decompressor (if applicable) to read
/// the value from.
fn _read_from_region_sectors<R: Read,T: Readable>(reader: &mut R) -> Result<T,McError> {
	let mut buffer = [0u8; 4];
	// Read the length of the chunk.
	reader.read_exact(&mut buffer)?;
	// 1 is subtracted from the lenth that is read because there is 1 byte for the compression scheme, and the rest of the length is the data.
	let length = u32::from_be_bytes(buffer) as u64;
	if length == 0 {
		return Err(McError::ChunkNotFound);
	}
	// Read compression scheme
	reader.read_exact(&mut buffer[..1])?;
	let compression_scheme = buffer[0];
	match compression_scheme {
		// GZip
		1 => {
			let mut dec = GzDecoder::new(reader.take(length - 1));
			T::read_from(&mut dec)
		}
		// ZLib
		2 => {
			let mut dec = ZlibDecoder::new(reader.take(length - 1));
			T::read_from(&mut dec)
		}
		// Uncompressed (since a version before 1.15.1)
		3 => {
			T::read_from(&mut reader.take(length - 1))
		}
		_ => return Err(McError::InvalidCompressionScheme(compression_scheme)),
	}
}

/// Writes a chunk to a writer, including pad bytes.
/// This function assumes that the writer's position is on a 4KiB boundary.
/// The return value is the number of 4KiB sectors that were written.
fn _write_chunk_to_region<W: Write + Seek, T: Writable>(writer: &mut W, chunk: &T, compression: Compression) -> Result<u32,crate::McError> {
	let mut chunk_buffer = Vec::new();
	let mut compressor = ZlibEncoder::new(chunk_buffer, compression);

	chunk.write_to(&mut compressor)?;

	chunk_buffer = compressor.finish()?;

	let length = chunk_buffer.len() as u32 + 1;
	let mut length_buffer = length.to_be_bytes();
	
	writer.write(&length_buffer)?;
	length_buffer[0] = 2;
	writer.write(&length_buffer[..1])?;
	writer.write(&chunk_buffer)?;

	let required_sectors = RegionFile::required_sectors(length + 4);
	let padsize = (required_sectors * 4096) - (length + 4);
	write_zeroes(writer, padsize as u64)?;
	Ok(required_sectors)
}

/// Writes a chunk offset to the chunk offset table in a region file.
/// (Note: this function will return the writer back to the position it
/// started at.)
/// In a region file, there is a 4KiB sector in the header of the file
/// that holds 1024 "chunk offsets". These offsets are two values,
/// 4 bytes total. a 24-bit offset value, and an 8-bit size value.
fn _write_offset_to_table<W: Write + Seek>(writer: &mut W, offset: ChunkOffset, index: usize) -> Result<(),McError> {
	if index >= 1024 {
		return Err(McError::OutOfRange);
	}
	// Store the position that the writer starts at so that we can
	// return to that position before returning.
	let return_offset = writer.stream_position()?;
	// The location in the file where the offset is written to is
	// index * 4
	writer.seek(SeekFrom::Start((index * 4) as u64))?;
	// Big-endian bytes, which is Minecraft's style.
	let buffer = offset.0.to_be_bytes();
	writer.write_all(&buffer)?;
	// Make sure to return to where we started.
	writer.seek(SeekFrom::Start(return_offset))?;
	Ok(())
}

/// Writes a timestamp to the chunk timestamp table in a region file.
/// (Note: this function will return the writer back to the position it
/// started at.)
fn _write_timestamp_to_table<W: Write + Seek>(writer: &mut W, timestamp: Timestamp, index: usize) -> Result<(),McError> {
	if index >= 1024 {
		return Err(McError::OutOfRange);
	}
	// Store the position that the writer starts at so that we can
	// return to that position before returning.
	let return_offset = writer.stream_position()?;
	// The location in the file where the offset is written to is (parenteses added for readability)
	// (index * 4) + 4096
	writer.seek(SeekFrom::Start((index * 4 + 4096) as u64))?;
	// Big-endian bytes, which is Minecraft's style.
	let buffer = timestamp.0.to_be_bytes();
	writer.write_all(&buffer)?;
	// Make sure to return to where we started.
	writer.seek(SeekFrom::Start(return_offset))?;
	Ok(())
}

/// Every header file has an 8KiB (8192) byte header that contains:
///	1024 Offset values (32-bits)
/// 1024 Timestamp values (32-bits)
/// A blank header would just be 8192 zeroes at the beginning of a
/// region file. This function writes that blank header to a writer.
/// The writer is expected to already be at the beginning of the file.
/// This file is simply a shortcut to write 8KiB of zeroes.
fn _write_blank_region_header<W: Write>(writer: &mut W) -> std::io::Result<u64> {
	write_zeroes(writer, 4096*2)
}

/// Shortcut for writing a chunk to a writer.
/// Sector offset is the current sector offset in the writer. The return
/// value is the sector_offset + the number of sectors written.
/// This function will also write the new offset to the offset table
/// of the chunk, then it will write the timestamp to the timestamp
/// table.
fn _write_chunk_data<W: Write + Seek, T: Writable>(
	sector_offset: u32,
	writer: &mut W,
	index: usize,
	chunk: &T,
	timestamp: Timestamp,
	compression: Compression,
) -> Result<u32,crate::McError> {
	let required_sectors = _write_chunk_to_region(writer, chunk, compression)?;
	let newoffset = ChunkOffset::new(sector_offset, required_sectors as u8);
	_write_offset_to_table(writer, newoffset, index)?;
	_write_timestamp_to_table(writer, timestamp, index)?;
	Ok(sector_offset + required_sectors)
}

/// Copies chunk data from one region file to another.
/// Also writes provided timestamp and offset to region file header.
/// Returns the number of sectors copied.
fn _copy_chunk_data<W: Write + Seek, R: Read + Seek>(
	sector_offset: u32,
	reader: &mut R,
	writer: &mut W,
	index: usize,
	offset: ChunkOffset,
	timestamp: Timestamp,
) -> Result<u32,McError> {

	if offset.empty() {
		return Ok(sector_offset)
	}

	let sector_count = offset.sector_count();
	reader.seek(SeekFrom::Start(offset.offset()))?;

	copy_bytes(reader, writer, sector_count * 4096)?;

	_write_timestamp_to_table(writer, timestamp, index)?;
	_write_offset_to_table(writer, ChunkOffset::new(sector_offset, sector_count as u8), index)?;

	Ok(sector_offset + sector_count as u32)
}

/// Maps the T value to a Some(T).
fn _map_key_value_pair_to_some<T>(item: ((i32, i32), T)) -> ((i32, i32), Option<T>) {
	(item.0, Some(item.1))
}