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
	ops::*, arch::x86_64::_MM_FROUND_NO_EXC, fmt::write,
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
	Structs
	Implementations
	Public functions
	Private functions
*/

/*	╭──────────────────────────────────────────────────────────────────────────────╮
	│ How do Region Files work?                                                    │
	╰──────────────────────────────────────────────────────────────────────────────╯
	Region files have an 8KiB header that contains two tables, each table with 1024
	elements.

	The first table is the Sector Offset table. Sector offsets are 2 values, the
	actual offset, and the size. Both of these values are packed into 4 bytes. The
	offset is 3 bytes big-endian and the size is 1 byte. They are laid out in 
	memory like so: |offset(3)|size(1)|
	This layout means that when these 4 bytes are turned into a single 32-bit
	unsigned integer, the individual values can be access like so:
		For the offset:	value_u32 >> 8
		For the size:	value_u32 & 0xFF
	This is the first 4KiB.

	Directly fter the offset table is the timestamp table, which also contains 1024
	32-bit values. The timestamps are Unix timestamps in (I believe UTC).

	These 1024 elements in these 2 tables represent data associated with some chunk
	that may be written to the file. There are 32x32 potential slots for chunks.
	If a chunk is not present, the offset value will be 0, or the length within the
	sector is 0 (more on that later.)

	Both values within the sector offset must be multiplied by 4096 in order to get
	the actual value. So to get the stream offset that you must seek to in order to
	find this sector, simple multiple the offset value by 4096. To get the size
	within the file that the data occupies, multiple the size by 4096.

	If the sector offset's values are not 0, there may be a chunk present in the
	file. If you go to the file offset that the sector offset points to, you will
	find a 32-bit unsigned (big-endian) integer representing the size in bytes of
	the data following that unsigned integer. If this value is zero, that means
	there is no data stored, but there is still a sector being occupied. I don't
	know if that is something that happens in region files, I have yet to do that
	research.

	TODO: Research whether or not Minecraft ever saves a sector offset as
		: occupied while the length at that offset is zero.

	Following the length is a single byte representing the compression scheme used
	used to save that chunk. The possible values are 1 for GZip, 2 for ZLib, and 3
	for uncompressed. After the compression scheme are length-1 bytes of data that
	represent a chunk within a Minecraft world, which is in NBT format.

	After the chunk is some pad bytes (typically zeroes, but I don't think that it
	is a requirement that the pad bytes are zeroes).

	The region file's size MUST be a multiple of 4096. I'm pretty sure Minecraft
	will reject it if it's not.
*/

// ========[ STRUCTS AND ENUMS ]========================

// TODO: Utilize CompresssionLevel and CompressionScheme as arguments to
//		 functions that perform compression.
#[repr(u8)]
pub enum CompressionLevel {
	/// Level of 0, uncompressed.
	None = 0,
	/// Level of 1, fastest compression.
	Fastest = 1,
	/// Level of 5.
	Balanced = 5,
	/// Level of 9, which is the best compression but takes the longest time.
	Best = 9,
	/// In case you really want a different compression level.
	/// Must be value between 0 and 9.
	Precise(u8),
}

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

// TODO: Move the following two implementations to the bottom of the file once you
// decide whether or not you would like to keep it.
impl Writable for CompressionScheme {
	fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize,crate::McError> {
		match self {
			CompressionScheme::GZip => writer.write_all(&[1u8])?,
			CompressionScheme::ZLib => writer.write_all(&[2u8])?,
			CompressionScheme::Uncompressed => writer.write_all(&[3u8])?,
		}
		Ok(1)
	}
}

impl Readable for CompressionScheme {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self,crate::McError> {
        let mut buffer = [0u8;1];
		reader.read_exact(&mut buffer)?;
		match buffer[0] {
			1 => Ok(Self::GZip),
			2 => Ok(Self::ZLib),
			3 => Ok(Self::Uncompressed),
			unexpected => Err(McError::InvalidCompressionScheme(unexpected)),
		}
    }
}

