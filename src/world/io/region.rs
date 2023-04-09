//! Module for creating, reading, and modifying Minecraft region files.

#![allow(unused)]

use core::time;
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
	ops::*, fmt::{write, Debug}, array::IntoIter,
};

use momo::momo;
use chrono::prelude::*;
use flate2::{
	read::GzDecoder,
	read::ZlibDecoder,
	write::ZlibEncoder,
};

pub use flate2::Compression;
use tempfile::tempfile;

use crate::{
	continue_if, break_if, return_if,
	McResult, McError,
	world::chunk,
	nbt::{
		io::NbtWrite,
		io::NbtRead,
		tag::{
			Tag,
			NamedTag,
		},
	},
	math::bit::GetBit,
};
use crate::{ioext::*, math::bit::SetBit};
use crate::world::io::*;
use crate::for_each_int_type;

/* Map of file:
	Traits
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

/*	Planning:
	At some point, there will be a `World` type that will manage
	a Minecraft world. This world type will load chunks for editing
	then save them when necessary.
	In order to make this work properly, I need to create a type
	that keeps track of loaded chunks in a region so that it can save
	those chunks once requested. That means that I'll also need to come
	up with a data structure for chunks.
*/
// ========[ Traits            ]========================

/// You really don't need to worry about this.
pub trait RegionTableItem {
	const OFFSET: u64;
}

// ========[ STRUCTS AND ENUMS ]========================

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

/// This is a bitmask containing 1024 bits.
/// This can be used however you want, but it was created
/// as a way to store flags for present chunks.
pub struct RegionBitmask(Box<[u32; 32]>);

/// A region file contains up to 1024 chunks, which is 32x32 chunks.
/// This struct represents a chunk coordinate within a region file.
/// The coordinate can be an absolute coordinate and it will be
/// normalized to relative coordinates.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug,)]
pub struct RegionCoord(u16);

/// Offset and size are packed together.
/// Having these two values packed together saves 4KiB per RegionFile.
/// It just seems a little wasteful to use more memory than is necessary.
/// |Offset:3|Size:1|
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct RegionSector(u32);

/// A 32-bit Unix timestamp.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub struct Timestamp(pub u32);

// I have an idea! I'll create a special abstraction for the RegionSector
// table and the Timestamp table.

/// A table of 1024 elements that contain information related to
/// a Minecraft chunk within a Region file.
#[derive(Debug, Clone)]
pub struct RegionTable<T: RegionTableItem>(Box<[T; 1024]>);

/// A table of 1024 [RegionSector] elements for each potential chunk in
/// a 32x32 chunk region file.
pub type SectorTable = RegionTable<RegionSector>;
/// A table of 1024 [Timestamp] elements for each potential chunk in a
/// 32x32 chunk region file.
pub type TimestampTable = RegionTable<Timestamp>;

#[derive(Debug, Clone, Default)]
pub struct RegionHeader {
	pub sectors: SectorTable,
	pub timestamps: TimestampTable,
}

