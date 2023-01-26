//! Module for creating, reading, and modifying Minecraft region files.

#![allow(unused)]

use std::{
	io::{
		Read, Write,
		BufReader, BufWriter,
		Seek, SeekFrom,
	},
	fs::{
		File,
	},
	path::{
		Path, PathBuf,
	},
	ops::*,
};

use chrono::prelude::*;
use flate2::{
	read::GzDecoder,
	read::ZlibDecoder,
	write::ZlibEncoder,
	Compression,
	Compress
};

use crate::{*, world::chunk};
use crate::{ioext::*, math::bit::SetBit};
use crate::world::io::*;
use crate::for_each_int_type;

/* Map of file:
	Structs:
		RegionBuilder
		RegionCoord
		RegionSector
		Timestamp
		RegionFileInfo
	Implementations
	Public functions
	Private functions
*/

/// A region file contains up to 1024 chunks, which is 32x32 chunks.
/// This struct represents a chuunk coordinate within a region file.
/// The coordinate can be an absolute coordinate and it will be
/// normalized to relative coordinates.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegionCoord(u16);

/// Offset and size are packed together.
/// Having these two values packed together saves 4KiB per RegionFile.
/// It just seems a little wasteful to use more memory than is necessary.
/// |Offset:3|Size:1|
#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct RegionSector(u32);

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Debug)]
pub struct Timestamp(pub u32);

// TODO: Add other info, such as metadata.
//		 Ideas:
//		 - File Creation Time
//		 - File Modification Time
//		 - File Size
pub struct RegionFileInfo {
	pub(crate) path: PathBuf,
	pub(crate) metadata: std::fs::Metadata,
	pub(crate) offsets: Vec<RegionSector>,
	pub(crate) timestamps: Vec<Timestamp>,
	pub(crate) present_bits: Box<[u64; 16]>,
}

/// An abstraction for reading Region files.
/// You open a region file, pass the reader over to this
/// struct, then you read the offsets/timestamps/chunks
/// that you need. When you're done reading, you can
/// call `.finish()` to take the reader back.
pub struct RegionReader<R: Read + Seek> {
	reader: R,
}

/// An abstraction for writing Region files.
/// You open a region file, pass the writer over to this
/// struct, then you write whatever offsets/timestamps/chunks
/// that you need to write. When you're done writing, you can
/// call `.finish()` to take the writer back.
pub struct RegionWriter<W: Write + Seek> {
	writer: W,
}

impl RegionCoord {
	pub fn new(x: u16, z: u16) -> Self {
		let xmod = (x & 31);
		let zmod = (z & 31);
		Self(xmod | zmod.overflowing_shl(5).0)
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

	pub fn tuple<T>(self) -> (T, T)
	where
	(T, T): From<Self> {
		self.into()
	}

	fn _seek_to_offset_table<S: Seek>(&self, seekable: &mut S) -> std::io::Result<u64> {
		seekable.seek(SeekFrom::Start((self.0 * 4) as u64))
	}

	fn _seek_to_timestamp_table<S: Seek>(&self, seekable: &mut S) -> std::io::Result<u64> {
		seekable.seek(SeekFrom::Start((self.0 * 4 + 4096) as u64))
	}
}

macro_rules! __regioncoord_impl {
	($type:ty) => {
		impl From<($type, $type)> for RegionCoord {
			fn from(value: ($type, $type)) -> Self {
				Self::new(value.0 as u16, value.1 as u16)
			}
		}

		impl From<$type> for RegionCoord {
			fn from(value: $type) -> Self {
				Self(value as u16)
			}
		}

		impl From<RegionCoord> for ($type, $type) {
			fn from(value: RegionCoord) -> Self {
				(value.x() as $type, value.z() as $type)
			}
		}

		impl From<RegionCoord> for $type {
			fn from(value: RegionCoord) -> Self {
				value.0 as $type
			}
		}
	};
}

for_each_int_type!(__regioncoord_impl);

impl RegionSector {
	pub fn new(offset: u32, size: u8) -> Self {
		Self(offset.overflowing_shl(8).0.bitor(size as u32))
	}