/// A region file contains up to 1024 chunks, which is 32x32 chunks.
/// This struct represents a chunk coordinate within a region file.
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

/// A 32-bit Unix timestamp.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Debug)]
pub struct Timestamp(pub u32);

/// Info about a region file.
/// This info includes:
/// - Metadata
/// - Chunk Sectors
/// - Timestamps
/// - Which chunks are present
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

// ========[ Implementations   ]========================

impl RegionCoord {
	/// Create a new RegionCoord.
	/// The x and z will be mathematically transformed into relative coordinates.
	/// So if the coordinate given to `new()` is `(32, 32)`, the result will be
	/// `(0, 0)`.
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

	/// Get a [SeekFrom] value that can be used to seek to the location where
	/// this chunk's sector offset is stored in the sector offset table.
	pub fn sector_table_offset(&self) -> SeekFrom {
		SeekFrom::Start(self.0 as u64 * 4)
	}

	/// Get a [SeekFrom] value that can be used to seek to the location where
	/// this chunk's timestamp is stored in the timestamp table.
	pub fn timestamp_table_offset(&self) -> SeekFrom {
		SeekFrom::Start(self.0 as u64 * 4 + 4096)
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

	/// Creates a new empty RegionSector.
	pub fn empty() -> Self {
		Self(0)
	}

	/// The 4KiB sector offset.
	/// Multiply this by `4096` to get the seek offset.
	pub fn sector_offset(self) -> u64 {
		self.0.overflowing_shr(8).0 as u64
	}

	/// The 4KiB sector offset that marks the end of this sector and the start of
	/// the next.
	pub fn sector_end_offset(self) -> u64 {
		self.sector_offset() + self.sector_count()
	}

	/// The 4KiB sector count.
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

	/// Checks if two sectors intersect.
	/// Note: If both sectors start at the same position, but one or both
	/// of them are size 0, this will return false.
	fn bitand(self, rhs: Self) -> Self::Output {
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

	/// Get a [Timestamp] for the current time (in Utc).
	pub fn utc_now() -> Timestamp {
		Timestamp(
			Utc::now().timestamp() as u32
		)
	}
}

macro_rules! __timestamp_impls {
	($type:ty) => {
		impl From<$type> for Timestamp {
			fn from(value: $type) -> Self {
				Self(value as u32)
			}
		}

		impl From<Timestamp> for $type {
			fn from(value: Timestamp) -> Self {
				value.0 as $type
			}
		}
	};
}

for_each_int_type!(__timestamp_impls);

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

	// TODO: Better documentation.
	/// Gathers information about a region file at the given path.
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

	/// Opens the file that this RegionFileInfo points to.
	pub fn open(&self) -> Result<File,crate::McError> {
		Ok(File::open(&self.path)?)
	}

	/// The path that this RegionFileInfo points to.
	pub fn path(&self) -> &Path {
		&self.path
	}

	/// Get the file's metadata.
	pub fn metadata(&self) -> std::fs::Metadata {
		self.metadata.clone()
	}

	/// Get a RegionSector for the provided coordinate.
	pub fn get_offset<C: Into<RegionCoord>>(&self, coord: C) -> RegionSector {
		self.offsets[coord.into().index()]
	}

	/// Get the Timestamp for the provided coordinate.
	pub fn get_timestamp<C: Into<RegionCoord>>(&self, coord: C) -> Timestamp {
		self.timestamps[coord.into().index()]
	}

	/// Checks if the chunk exists in the region file.
	pub fn has_chunk<C: Into<RegionCoord>>(&self, coord: C) -> bool {
		use crate::math::bit::GetBit;
		let index = coord.into().index();
		let bitword_index = index.div_euclid(64);
		let bit_index = index.rem_euclid(64);
		self.present_bits[bitword_index].get_bit(bit_index)
	}

	/// The time that the file was created.
	pub fn creation_time(&self) -> std::io::Result<std::time::SystemTime> {
		self.metadata.created()
	}

	/// The last modification time of this file.
	pub fn modified_time(&self) -> std::io::Result<std::time::SystemTime> {
		self.metadata.modified()
	}

	/// The last time this file was accessed. (This will probably end up being very
	/// recent since it was accessed for reading to load it.)
	pub fn accessed_time(&self) -> std::io::Result<std::time::SystemTime> {
		self.metadata.accessed()
	}

	/// Returns the size of the region file.
	pub fn size(&self) -> u64 {
		self.metadata.len()
	}

	/// Returns true if the region file has a size
	/// that is a multiple of 4KiB. Minecraft will
	/// consider the region to be corrupted
	/// otherwise.
	pub fn is_correct_size_multiple(&self) -> bool {
		(self.size() & 4095) == 0
	}

}

impl<R: Read + Seek> RegionReader<R> {
	pub fn new(reader: R) -> Self {
		Self {
			reader,
		}
	}