/// Info about a region file.
/// This info includes:
/// - Metadata
/// - Chunk Sectors
/// - Timestamps
/// - Which chunks are present
pub struct RegionFileInfo {
	pub(crate) path: PathBuf,
	pub(crate) metadata: std::fs::Metadata,
	pub(crate) header: RegionHeader,
	pub(crate) present_bits: RegionBitmask,
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

// @CompressionScheme

// TODO: Move the following two implementations to the bottom of the file once you
// decide whether or not you would like to keep it.
impl Writable for CompressionScheme {
	fn write_to<W: Write>(&self, writer: &mut W) -> McResult<usize> {
		match self {
			CompressionScheme::GZip => writer.write_all(&[1u8])?,
			CompressionScheme::ZLib => writer.write_all(&[2u8])?,
			CompressionScheme::Uncompressed => writer.write_all(&[3u8])?,
		}
		Ok(1)
	}
}

impl Readable for CompressionScheme {
	fn read_from<R: Read>(reader: &mut R) -> McResult<Self> {
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

impl RegionBitmask {
	/// Creates a new bitmask with all bits set to off.
	pub fn new() -> Self {
		Self(
			Box::new([0; 32])
		)
	}
	
	/// Creates a new bitmask with all bits set to on.
	pub fn new_on() -> Self {
		Self(
			Box::new([u32::MAX; 32])
		)
	}

	pub fn get<C: Into<RegionCoord>>(&self, coord: C) -> bool {
		let coord: RegionCoord = coord.into();
		let index = coord.index();
		let sub_index = index.div_euclid(32);
		let bit_index = index.rem_euclid(32);
		self.0[sub_index].get_bit(bit_index)
	}

	pub fn set<C: Into<RegionCoord>>(&mut self, coord: C, on: bool) {
		let coord: RegionCoord = coord.into();
		let index = coord.index();
		let sub_index = index.div_euclid(32);
		let bit_index = index.rem_euclid(32);
		self.0[sub_index] = self.0[sub_index].set_bit(bit_index, on);
	}

	/// Clear all bits (Setting them to 0).
	pub fn clear(&mut self) {
		self.0.iter_mut().for_each(|value| {
			*value = 0;
		});
	}
}

impl Default for RegionBitmask {
    fn default() -> Self {
        Self::new()
    }
}

impl From<[u32; 32]> for RegionBitmask {
    fn from(value: [u32; 32]) -> Self {
        RegionBitmask(Box::new(value))
    }
}

impl From<[bool; 1024]> for RegionBitmask {
    fn from(value: [bool; 1024]) -> Self {
        let mut mask = RegionBitmask::new();
		value.into_iter()
			.enumerate()
			.for_each(|(index, on)| {
				mask.set(index, on)
			});
		mask
    }
}

impl From<&[bool; 1024]> for RegionBitmask {
    fn from(value: &[bool; 1024]) -> Self {
		let mut mask = RegionBitmask::new();
		value.iter()
			.enumerate()
			.for_each(|(index, &on)| {
				mask.set(index, on)
			});
		mask
    }
}

impl From<RegionBitmask> for [bool; 1024] {
    fn from(value: RegionBitmask) -> Self {
		let mut bits = [false; 1024];
        bits.iter_mut()
			.enumerate()
			.for_each(|(index, bit)| {
				*bit = value.get(index);
			});
		bits
    }
}

impl From<RegionBitmask> for [u32; 32] {
    fn from(value: RegionBitmask) -> Self {
        *value.0
    }
}

impl From<&RegionBitmask> for [u32; 32] {
    fn from(value: &RegionBitmask) -> Self {
        let mut bits = [0u32; 32];
		bits.iter_mut()
			.enumerate()
			.for_each(|(i, bitmask)| {
				*bitmask = value.0[i];
			});
		bits
    }
}

// @RegionCoord

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

impl<T: Into<RegionCoord> + Copy> From<&T> for RegionCoord {
    fn from(value: &T) -> Self {
		T::into(*value)
    }
}

// @RegionSector

impl RegionSector {
	pub fn new(offset: u32, size: u8) -> Self {
		Self(offset.overflowing_shl(8).0.bitor(size as u32))
	}

	/// Creates a new empty RegionSector.
	pub const fn empty() -> Self {
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
	fn read_from<R: Read>(reader: &mut R) -> McResult<Self> {
		Ok(Self(reader.read_value()?))
	}
}

impl Writable for RegionSector {
	fn write_to<W: Write>(&self, writer: &mut W) -> McResult<usize> {
		writer.write_value(self.0)
	}
}

impl Seekable for RegionSector {
	/// A [SeekFrom] that points to this [RegionSector]
	fn seeker(&self) -> SeekFrom {
		SeekFrom::Start(self.offset())
	}
}

// @Timestamp

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

impl<T: Into<Timestamp> + Copy> From<&T> for Timestamp {
    fn from(value: &T) -> Self {
        T::into(*value)
    }
}

impl Readable for Timestamp {
	fn read_from<R: Read>(reader: &mut R) -> McResult<Self> {
		Ok(Self(u32::nbt_read(reader)?))
	}
}

impl Writable for Timestamp {
	fn write_to<W: Write>(&self, writer: &mut W) -> McResult<usize> {
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

// @RegionTableItem

impl RegionTableItem for RegionSector {
	const OFFSET: u64 = 0;
}

impl RegionTableItem for Timestamp {
	const OFFSET: u64 = 4096;
}

// @RegionTable

impl<T: RegionTableItem> RegionTable<T> {
	pub const OFFSET: u64 = T::OFFSET;
	pub fn offset() -> u64 {
		Self::OFFSET
	}

	pub const fn seeker() -> SeekFrom {
		SeekFrom::Start(Self::OFFSET)
	}

	pub fn iter(&self) -> std::slice::Iter<T> {
		self.0.iter()
	}

	pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
		self.0.iter_mut()
	}

	pub fn take_box(self) -> Box<[T; 1024]> {
		self.0
	}

	pub fn take_array(self) -> [T; 1024] {
		*self.0
	}
}

impl<T: RegionTableItem> IntoIterator for RegionTable<T> {
    type Item = T;

    type IntoIter = IntoIter<T, 1024>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T: Default + Copy + RegionTableItem> Default for RegionTable<T> {
	fn default() -> Self {
		Self(Box::new([T::default(); 1024]))
	}
}

impl<C: Into<RegionCoord>,T: RegionTableItem> Index<C> for RegionTable<T> {
	type Output = T;

	fn index(&self, index: C) -> &Self::Output {
		let coord: RegionCoord = index.into();
		&self.0[coord.index()]
	}
}

impl<C: Into<RegionCoord>,T: RegionTableItem> IndexMut<C> for RegionTable<T> {
	fn index_mut(&mut self, index: C) -> &mut Self::Output {
		let coord: RegionCoord = index.into();
		&mut self.0[coord.index()]
	}
}

impl<T: Readable + Debug + RegionTableItem> Readable for RegionTable<T> {
	fn read_from<R: Read>(reader: &mut R) -> McResult<Self> {
		let table: Box<[T; 1024]> = (0..1024).map(|_| {
			T::read_from(reader)
		}).collect::<McResult<Box<[T]>>>()?
		.try_into().unwrap();
		Ok(Self(table))
	}
}

impl<T: Writable + Debug + RegionTableItem + Sized> Writable for RegionTable<T> {
	fn write_to<W: Write>(&self, writer: &mut W) -> McResult<usize> {
		let mut write_size: usize = 0;
		for i in 0..1024 {
			write_size += self.0[i].write_to(writer)?;
		}
		Ok(write_size)
	}
}

impl<T: RegionTableItem> From<[T; 1024]> for RegionTable<T> {
	fn from(value: [T; 1024]) -> Self {
		Self(Box::new(value))
	}
}

impl<T: RegionTableItem> From<RegionTable<T>> for Box<[T; 1024]> {
	fn from(value: RegionTable<T>) -> Self {
		value.0
	}
}

// @RegionHeader

impl Readable for RegionHeader {
	fn read_from<R: Read>(reader: &mut R) -> McResult<Self> {
		Ok(Self {
			sectors: SectorTable::read_from(reader)?,
			timestamps: TimestampTable::read_from(reader)?,
		})
	}
}

impl Writable for RegionHeader {
	fn write_to<W: Write>(&self, writer: &mut W) -> McResult<usize> {
		Ok(
			self.sectors.write_to(writer)? + self.timestamps.write_to(writer)?
		)
	}
}

// @RegionFileInfo

impl RegionFileInfo {

	// TODO: Better documentation.
	/// Gathers information about a region file at the given path.
	pub fn load<P: AsRef<Path>>(path: P) -> McResult<Self> {
		let file = File::open(path.as_ref())?;
		let metadata = std::fs::metadata(path.as_ref())?;
		let mut reader = BufReader::with_capacity(4096, file);
		let header = RegionHeader::read_from(&mut reader)?;
		let mut bits = RegionBitmask::new();
		let counter = 0;
		for i in 0..1024 {
			if !header.sectors[i].is_empty() {
				reader.seek(header.sectors[i].seeker())?;
				let length = u32::read_from(&mut reader)?;
				if length != 0 {
					bits.set(i, true);
				}
			}
		}
		Ok(Self {
			path: PathBuf::from(path.as_ref()),
			metadata,
			header,
			present_bits: bits,
		})
	}

	/// Opens the file that this RegionFileInfo points to.
	pub fn open(&self) -> McResult<File> {
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
		self.header.sectors[coord]
	}

	/// Get the Timestamp for the provided coordinate.
	pub fn get_timestamp<C: Into<RegionCoord>>(&self, coord: C) -> Timestamp {
		self.header.timestamps[coord]
	}

	/// Checks if the chunk exists in the region file.
	pub fn has_chunk<C: Into<RegionCoord>>(&self, coord: C) -> bool {
		self.present_bits.get(coord)
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

// @RegionReader

impl RegionReader<BufReader<File>> {
	/// Opens buffered file as a RegionReader.
	pub fn open_with_capacity(
		capacity: usize,
		path: impl AsRef<Path>,
	) -> McResult<RegionReader<BufReader<File>>> {
		let file = File::open(path)?;
		Ok(RegionReader::with_capacity(capacity, file))
	}
}

impl<R: Read + Seek> RegionReader<R> {
	pub fn new(reader: R) -> Self {
		Self {
			reader,
		}
	}

	pub fn with_capacity(capacity: usize, inner: R) -> RegionReader<BufReader<R>> {
		let reader = BufReader::with_capacity(capacity, inner);
		RegionReader {
			reader
		}
	}

	/// Read a [RegionSector] from the [RegionSector] table in the region file header.
	/// This function preserves the position in the stream that it starts at. That
	/// means that it will seek to the header to read the offset, then it will return
	/// to the position it started at when the function was called.
	pub fn read_offset<C: Into<RegionCoord>>(&mut self, coord: C) -> McResult<RegionSector> {
		let coord: RegionCoord = coord.into();
		let return_offset = self.reader.seek_return()?;
		self.reader.seek(coord.sector_table_offset())?;
		let sector = RegionSector::read_from(&mut self.reader)?;
		self.reader.seek(return_offset)?;
		Ok(sector)
	}

	/// Read entire [RegionSector] table from region file.
	pub fn read_offset_table(&mut self) -> McResult<Box<[RegionSector; 1024]>> {
		let mut table = Box::new([RegionSector(0); 1024]);
		let original_position = self.reader.stream_position()?;
		// Make sure that we aren't already at the beginning of the offset table.
		if original_position != 0 {
			self.reader.seek(SeekFrom::Start(0))?;
		}
		let mut buffer = [0u8; 4];
		for i in 0..1024 {
			table[i] = self.reader.read_value()?;
		}
		self.reader.seek(SeekFrom::Start(original_position))?;
		Ok(table)
	}

	/// Read entire [Timestamp] table from region file.
	pub fn read_timestamp_table(&mut self) -> McResult<Box<[Timestamp; 1024]>> {
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

	/// Read a [RegionSector] from the [RegionSector] table in the region file header.
	/// This function preserves the position in the stream that it starts at. That
	/// means that it will seek to the header to read the offset, then it will return
	/// to the position it started at when the function was called.
	pub fn read_timestamp<C: Into<RegionCoord>>(&mut self, coord: C) -> McResult<Timestamp> {
		let coord: RegionCoord = coord.into();
		let return_offset = self.reader.seek_return()?;
		self.reader.seek(coord.timestamp_table_offset())?;
		let timestamp = Timestamp::read_from(&mut self.reader)?;
		self.reader.seek(return_offset)?;
		Ok(timestamp)
	}

	/// Seek to the sector at the given coordinate.
	/// If the chunk is not found, this function returns [Err(McError::ChunkNotFound)].
	pub fn seek_to_sector<C: Into<RegionCoord>>(&mut self, coord: C) -> McResult<u64> {
		let coord: RegionCoord = coord.into();
		self.reader.seek(coord.sector_table_offset())?;
		let sector = RegionSector::read_from(&mut self.reader)?;
		if sector.is_empty() {
			return Err(McError::ChunkNotFound);
		}
		Ok(self.reader.seek(sector.seeker())?)
	}

	pub fn copy_data_at_coord<W: Write, C: Into<RegionCoord>>(&mut self, coord: C, writer: &mut W) -> McResult<u64> {
		let offset = self.read_offset(coord)?;
		if offset.is_empty() {
			return Ok(0);
		}
		self.reader.seek(offset.seeker())?;
		self.copy_data_from_sector(writer)
	}

	/// Copies data from the current sector in the region file.
	/// If the data is not found, it will not copy anything.
	/// This function does not move the stream before reading. It starts reading from wherever it is in the stream.
	pub fn copy_data_from_sector<W: Write>(&mut self, writer: &mut W) -> McResult<u64> {
		fn copy_from_region_sectors<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> McResult<u64> {
			let mut buffer = [0u8; 4];
			// Read the length of the chunk.
			reader.read_exact(&mut buffer)?;
			let length = u32::from_be_bytes(buffer) as u64;
			if length == 0 {
				return Ok(0);
			}
			// Read compression scheme
			reader.read_exact(&mut buffer[..1])?;
			let compression_scheme = buffer[0];
			Ok(match compression_scheme {
				// GZip
				1 => {
					let mut dec = GzDecoder::new(reader.take(length - 1)); // Subtract 1 from length for compression scheme.
					std::io::copy(&mut dec, writer)?
				}
				// ZLib
				2 => {
					let mut dec = ZlibDecoder::new(reader.take(length - 1)); // Subtract 1 from length for compression scheme.
					std::io::copy(&mut dec, writer)?
				}
				// Uncompressed (since a version before 1.15.1)
				3 => {
					std::io::copy(&mut reader.take(length - 1), writer)?
				}
				invalid_scheme => return Err(McError::InvalidCompressionScheme(invalid_scheme)),
			})
		}
		copy_from_region_sectors(&mut self.reader, writer)
	}

	/// Read data from the region file at the specified coordinate.
	/// Will return None if the data does not exist in the file rather than returning an error.
	pub fn read_data_at_coord<T: Readable, C: Into<RegionCoord>>(&mut self, coord: C) -> McResult<Option<T>> {
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
	pub fn read_data_from_sector<T: Readable>(&mut self) -> McResult<Option<T>> {

		/// This function will read a value from a reader that is an open region
		/// file. The reader is expected to be at the beginning of a 4KiB sector
		/// within the file. This function does not perform that check. It will
		/// read a 32-bit length, an 8-bit compression scheme (1, 2, or 3), then
		/// if will create the appropriate decompressor (if applicable) to read
		/// the value from.
		/// 
		/// If the chunk is not present in the file (a length of zero was read)
		/// then None is returned.
		fn read_from_region_sectors<R: Read,T: Readable>(reader: &mut R) -> McResult<Option<T>> {
			let length = u32::read_from(reader)? as u64;
			if length == 0 {
				return Ok(None);
			}
			let compression_scheme = CompressionScheme::read_from(reader)?;
			Ok(Some(match compression_scheme {
				CompressionScheme::GZip => {
					let mut dec = GzDecoder::new(reader.take(length - 1)); // Subtract 1 from length for compression scheme.
					T::read_from(&mut dec)?
				}
				CompressionScheme::ZLib => {
					let mut dec = ZlibDecoder::new(reader.take(length - 1)); // Subtract 1 from length for compression scheme.
					T::read_from(&mut dec)?
				}
				// Uncompressed (since a version before 1.15.1)
				CompressionScheme::Uncompressed => {
					T::read_from(&mut reader.take(length - 1))? // Subtract 1 from length for compression scheme.
				}
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
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		self.reader.read(buf)
	}
}

// @RegionWriter

impl<W: Write + Seek> Write for RegionWriter<W> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.writer.write(buf)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		self.writer.flush()
	}
}

impl<R: Read + Seek> Seek for RegionReader<R> {
	fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
		self.reader.seek(pos)
	}
}

impl<W: Write + Seek> Seek for RegionWriter<W> {
	fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
		self.writer.seek(pos)
	}
}

impl<W: Write + Seek> RegionWriter<W> {
	pub fn new(writer: W) -> Self {
		Self {
			writer,
		}
	}

	pub fn with_capacity(capacity: usize, inner: W) -> RegionWriter<BufWriter<W>> {
		RegionWriter::<BufWriter<W>>{
			writer: BufWriter::with_capacity(capacity, inner)
		}
	}

	/// Returns the 4KiB offset of the sector that the writer is writing to.
	/// This is NOT the stream position.
	pub fn sector_offset(&mut self) -> McResult<u32> {
		Ok((self.writer.stream_position()? as u32).overflowing_shr(12).0)
	}

	/// This function writes an 8KiB zeroed header to the writer.
	/// In order to reduce system calls and whatever, this function
	/// assumes that you are already at the start of the file.
	/// This is a function that you would call while building a new
	/// region file.
	pub fn write_empty_header(&mut self) -> McResult<u64> {
		Ok(self.writer.write_zeroes(4096*2)?)
	}

	/// Seeks to the beginning of the stream and writes a header.
	pub fn write_header(&mut self, header: RegionHeader) -> McResult<()> {
		let ret = self.writer.seek_return()?;
		self.seek(SeekFrom::Start(0))?;
		header.write_to(&mut self.writer)?;
		self.writer.seek(ret)?;
		Ok(())
	}

	/// Seeks to the table and writes it to the file.
	pub fn write_sector_table(&mut self, table: SectorTable) -> McResult<()> {
		let ret = self.writer.seek_return()?;
		self.seek(SectorTable::seeker())?;
		table.write_to(&mut self.writer)?;
		self.writer.seek(ret)?;
		Ok(())
	}

	/// Seeks to the table and writes it to the file.
	pub fn write_timestamp_table(&mut self, table: TimestampTable) -> McResult<()> {
		let ret = self.writer.seek_return()?;
		self.seek(TimestampTable::seeker())?;
		table.write_to(&mut self.writer)?;
		self.writer.seek(ret)?;
		Ok(())
	}

	/// Write an offset to the offset table of the Region file.
	pub fn write_offset_at_coord<C: Into<RegionCoord>,O: Into<RegionSector>>(&mut self, coord: C, offset: O) -> McResult<usize> {
		let coord: RegionCoord = coord.into();
		let oldpos = self.writer.seek_return()?;
		self.writer.seek(coord.sector_table_offset())?;
		let offset: RegionSector = offset.into();
		let result = self.writer.write_value(offset);
		// Return to the original seek position.
		self.writer.seek(oldpos)?;
		result
	}

	/// Write a [Timestamp] to the [Timestamp] table of the Region file.
	pub fn write_timestamp_at_coord<C: Into<RegionCoord>, O: Into<Timestamp>>(&mut self, coord: C, timestamp: O) -> McResult<usize> {
		let coord: RegionCoord = coord.into();
		let oldpos = self.writer.seek_return()?;
		self.writer.seek(coord.timestamp_table_offset())?;
		let timestamp: Timestamp = timestamp.into();
		let result = self.writer.write_value(timestamp);
		// Return to the original seek position.
		self.writer.seek(oldpos)?;
		result
	}

	/// Write data to Region File, then write the sector that data
	/// was written to into the sector table.
	/// `compression_level` must be a value from 0 to 9, where 0 means
	/// "no compression" and 9 means "take as along as you like" (best compression)
	pub fn write_data_at_coord<T: Writable,C: Into<RegionCoord>>(
		&mut self,
		compression: Compression,
		coord: C,
		data: &T,
	) -> McResult<RegionSector> {
		let sector = self.write_data_to_sector(compression, data)?;
		self.write_offset_at_coord(coord, sector)?;
		Ok(sector)
	}

	//	TODO: Replace compression_level argument with custom type for fine tuning.
	/// Write a chunk to the region file starting at the current
	/// position in the file. After writing the chunk, pad bytes will 
	/// be written to ensure that the region file is a multiple of 4096
	/// bytes.
	/// This function does not write anything to the header. 
	/// Returns the RegionSector that was written to.
	pub fn write_data_to_sector<T: Writable>(
		&mut self,
		compression: Compression,
		data: &T
	) -> McResult<RegionSector> {
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
		let sector_offset = self.writer.stream_position()?;
		// Step 02.)
		// Fast way to make sure writer is on 4KiB boundary.
		if sector_offset & 4095 != 0 {
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
			compression
		);
		// Step 06.)
		data.write_to(&mut compressor)?;
		// Step 07.)
		compressor.finish()?;
		// Step 08.)
		let final_offset = self.writer.stream_position()?;
		// Step 09.)
		let length = (final_offset - sector_offset) + 4;
		let mut length_buffer = length.to_be_bytes();
		// Step 10.)
		let padsize = _pad_size(length + 4);
		self.writer.write_zeroes(padsize)?;
		// Step 11.)
		self.writer.seek(SeekFrom::Start(sector_offset))?;
		// Step 12.)
		self.writer.write_all(&length_buffer)?;
		let length = length as u32;
		Ok(RegionSector::new(
			// Shifting right 12 bits is a shortcut to get the 4KiB sector offset.
			sector_offset.overflowing_shr(12).0 as u32,
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
	pub fn copy_chunk_from<R: Read>(&mut self, reader: &mut R) -> McResult<RegionSector> {
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
		- Check that region 
		file is at least 8KiB in size.
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

/// Returns a [RegionBitmask] that contains the information for what
/// chunks exist in a regon file.
pub fn get_present_chunks(region_path: impl AsRef<Path>) -> McResult<RegionBitmask> {
	let file = File::open(region_path)?;
	let mut reader = BufReader::with_capacity(4096, file);
	let mut bits = RegionBitmask::new();
	let sectors = SectorTable::read_from(&mut reader)?;

	sectors.0.iter()
		.enumerate()
		.try_for_each(|(i, sector)| {
			return_if!(McResult::Ok(()); sector.is_empty());
			reader.seek(sector.seeker())?;
			let length = u32::read_from(&mut reader)?;
			return_if!(McResult::Ok(()); length == 0);
			bits.set(i, true);
			McResult::Ok(())
		})?;

	Ok(bits)
}

#[momo]
pub fn create_empty_region_file(path: impl AsRef<Path>) -> McResult<u64> {
	let file = File::create(path.as_ref())?;
	let mut writer = BufWriter::with_capacity(4096, file);
	let result = writer.write_zeroes(1024*8)?;
	writer.flush()?;
	Ok(result)
}

pub fn read_chunks<I: Into<RegionCoord>, It: IntoIterator<Item = I>, T: Readable>(
	region_file: impl AsRef<Path>,
	it: It
) -> McResult<Vec<(RegionCoord, Option<T>)>> {
	let file = File::open(region_file.as_ref())?;
	let mut reader = RegionReader::with_capacity(4096, file);
	let mut items = Vec::new();
	it.into_iter()
		.map(I::into)
		.try_for_each(|coord| {
			items.push((coord, reader.read_data_at_coord(coord)?));
			McResult::<()>::Ok(())
		})?;
	Ok(items)
}

/*
I have an idea for an algorithm for rebuilding a region file.
Since a region file may be missing chunks, I can iterate over
those. But I also need to account for the edits. So how do I
do that? By zipping the two sets together, and iterating
through all chunks that would be written, set the appropriate
sector values for the sector table, write timestamps if necessary.

If both the available chunks and the edits are sorted, then
it makes it very simple to zip them.

So the basic idea is that you would have two collections of
coordinates/indices.
*/

pub enum EditAction<T> {
	/// For deleting chunks if they exist.
	Delete,
	/// For copying chunks from the old region file into the new one.
	Copy,
	/// For writing data to the new region file.
	Write(T),
	/// For writing data to the new region file with a specific [Timestamp].
	WriteTimestamped(T, Timestamp),
}

// Deleting is less useful, so I don't want to make a generic
// function that does both deleting and writing.
/// Deletes chunks from the region file at the given coordinates.
/// On success, returns the size of the region file after deletion.
pub fn delete_chunks<P, I, It>(region_file: P, it: It) -> McResult<u64>
where
	P: AsRef<Path>,
	I: Into<RegionCoord>,
	It: IntoIterator<Item = I> {
		
	let mut delete: [bool; 1024] = [false; 1024];
	it.into_iter().try_for_each(|coord| {
		let coord: RegionCoord = coord.into();
		delete[coord.index()] = true;
		McResult::Ok(())
	})?;
	
	// Now we can start building the region file.
	let input_file = File::open(region_file.as_ref())?;
	let output_file = tempfile::NamedTempFile::new()?;
	let mut writer = RegionWriter::new(
		BufWriter::with_capacity(4096, output_file)
	);
	let mut reader = RegionReader::new(
		BufReader::with_capacity(4096, input_file)
	);
	// This header will be modified as the region file is being rebuilt, then it will be written
	// to the region file.
	let mut header = RegionHeader::read_from(&mut reader)?;
	// Write the blank header to the writer so that we can get the stream positioned to sector 2.
	// We will later return to the beginning of the file to write the header.
	writer.write_zeroes(1024*8)?;

	// Now we will iterate from 0 to 1023 and write the correct sectors to the file.

	for i in 0..1024 {
		if delete[i] {
			header.sectors[i] = RegionSector::empty();
			header.timestamps[i] = Timestamp(0);
		} else {
			let sector = header.sectors[i];
			continue_if!(sector.is_empty());
			reader.seek(sector.seeker())?;
			header.sectors[i] = writer.copy_chunk_from(&mut reader)?;
		}
	}

	// Seek to beginning of region file to write the header.
	writer.seek(SeekFrom::Start(0))?;
	header.write_to(&mut writer)?;
	writer.flush()?;

	let writer = writer.finish();
	let tempfile_path = writer.get_ref().path();
	Ok(std::fs::copy(tempfile_path, region_file)?)
}

// TODO: Move the RegionBuilder stuff to the appropriate places in the file. Organization is key!
/// A helper for creating or updating region files.
pub struct RegionRebuilder {
	origin: PathBuf,
	header: RegionHeader,
	writer: RegionWriter<BufWriter<tempfile::NamedTempFile>>,
	reader: RegionReader<BufReader<File>>,
	compression: Option<Compression>,
	timestamp: Option<Timestamp>,
}

pub enum BuildAction<T: Writable> {
	Delete,
	Copy,
	Write(T),
	WriteTimestamped(T, Timestamp),
}

impl RegionRebuilder {
	/// Creates a new region builder based on the pre-existing region file.
	pub fn load(region_file: impl AsRef<Path>) -> McResult<Self> {
		let file_origin = region_file.as_ref().to_owned();
		let mut reader = RegionReader::open_with_capacity(BUFFERSIZE, region_file.as_ref())?;
		let mut writer = RegionWriter::with_capacity(BUFFERSIZE, tempfile::NamedTempFile::new()?);
		let header = RegionHeader::read_from(&mut reader)?;
		Ok(
			Self {
				origin: file_origin,
				header,
				writer,
				reader,
				compression: None,
				timestamp: None,
			}
		)
	}

	/// Set the compression value.
	pub fn compression(mut self, value: Compression) -> Self {
		self.compression = Some(value);
		self
	}

	/// Set the default timestamp value.
	pub fn timestamp(mut self, value: Timestamp) -> Self {
		self.timestamp = Some(value);
		self
	}

	/// Creates a new region builder that also creates a new region file.
	pub fn create(region_file: impl AsRef<Path>) -> McResult<Self> {
		create_empty_region_file(region_file.as_ref())?;
		RegionRebuilder::load(region_file)
	}

	/// Rebuilds the region file, calling `callback` for each chunk.
	/// `default_timestamp` is the default timestamp to write to the timestamp table.
	/// If you are writing data to the file, the timestamp must be modified. If no timestamp
	/// is provided by the user, `utc_now` is used.
	pub fn rebuild<C,T,F>(
		// rebuild is only called once.
		mut self,
		mut callback: F,
	) -> McResult<u64>
	where
		C: From<RegionCoord>,
		T: Writable,
		F: FnMut(C) -> BuildAction<T> {
		let mut coord: RegionCoord = RegionCoord(0);
		let compression = self.compression.unwrap_or(Compression::best());
		let default_timestamp = self.timestamp.unwrap_or(Timestamp::utc_now());
		(0..1024usize)
			.try_for_each(|index| {
				let coord = RegionCoord::from(index);
				let action = callback(C::from(coord));
				match action {
					BuildAction::Delete => {
						self.header.sectors[index] = RegionSector::empty();
						self.header.timestamps[index] = Timestamp(0);
					}
					BuildAction::Copy => {
						let sector = self.header.sectors[index];
						return_if!(McResult::Ok(()); sector.is_empty());
						self.reader.seek(sector.seeker())?;
						self.header.sectors[index] = self.writer.copy_chunk_from(&mut self.reader)?;
					},
					BuildAction::Write(value) => {
						self.header.sectors[index] = self.writer.write_data_to_sector(compression, &value)?;
						self.header.timestamps[index] = default_timestamp;
					},
					BuildAction::WriteTimestamped(value, timestamp) => {
						self.header.sectors[index] = self.writer.write_data_to_sector(compression, &value)?;
						self.header.timestamps[index] = timestamp;
					},
				}
				McResult::Ok(())
			})?;
		self.finish()
	}

	/// When you are finished rebuilding, call this function to commit the changes.
	/// This function will also go to the beginning of the writer to write the header
	/// so that you don't have to worry about rewriting it.
	/// to the region file.
	fn finish(mut self) -> McResult<u64> {
		let mut writer = self.writer.finish();
		writer.seek(SeekFrom::Start(0))?;
		self.header.write_to(&mut writer)?;
		let tempfile_path = writer.get_ref().path();
		Ok(std::fs::copy(tempfile_path, self.origin)?)
	}

	// What the fuck is this?
	fn __copy_callback<T: Writable>(coord: RegionCoord) -> BuildAction<T> {
		BuildAction::Copy
	}

}
// TODO: Remove this when you're done
fn regionrebuilder_test() -> McResult<()>{
	use crate::nbt::tag::NamedTag;
	let mut chunk = NamedTag::new(Tag::string("Hello, world!"));
	let target: (i32, i32) = (1, 1);
	let bb = RegionRebuilder::create("test.mcr")?
		.compression(Compression::none())
		.timestamp(Timestamp::utc_now())
		.rebuild(|index: (i32, i32)| {
			if index == target {
				BuildAction::Write(&chunk)
			} else {
				BuildAction::Copy
			}
		})?;
		Ok(())
}

pub fn edit_region_file<T: Writable>(
	region_file: impl AsRef<Path>
	
) -> McResult<u64> {
	let path = region_file.as_ref();
	if !path.is_file() {
		create_empty_region_file(path)?;
	}
	let mut builder = RegionRebuilder::load(region_file)?;

	todo!()
}

pub fn new_write_chunks<P,I,T,It>(
	region_file: P,
)
where
	P: AsRef<Path>,
	I: Into<RegionCoord>,
	T: Writable,
	It: IntoIterator<Item = (I, T)> {

	}

/// Writes the given chunks to the region file at the given coordinates with the given timestamp.
/// `timestamp` is the timestamp you want written to the timestamp table for each new chunk.
/// On success, the return value is the size of the file after writing.
pub fn write_chunks<'a, I: Into<RegionCoord>, T: Writable + 'a, It: IntoIterator<Item = (I, &'a T)>>(
	region_file: impl AsRef<Path>,
	compression: Compression,
	timestamp: Timestamp,
	it: It
) -> McResult<u64> {
	/*	Write the given chunks to a region file, overwriting the chunks at the given coordinates.
		First step is collect the chunks to be written into an array with 1024 elements.
		This is easy to do using Option<T>. We can write all elements as None, then overwrite the ones from the iterator.
	*/
	// On the off chance that the region file has not been created, create one
	let path = region_file.as_ref();
	if !path.is_file() {
		create_empty_region_file(path)?;
	}
	let mut chunks: [Option<&'a T>; 1024] = [None; 1024];
	it.into_iter().try_for_each(|(index, item)| {
		let coord: RegionCoord = index.into();
		if chunks[coord.index()].is_some() {
			return Err(McError::DuplicateChunk);
		}
		chunks[coord.index()] = Some(item);
		Ok(())
	})?;
	// Now we can start building the region file.
	let input_file = File::open(region_file.as_ref())?;
	let output_file = tempfile::NamedTempFile::new()?;
	let mut writer = RegionWriter::new(
		BufWriter::with_capacity(4096, output_file)
	);
	let mut reader = RegionReader::new(
		BufReader::with_capacity(4096, input_file)
	);
	// This header will be modified as the region file is being rebuilt, then it will be written
	// to the region file.
	let mut header = RegionHeader::read_from(&mut reader)?;
	// Write the blank header to the writer so that we can get the stream positioned to sector 2.
	// We will later return to the beginning of the file to write the header.
	writer.write_zeroes(1024*8)?;

	// Now we will iterate from 0 to 1023 and write the correct sectors to the file.

	for i in 0..1024 {
		match chunks[i] {
			// Write the new chunk to the new file.
			Some(chunk) => { 
				header.sectors[i] = writer.write_data_to_sector(compression, chunk)?;
				header.timestamps[i] = timestamp;

			}
			// Copy the old chunk from the old file.
			None => {
				let sector = header.sectors[i];
				continue_if!(sector.is_empty());
				reader.seek(sector.seeker())?;
				header.sectors[i] = writer.copy_chunk_from(&mut reader)?;
			}
		}
	}

	// Seek to beginning of region file to write the header.
	writer.seek(SeekFrom::Start(0))?;
	header.write_to(&mut writer)?;
	writer.flush()?;

	let writer = writer.finish();
	let tempfile_path = writer.get_ref().path();
	Ok(std::fs::copy(tempfile_path, region_file)?)
}

// /// The generalized approach to modifying a Region File.
// fn edit_chunks<P,C,F,It>(region_file: P, it: It) -> McResult<u64>
// where
// 	P: AsRef<Path>,
// 	C: Into<RegionCoord>,
// 	F: FnMut(RegionCoord, )
// 	It: IntoIterator<Item = C> {
	
// 	todo!()
// }

/// This function will sequentially rebuild a region file.
/// There likely isn't really a need for this, but it could
/// potentially be useful in some regard.
/// `input` and `output` can be the same, this function writes to a temporary file
/// before copying over the original file.
pub fn rebuild_region_file<P1: AsRef<Path>, P2: AsRef<Path>>(input: P1, output: P2) -> McResult<u64> {
	fn _rebuild(input: &Path, output: &Path) -> McResult<u64> {
		let input_file = File::open(input)?;
		// Since this function may want to overwrite the input region, it is
		// best that we use a temporary file to write to before copying it
		// over the old region file.		
		let output_file = tempfile::NamedTempFile::new()?;
		// To speed up writing the offset table to the file, I can store
		// the table in memory while the new region file is being built.
		let mut writer = RegionWriter::new(
			BufWriter::with_capacity(4096, output_file)
		);
		let mut reader = RegionReader::new(
			BufReader::with_capacity(4096, input_file)
		);
		let mut sectors = SectorTable::read_from(&mut reader)?;
		// Write blank sector offset table.
		writer.write_zeroes(4096)?;

		// Copy timestamp table since it is assumed that this won't change.
		copy_bytes(&mut reader, &mut writer, 4096)?;

		// Write sectors from reader
		for i in 0..1024 {
			continue_if!(sectors[i].is_empty());
			reader.seek(sectors[i].seeker())?;
			sectors[i] = writer.copy_chunk_from(&mut reader)?;
		}

		writer.writer.seek(SeekFrom::Start(0))?;
		sectors.write_to(&mut writer)?;
		writer.flush()?;
		// Overwrite output file with tempfile.
		let writer = writer.finish();
		let tempfile_path = writer.get_ref().path();
		Ok(std::fs::copy(tempfile_path, output)?)
	}
	_rebuild(input.as_ref(), output.as_ref())
}

/// Checks that all present chunks in a region file are sequential.
/// That is, it checks that chunks are written in a sequential order.
#[momo]
pub fn chunks_are_sequential<P: AsRef<Path>>(region: P) -> McResult<bool> {
	
	let table = {
		let file = File::open(region.as_ref())?;
		let mut reader = BufReader::with_capacity(4096, file);
		SectorTable::read_from(&mut reader)?
	};

	let mut last = table[0];
	for i in 1..1024 {
		// skip empty sectors
		if table[i].is_empty() {
			continue;
		}
		// If the current sector offset is less than or equal to
		// the previous, that means that the chunks are not sequential.
		if table[i].sector_offset() <= last.sector_offset() {
			return Ok(false);
		}
		last = table[i];
	}
	Ok(true)
}


/// Counts how many chunks (out of 1024) exist in a Region file.
#[momo]
pub fn count_chunks(
	region_file: impl AsRef<Path>
) -> McResult<usize> {
	let mut reader = RegionReader::open_with_capacity(4096, region_file)?;
	
	let table = SectorTable::read_from(&mut reader)?;

	let mut count = 0;
	
	for sector in table.0.iter() {
		continue_if!(sector.is_empty());
		reader.seek(sector.seeker())?;
		let length = u32::read_from(&mut reader)?;
		continue_if!(length == 0);
		count += 1;
	}

	Ok(count)
}

/// Counts how many sectors are wasted in the region file.
/// This probably going to return 0, but if it ever does return
/// something besides 0, please let me know. I'm curious.
#[momo]
pub fn wasted_sectors(region: impl AsRef<Path>) -> McResult<u32> {
	let file = File::open(region.as_ref())?;
	let mut reader = BufReader::with_capacity(4096, file);

	let table = SectorTable::read_from(&mut reader)?;

	let mut waste_count = 0u32;

	for i in 0..1024 {
		// skip empty sectors
		if table[i].is_empty() {
			continue;
		}
		reader.seek(table[i].seeker())?;
		let length = u32::read_from(&mut reader)?;
		// This means the sector was wasted.
		if length == 0 {
			waste_count += table[i].sector_count() as u32;
		}
	}

	Ok(waste_count)
}

// TODO:	Perhaps make it possible to provide a file name formatter
//			to control how the chunk names are formatted.
/// Extracts all chunks present in a region file and writes them to an
/// output directory.
/// Note: All chunk names have the following format: `chunk.{x}.{z}.nbt`
/// where `x` and `z` are coordinates relative to the region origin.
#[momo]
pub fn extract_all_chunks(
	region_file: impl AsRef<Path>,
	output_directory: impl AsRef<Path>,
) -> McResult<()> {
	// Iterate through all that are present in Region File, then deposit
	// them into the provided output_directory with 
	// the format: chunk.{x}.{z}.nbt.
	if !output_directory.as_ref().is_dir() {
		std::fs::create_dir_all(output_directory.as_ref())?;
	}
	let region_file = File::open(region_file.as_ref())?;
	let mut reader = RegionReader::new(
		BufReader::with_capacity(4096, region_file)
	);
	// Load the sector table into memory so we don't need to needlessly
	// seek around the file gathering sector data from the table.
	let sectors = SectorTable::read_from(&mut reader)?;
	for i in 0..1024 {
		// Skip empty sectors because there's nothing to extract.
		continue_if!(sectors.0[i].is_empty());
		let coord = RegionCoord::from(i);
		let out_path = output_directory.as_ref().join(format!("chunk.{}.{}.nbt", coord.x(), coord.z()));
		let chunk_file = File::create(out_path)?;
		let mut writer = BufWriter::with_capacity(4096, chunk_file);
		reader.reader.seek(sectors.0[i].seeker())?;
		reader.copy_data_from_sector(&mut writer)?;
	}
	Ok(())
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

// I don't think I need this, but I'm going to keep the code just in case.
// /// Takes a [Result<T,McError>] and transforms it into a [Result<Option<T>,McError>]
// /// where a value of [Err(McError::ChunkNotFound)] is transformed into [Ok(None)].
// fn _filter_chunk_not_found<T>(result: Result<T,McError>) -> Result<Option<T>, McError> {
// 	match result {
// 		Ok(ok) => Ok(Some(ok)),
// 		Err(McError::ChunkNotFound) => Ok(None),
// 		Err(other) => Err(other),
// 	}
// }

// I need a function that collects region coordinates
// into some sort of bitmask that tells me what region coordinates
// are present.

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn region_file_info_test() -> McResult<()> {
		let info = RegionFileInfo::load("r.0.0.mca")?;
		let sect = info.header.sectors[(1,3)];
		Ok(())
	}

	#[test]
	fn region_coord_test() -> McResult<()> {
		let sector = RegionSector::new(0x010203, 0x04);
		let mut file = File::create("buffer.dat")?;
		sector.write_to(&mut file);
		drop(file);
		let mut file = File::open("buffer.dat")?;
		let result = RegionSector::read_from(&mut file)?;
		println!("Sector 1: {} {}", sector.sector_offset(), sector.sector_count());
		println!("Sector 2: {} {}", result.sector_offset(), result.sector_count());
		Ok(())
	}
	
	#[test]
	fn required_sectors_test() {
		assert_eq!(0, _required_sectors(0));
		assert_eq!(1, _required_sectors(1));
		assert_eq!(1, _required_sectors(4095));
		assert_eq!(1, _required_sectors(4096));
		assert_eq!(2, _required_sectors(4097));
	}

	#[test]
	fn pad_test() {
		assert_eq!(0, _pad_size(4096));
		assert_eq!(0, _pad_size(8192));
		assert_eq!(4095, _pad_size(4097));
		assert_eq!(4095, _pad_size(1));
		assert_eq!(1, _pad_size(4095));
		assert_eq!(1, _pad_size(8191));
	}

}