	/// The raw 4KiB sector offset.
	/// Multiply this by `4096` to get the seek offset.
	pub fn sector_offset(self) -> u64 {
		self.0.overflowing_shr(8).0 as u64
	}

	pub fn sector_end_offset(self) -> u64 {
		self.sector_offset() + self.sector_count()
	}

	/// The raw 4KiB sector count.
	/// Multiply this by `4096` to get the sector size.
	pub fn sector_count(self) -> u64 {
		(self.0 & 0xFF) as u64
	}

	/// The offset in bytes that this sector begins
	/// at in the region file.
	pub fn offset(self) -> u64 {
		self.sector_offset() * 4096
	}

	pub fn end_offset(self) -> u64 {
		self.sector_end_offset() * 4096
	}

	/// The size in bytes that this sector occupies.
	pub fn size(self) -> u64 {
		self.sector_count() * 4096
	}

	/// Determines if this is an "empty" sector.
	pub fn is_empty(self) -> bool {
		self.0 == 0
	}
}

macro_rules! __regionsector_impls {
	($type:ty) => {
		impl From<Range<$type>> for RegionSector {
			fn from(value: Range<$type>) -> Self {
				RegionSector::new(value.start as u32, (value.end - value.start) as u8)
			}
		}
	};
}

for_each_int_type!(__regionsector_impls);

impl BitAnd for RegionSector {
    type Output = bool;

	/// Checks if two sectors intersect. If they intersect, this will
	/// return true.
	fn bitand(self, rhs: Self) -> Self::Output {
		if self.sector_offset() == rhs.sector_offset() {
			return true;
		}
		!(self.sector_end_offset() <= rhs.sector_offset()
		|| rhs.sector_end_offset() <= self.sector_offset())
	}
}

impl Readable for RegionSector {
	fn read_from<R: Read>(reader: &mut R) -> Result<Self,crate::McError> {
		Ok(Self(u32::nbt_read(reader)?))
	}
}

impl Writable for RegionSector {
	fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize,crate::McError> {
		Ok(self.0.nbt_write(writer)?)
	}
}

impl Seekable for RegionSector {
    fn seeker(&self) -> SeekFrom {
        SeekFrom::Start(self.offset())
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
}

impl Readable for Timestamp {
	fn read_from<R: Read>(reader: &mut R) -> Result<Self,crate::McError> {
		Ok(Self(u32::nbt_read(reader)?))
	}
}

impl Writable for Timestamp {
	fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize,crate::McError> {
		Ok(self.0.nbt_write(writer)?)
	}
}

impl From<DateTime<Utc>> for Timestamp {
	fn from(value: DateTime<Utc>) -> Self {
		Timestamp(value.timestamp() as u32)
	}
}

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

impl RegionFileInfo {

	pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, crate::McError> {
		let file = File::open(path.as_ref())?;
		let metadata = std::fs::metadata(path.as_ref())?;
		let mut reader = BufReader::with_capacity(4096, file);
		// Read the Chunk Offsets (32x32)
		// 		The Chunk Offsets tell us the location within the file and size of chunks.
		let offsets: Vec<RegionSector> = (0..32*32).map(|_|
			RegionSector::read_from(&mut reader)
		).collect::<Result<Vec<RegionSector>,crate::McError>>()?;
		// Read the timestamps (32x32)
		let timestamps: Vec<Timestamp> = (0..32*32).map(|_|
			Timestamp::read_from(&mut reader)
		).collect::<Result<Vec<Timestamp>,crate::McError>>()?;
		let mut bits: Box<[u64; 16]> = Box::new([0; 16]);
		let mut buffer: [u8; 4] = [0; 4];
		let counter = 0;
		for i in 0..1024 {
			if !offsets[i].is_empty() {
				reader.seek(SeekFrom::Start(offsets[i].offset()))?;
				reader.read_exact(&mut buffer)?;
				let length = u32::from_be_bytes(buffer);
				if length != 0 {
					let bitword_index = i.div_euclid(64);
					bits[bitword_index] = bits[bitword_index].set_bit(i.rem_euclid(64), true);
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

	pub fn open(&self) -> Result<File,crate::McError> {
		Ok(File::open(&self.path)?)
	}

	pub fn path(&self) -> &Path {
		&self.path
	}

	pub fn metadata(&self) -> std::fs::Metadata {
		self.metadata.clone()
	}

	pub fn get_offset(&self, x: i32, z: i32) -> RegionSector {
		self.offsets[RegionCoord::from((x, z)).index()]
	}

	pub fn get_timestamp(&self, x: i32, z: i32) -> Timestamp {
		self.timestamps[RegionCoord::from((x, z)).index()]
	}

	pub fn has_chunk(&self, x: i32, z: i32) -> bool {
		use crate::math::bit::GetBit;
		let index = RegionCoord::from((x, z)).index();
		let bitword_index = index.div_euclid(64);
		let bit_index = index.rem_euclid(64);
		self.present_bits[bitword_index].get_bit(bit_index)
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

impl<R: Read + Seek + SeekExt> RegionReader<R> {
	pub fn new(reader: R) -> Self {
		Self {
			reader,
		}
	}

	/// Read a sector offset from the sector offset table in the region file
	/// header.
	pub fn read_offset<C: Into<RegionCoord>>(&mut self, coord: C) -> Result<RegionSector, crate::McError> {
		let coord: RegionCoord = coord.into();
		coord._seek_to_offset_table(&mut self.reader)?;
		let sector = RegionSector::read_from(&mut self.reader)?;
		Ok(sector)
	}

	/// Read a timestamp from the timestamp table in the region file
	/// header.
	pub fn read_timestamp<C: Into<RegionCoord>>(&mut self, coord: C) -> Result<Timestamp,crate::McError> {
		let coord: RegionCoord = coord.into();
		coord._seek_to_timestamp_table(&mut self.reader)?;
		let timestamp = Timestamp::read_from(&mut self.reader)?;
		Ok(timestamp)
	}

	/// Read a chunk from the region file.
	pub fn read_chunk<T: Readable, C: Into<RegionCoord>>(&mut self, coord: C) -> Result<Option<T>,crate::McError> {
		let offset = self.read_offset(coord)?;
		if offset.is_empty() {
			return Ok(None);
		}
		self.reader.seek(offset.seeker())?;
		Ok(Some(
			_read_from_region_sectors(&mut self.reader)?
		))
	}

	/// Finish reading and return the contained reader.
	pub fn finish(self) -> R {
		self.reader
	}
}

impl<W: Write + Seek> RegionWriter<W> {
	pub fn new(writer: W) -> Self {
		Self {
			writer,
		}
	}

	/// Returns the 4KiB offset of the current sector.
	pub fn sector_offset(&mut self) -> Result<u32,McError> {
		Ok(self.writer.stream_position()? as u32)
	}

	/// This function writes an 8KiB zeroed header to the writer.
	/// In order to reduce system calls and whatever, this function
	/// assumes that you are already at the start of the file.
	/// This is a function that you would call while building a new
	/// region file.
	pub fn write_empty_header(&mut self) -> Result<u64,McError> {
		Ok(self.writer.write_zeroes(4096*2)?)
	}

	/// Write an offset to the offset table of the Region file.
	pub fn write_offset<C: Into<RegionCoord>,O: Into<RegionSector>>(&mut self, coord: C, offset: O) -> Result<usize,McError> {
		let coord: RegionCoord = coord.into();
		let oldpos = self.writer.stream_position()?;
		coord._seek_to_offset_table(&mut self.writer)?;
		let offset: RegionSector = offset.into();
		let result = offset.write_to(&mut self.writer);
		self.writer.seek(SeekFrom::Start(oldpos))?;
		result
	}

	/// Write a timestamp to the timestamp table of the Region file.
	pub fn write_timestamp<C: Into<RegionCoord>, O: Into<Timestamp>>(&mut self, coord: C, timestamp: O) -> Result<usize,McError> {
		let coord: RegionCoord = coord.into();
		let oldpos = self.writer.stream_position()?;
		coord._seek_to_timestamp_table(&mut self.writer)?;
		let timestamp: Timestamp = timestamp.into();
		let result = timestamp.write_to(&mut self.writer);
		self.writer.seek(SeekFrom::Start(oldpos))?;
		result
	}

	/// Write a chunk to the region file starting at the current
	/// position in the file. After writing the chunk, pad bytes will 
	/// be written to ensure that the region file is a multiple of 4096
	/// bytes.
	/// This function does not write anything to the header. 
	pub fn write_chunk<C: Into<RegionCoord>,T: Writable>(&mut self, coord: C, value: &T) -> Result<RegionSector,McError> {
		let start_sector = self.sector_offset()?;

		todo!()
	}
}

// TODO: 	Create enum for compression level that can be exposed to crate users.
//			I don't want to export flate2::Compression to users of this crate.
/// Writes data to the writer, then pads that data to a 4KiB boundary.
/// This function assume that the writer is already on a 4KiB boundary.
/// If you're not on a 4KiB boundary, you're probably going to corrupt your data.
/// If you manage to not corrupt your data, it will be a miracle.
/// Oh, and compression level can be value from 0 to 9 where 0 is no compression
/// and 9 is best compression.
/// If the function succeeds, it will return the RegionSector where it was written in the writer.
/// (Note: This is a building block function. It's not meant for general usage)
fn _write_padded_region_data<T: Writable, W: Write + Seek>(writer: &mut W, compression_level: u32, data: &T) -> Result<RegionSector,McError> {
	// Figure out what sector we are in. Hopefully we are on a 4KiB boundary.
	// TODO: Perform 4KiB boundary check. Or don't.
	let start_sector = writer.stream_position()? / 4096;
	let compression = Compression::new(compression_level);
	let mut chunk_buffer = Vec::with_capacity(4096);
	let mut compressor = ZlibEncoder::new(chunk_buffer, compression);

	data.write_to(&mut compressor)?;

	chunk_buffer = compressor.finish()?;

	let length = chunk_buffer.len() as u32 + 1; // add 1 for the single byte representing the compression scheme.
	let mut length_buffer = length.to_be_bytes();

	writer.write_all(&length_buffer)?;
	// this is for the compression scheme (2 => ZLib).
	length_buffer[0] = 2;
	writer.write_all(&length_buffer[..1])?;
	writer.write_all(&chunk_buffer)?;

	// add 4 to the length because you have to include the 4 bytes for the length value.
	let required_sectors = _required_sectors(length + 4);
	let padsize = (required_sectors * 4096) - (length + 4);
	writer.write_zeroes(padsize as u64)?;

	Ok(RegionSector::new(start_sector as u32, required_sectors as u8))
}

/// This function will read a value from a reader that is an open region
/// file. The reader is expected to be at the beginning of a 4KiB sector
/// within the file. This function does not perform that check. It will
/// read a 32-bit length, an 8-bit compression scheme (1, 2, or 3), then
/// if will create the appropriate decompressor (if applicable) to read
/// the value from.
/// 
/// If the chunk is not present in the file (a length of zero was read)
/// then `Err(McError::ChunkNotFound)` is returned.
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

/// Write 8KiB of zeroes to writer.
/// This function assumes that the writer's position is already
/// at the start of the file.
/// Returns the number of bytes that were written (Should always be 8192)
fn _write_empty_region_header<W: Write>(writer: &mut W) -> std::io::Result<u64> {
	writer.write_zeroes(1024*8)
}

/// Counts the number of 4KB sectors required to accomodate `size` bytes.
const fn _required_sectors(size: u32) -> u32 {
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