	/// Read a sector offset from the sector offset table in the region file header.
	/// This function preserves the position in the stream that it starts at. That
	/// means that it will seek to the header to read the offset, then it will return
	/// to the position it started at when the function was called.
	pub fn read_offset<C: Into<RegionCoord>>(&mut self, coord: C) -> Result<RegionSector, crate::McError> {
		let coord: RegionCoord = coord.into();
		let return_offset = self.reader.seek_return()?;
		self.reader.seek(coord.sector_table_offset())?;
		let sector = RegionSector::read_from(&mut self.reader)?;
		self.reader.seek(return_offset)?;
		Ok(sector)
	}

	/// Read entire [RegionSector] table from region file.
	pub fn read_offset_table(&mut self) -> Result<Box<[RegionSector; 1024]>,McError> {
		let mut table = Box::new([RegionSector(0); 1024]);
		let original_position = self.reader.stream_position()?;
		// Make sure that we aren't already at the beginning of the offset table.
		if original_position != 0 {
			self.reader.seek(SeekFrom::Start(0))?;
		}
		let mut buffer = [0u8; 4];
		for i in 0..1024 {
			self.reader.read_exact(&mut buffer)?;
			table[i] = RegionSector(u32::from_be_bytes(buffer));
		}
		self.reader.seek(SeekFrom::Start(original_position))?;
		Ok(table)
	}

	/// Read entire [Timestamp] table from region file.
	pub fn read_timestamp_table(&mut self) -> Result<Box<[Timestamp; 1024]>,McError> {
		let mut table = Box::new([Timestamp(0); 1024]);
		let original_position = self.reader.stream_position()?;
		// Make sure that we aren't already at the beginning of the timestamp table.
		if original_position != 4096 {
			self.reader.seek(SeekFrom::Start(4096))?;
		}
		let mut buffer = [0u8; 4];
		for i in 0..1024 {
			self.reader.read_exact(&mut buffer)?;
			table[i] = Timestamp(u32::from_be_bytes(buffer));
		}
		self.reader.seek(SeekFrom::Start(original_position))?;
		Ok(table)
	}

	/// Read a timestamp from the timestamp table in the region file header.
	/// This function preserves the position in the stream that it starts at. That
	/// means that it will seek to the header to read the offset, then it will return
	/// to the position it started at when the function was called.
	pub fn read_timestamp<C: Into<RegionCoord>>(&mut self, coord: C) -> Result<Timestamp,McError> {
		let coord: RegionCoord = coord.into();
		let return_offset = self.reader.seek_return()?;
		self.reader.seek(coord.timestamp_table_offset())?;
		let timestamp = Timestamp::read_from(&mut self.reader)?;
		self.reader.seek(return_offset)?;
		Ok(timestamp)
	}

	/// Seek to the sector at the given coordinate.
	/// If the chunk is not found, this function returns Err(McError::ChunkNotFound).
	pub fn seek_to_sector<C: Into<RegionCoord>>(&mut self, coord: C) -> Result<u64,McError> {
		let coord: RegionCoord = coord.into();
		self.reader.seek(coord.sector_table_offset())?;
		let sector = RegionSector::read_from(&mut self.reader)?;
		if sector.is_empty() {
			return Err(McError::ChunkNotFound);
		}
		Ok(self.reader.seek(sector.seeker())?)
	}

	/// Read data from the region file at the specified coordinate.
	/// Will return None if the data does not exist in the file rather than returning an error.
	pub fn read_data_at_coord<T: Readable, C: Into<RegionCoord>>(&mut self, coord: C) -> Result<Option<T>,McError> {
		let offset = self.read_offset(coord)?;
		if offset.is_empty() {
			return Ok(None);
		}
		self.reader.seek(offset.seeker())?;
		self.read_data_from_sector()
	}
	
	/// Read data from the current sector in the region file.
	/// If the data is not found, it will return None.
	/// This function does not move the stream before reading. It starts reading from wherever it is in the stream.
	pub fn read_data_from_sector<T: Readable>(&mut self) -> Result<Option<T>,McError> {

		/// This function will read a value from a reader that is an open region
		/// file. The reader is expected to be at the beginning of a 4KiB sector
		/// within the file. This function does not perform that check. It will
		/// read a 32-bit length, an 8-bit compression scheme (1, 2, or 3), then
		/// if will create the appropriate decompressor (if applicable) to read
		/// the value from.
		/// 
		/// If the chunk is not present in the file (a length of zero was read)
		/// then None is returned.
		fn read_from_region_sectors<R: Read,T: Readable>(reader: &mut R) -> Result<Option<T>,McError> {
			let mut buffer = [0u8; 4];
			// Read the length of the chunk.
			reader.read_exact(&mut buffer)?;
			let length = u32::from_be_bytes(buffer) as u64;
			if length == 0 {
				return Ok(None);
			}
			// Read compression scheme
			reader.read_exact(&mut buffer[..1])?;
			let compression_scheme = buffer[0];
			Ok(Some(match compression_scheme {
				// GZip
				1 => {
					let mut dec = GzDecoder::new(reader.take(length - 1)); // Subtract 1 from length for compression scheme.
					T::read_from(&mut dec)?
				}
				// ZLib
				2 => {
					let mut dec = ZlibDecoder::new(reader.take(length - 1)); // Subtract 1 from length for compression scheme.
					T::read_from(&mut dec)?
				}
				// Uncompressed (since a version before 1.15.1)
				3 => {
					T::read_from(&mut reader.take(length - 1))? // Subtract 1 from length for compression scheme.
				}
				invalid_scheme => return Err(McError::InvalidCompressionScheme(invalid_scheme)),
			}))
		}
		// Due to the way the borrow checker works, it's best to throw all this code into its own function.
		read_from_region_sectors(&mut self.reader)
	}

	/// Finish reading and return the contained reader.
	pub fn finish(self) -> R {
		self.reader
	}
}

impl<R: Read + Seek> Read for RegionReader<R> {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		self.reader.read(buf)
	}
}

impl<W: Write + Seek> Write for RegionWriter<W> {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		self.writer.write(buf)
	}

	fn flush(&mut self) -> io::Result<()> {
		self.writer.flush()
	}
}

impl<R: Read + Seek> Seek for RegionReader<R> {
	fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
		self.reader.seek(pos)
	}
}

impl<W: Write + Seek> Seek for RegionWriter<W> {
	fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
		self.writer.seek(pos)
	}
}

impl<W: Write + Seek> RegionWriter<W> {
	pub fn new(writer: W) -> Self {
		Self {
			writer,
		}
	}

	/// Returns the 4KiB offset of the sector that the writer is writing to.
	/// This is NOT the stream position.
	pub fn sector_offset(&mut self) -> Result<u32,McError> {
		Ok((self.writer.stream_position()? as u32).overflowing_shr(12).0)
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
		let oldpos = self.writer.seek_return()?;
		self.writer.seek(coord.sector_table_offset())?;
		let offset: RegionSector = offset.into();
		let result = self.writer.write_value(offset);
		// Return to the original seek position.
		self.writer.seek(oldpos)?;
		result
	}

	/// Write a timestamp to the timestamp table of the Region file.
	pub fn write_timestamp<C: Into<RegionCoord>, O: Into<Timestamp>>(&mut self, coord: C, timestamp: O) -> Result<usize,McError> {
		let coord: RegionCoord = coord.into();
		let oldpos = self.writer.seek_return()?;
		self.writer.seek(coord.timestamp_table_offset())?;
		let timestamp: Timestamp = timestamp.into();
		let result = self.writer.write_value(timestamp);
		// Return to the original seek position.
		self.writer.seek(oldpos)?;
		result
	}

	//	TODO: Replace compression_level argument with custom type for fine tuning.
	/// Write a chunk to the region file starting at the current
	/// position in the file. After writing the chunk, pad bytes will 
	/// be written to ensure that the region file is a multiple of 4096
	/// bytes.
	/// This function does not write anything to the header. 
	/// Returns the RegionSector that was written to.
	pub fn write_data_to_sector<T: Writable>(&mut self, compression_level: u32, data: &T) -> Result<RegionSector,McError> {
		/*	╭────────────────────────────────────────────────────────────────────────────────────────────────╮
			│ Instead of using an in-memory buffer to do compression, I'll write                             │
			│ directly to the writer. This should speed things up a bit, and reduce                          │
			│ resource load.                                                                                 │
			│ Steps:                                                                                         │
			│ 01.) Retrieve starting position in stream (on 4KiB boundary).                                  │
			│ 02.) Check that position is on 4KiB boundary.                                                  │
			│ 03.) Move the stream forward 4 bytes.                                                          │
			│ 04.) Write the compression scheme (2 for ZLib) .                                               │
			│ 05.) Create ZLib encoder from writer.                                                          │
			│ 06.) Write the data.                                                                           │
			│ 07.) Release the ZLib encoder.                                                                 │
			│ 08.) Get the final offset.                                                                     │
			│ 09.) Subtract starting offset from final offset then add 4 (for the length) to get the length. │
			│ 10.) Write pad zeroes.                                                                         │
			│ 11.) Return to the starting offset.                                                            │
			│ 12.) Write length.                                                                             │
			╰────────────────────────────────────────────────────────────────────────────────────────────────╯*/
		// Step 01.)
		let position = self.writer.stream_position()?;
		// Step 02.)
		// Fast way to make sure writer is on 4KiB boundary.
		if position & 4095 != 0 {
			return Err(McError::StreamSectorBoundaryError);
		}
		// Step 03.)
		self.writer.seek(SeekFrom::Current(4))?;
		// Compression scheme buffer. (2 for ZLib)
		let compression_scheme = [2u8; 1];
		// Step 04.)
		self.writer.write_all(&compression_scheme)?;
		// Step 05.)
		let mut compressor = ZlibEncoder::new(
			&mut self.writer,
			Compression::new(compression_level.min(9))
		);
		// Step 06.)
		data.write_to(&mut compressor)?;
		// Step 07.)
		compressor.finish()?;
		// Step 08.)
		let final_offset = self.writer.stream_position()?;
		// Step 09.)
		let length = (final_offset - position) + 4;
		let mut length_buffer = length.to_be_bytes();
		// Step 10.)
		let padsize = _pad_size(length + 4);
		self.writer.write_zeroes(padsize)?;
		// Step 11.)
		self.writer.seek(SeekFrom::Start(position))?;
		// Step 12.)
		self.writer.write_all(&length_buffer)?;
		let length = length as u32;
		Ok(RegionSector::new(
			// Shifting right 12 bits is a shortcut to get the 4KiB sector offset.
			position.overflowing_shr(12).0 as u32,
			// add 4 to the length because you have to include the 4 bytes for the length value.
			_required_sectors(length + 4) as u8
		))
	}

	/// Copies a chunk from a reader into this writer.
	/// This function assumes that the given reader is already positioned
	/// to the beginning of the sector that you would like to copy from.
	/// 
	/// For a refresher on region file format, each sector begins with a
	/// 32-bit unsigned big-endian length value, which represents the
	/// length in bytes that the sector data occupies. This length also
	/// includes a single byte for the compression scheme (which is 
	/// irrellevant for copying).
	/// This function will read that length, then it will copy the sector
	/// data over to the writer. If the length is zero, nothing is copied
	/// and the value returned is an empty RegionSector.
	pub fn copy_chunk_from<R: Read>(&mut self, reader: &mut R) -> Result<RegionSector,McError> {
		let sector_offset = self.sector_offset()?;
		let mut length_buffer = [0u8; 4];
		reader.read_exact(&mut length_buffer)?;
		let length = u32::from_be_bytes(length_buffer);
		// The length is zero means that there isn't any data in this
		// sector, but the sector is still being used. That means it's
		// a wasted sector. This can be fixed by simply not writing
		// anything to the writer and returning an empty RegionSector
		// to tell anything upstream that nothing was written.
		if length == 0  {
			return Ok(RegionSector::empty());
		}
		// Copy the length to the writer. Very important step.
		self.write_all(&length_buffer)?;
		copy_bytes(reader, &mut self.writer, length as u64)?;
		// The padsize is the number of bytes required to to put
		// the writer on a 4KiB boundary. You have to add 4 because you need
		// to include the 4 bytes for the length.
		let padsize = _pad_size((length + 4) as u64);
		self.writer.write_zeroes(padsize)?;
		Ok(RegionSector::new(
			sector_offset,
			// + 4 to include the 4 bytes holding the length.
			_required_sectors(length + 4) as u8
		))
	}


	/// Returns the inner writer.
	pub fn finish(self) -> W {
		self.writer
	}
}

// ========[ PUBLIC FUNCTIONS  ]========================
//	TODO: Public interface.
/*	What should public functions be able to do?
	- Verify a region file. (Create custom type to hold the region integrity information)
		- Check that region file is multiple of 4KiB in size.
		- Check that region file is at least 8KiB in size.
		- Check that all sector offsets in the offset table are non-intersecting. (This may prove to be a bit difficult.)
		- Check that all timestamps are less than current time.
		- Check that all allocated sectors have valid NBT data.
		- (Maybe?) Check that each chunk's NBT data has a valid structure.
		- Check that each allocated chunk has valid `xPos` and zPos` nodes, and the xPos and zPos are correct.
	- Attempt data recovery from corrupted region file.
		This should be fairly simple. Just walk through the region file looking for each chunk that's present.
		Attempt to read that chunk from the sector, and if it is successfully read, it is written to the new
		region.
	- Check if chunk sectors are sequential.
		This shouldn't be necessary, but I thought it would be interesting to do anyway.
	- Rewrite region file so that sectors are sequential.
		Yet again, I don't think that this should be necessary.
	- Remove blank chunks that take up sectors.
		If there is a sector offset in the offset table that points to a non-empty sector, but the `length` value at
		the beginning of that sector is zero, that sector can be effectively removed.
	- Delete chunks.
		Rebuild region file with all chunks except the ones that you want to delete.
	- Write chunks to region file, replacing any existing chunks.
		Just like deleting, this will need to rebuild the region file, injecting the chunks that you would like to write and
		copying the ones you aren't trying to overwrite.
	- Open series of chunks.
		This could be more than one function depending on needs. But I would like to be able to open multiple chunks at the same
		time. Either by selecting a rectangular region, or by providing the exact coordinates of the chunks that you would like
		to open.
	- Extract all chunks into directory.
		Extract all chunks in region file into a directory, each chunk being an NBT file.
	- Build region file from directory.
		Take chunks from within a directory and build them into a region file.
	- Create detailed report about region file.
		Off the top of my head, this report could include things like number of chunks, most recent write time,
		earliest write time, time per chunk, etc. There are all kinds of things that could be included in such a report.
	- Recompress file
		rebuild a file with a new compression scheme, or none at all!
*/

/// This function will sequentially rebuild a region file.
/// There likely isn't really a need for this, but it could
/// potentially be useful in some regard.
/// `input` and `output` can be the same, this function writes to a temporary file
/// before copying over the original file.
pub fn rebuild_region_file<P1: AsRef<Path>, P2: AsRef<Path>>(input: P1, output: P2) -> Result<u64,McError> {
	fn _rebuild(input: &Path, output: &Path) -> Result<u64,McError> {
		let input_file = File::open(input)?;
		// Since this function may want to overwrite the input region, it is
		// best that we use a temporary file to write to before copying it
		// over the old region file.		
		let output_file = tempfile::NamedTempFile::new()?;
		// To speed up writing the offset table to the file, I can store
		// the table in memory while the new region file is being built.
		let mut sectors = [RegionSector::empty(); 1024];
		let mut writer = RegionWriter::new(
			BufWriter::with_capacity(4096, output_file)
		);
		let mut reader = RegionReader::new(
			BufReader::with_capacity(4096, input_file)
		);
		// Write blank sector offset table.
		writer.write_zeroes(4096)?;
		// Copy timestamp table since it is assumed that this won't change.
		copy_bytes(&mut reader, &mut writer, 4096)?;

		// Write sectors from reader
		for i in 0..1024 {
			// to spare confusion:
			// let-else (https://rust-lang.github.io/rfcs/3137-let-else.html)
			let Some(_) = _filter_chunk_not_found(reader.seek_to_sector(i))?
			else { continue };
			sectors[i] = writer.copy_chunk_from(&mut reader)?;
		}
		writer.seek(SeekFrom::Start(0))?;
		// Write the sector offset table
		for i in 0..1024 {
			writer.write_value(sectors[i])?;
		}

		// Overwrite output file with tempfile.
		let writer = writer.finish();
		let tempfile_path = writer.get_ref().path();
		Ok(std::fs::copy(tempfile_path, output)?)
	}
	_rebuild(input.as_ref(), output.as_ref())
}

// ========[ PRIVATE FUNCTIONS ]========================

/// Counts the number of 4KiB sectors required to accomodate `size` bytes.
const fn _required_sectors(size: u32) -> u32 {
	// Yay for branchless programming!
	let sub = size.overflowing_shr(12).0;
	// use some casting magic to turn a boolean into an integer.
	// true => 1 | false => 0
	let overflow = ((size & 4095) != 0) as u32;
	sub + overflow
}

/// Returns the 4KiB pad size for the given size.
/// The pad size is the number of bytes required
/// to add to the size in order to make it a
/// multiple of 4096.
const fn _pad_size(size: u64) -> u64 {
	// Some bit-level hacking makes this really easy.
	(4096 - (size & 4095)) & 4095
}

/// Takes a [Result<T,McError>] and transforms it into a [Result<Option<T>,McError>]
/// where a value of [Err(McError::ChunkNotFound)] is transformed into [Ok(None)].
fn _filter_chunk_not_found<T>(result: Result<T,McError>) -> Result<Option<T>, McError> {
	match result {
		Ok(ok) => Ok(Some(ok)),
		Err(McError::ChunkNotFound) => Ok(None),
		Err(other) => Err(other),
	}
}

#[cfg(test)]
mod tests {
	
	#[test]
	fn required_sectors_test() {
		use super::*;
		assert_eq!(0, _required_sectors(0));
		assert_eq!(1, _required_sectors(1));
		assert_eq!(1, _required_sectors(4095));
		assert_eq!(1, _required_sectors(4096));
		assert_eq!(2, _required_sectors(4097));
	}

	#[test]
	fn pad_test() {
		use super::*;
		assert_eq!(0, _pad_size(4096));
		assert_eq!(0, _pad_size(8192));
		assert_eq!(4095, _pad_size(4097));
		assert_eq!(4095, _pad_size(1));
		assert_eq!(1, _pad_size(4095));
		assert_eq!(1, _pad_size(8191));
	}

}
