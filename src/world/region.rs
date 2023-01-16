// TODO: Remove this eventually.
#![allow(unused)]

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
		File,
	}, result, str::FromStr, ops::BitOr
};

use std::io::prelude::*;
use flate2::read::GzDecoder;
use flate2::read::ZlibDecoder;

use chrono::prelude::*;

use crate::nbt::{tag::NamedTag, NbtError, io::ReadNbt, io::NbtRead};
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum RegionError {
	#[error("io error.")]
	IO(#[from] std::io::Error),
	#[error("NBT error")]
	Nbt(#[from] crate::nbt::NbtError),
	#[error("Chunk doesn't exist.")]
	ChunkError,
	#[error("Invalid compression scheme.")]
	InvalidCompressionScheme(u8),
}

/*
Layout of region module:
	Functions:
		create_region_file<P: AsRef<Path>>(path: P) -> 
*/

pub fn create_region_file<P: AsRef<Path>>(path: P, x: i64, z: i64) -> std::io::Result<()> {
	// 	A region consists of three portions:
	// 	4KiB sector for Chunk Offsets
	//	4KiB sector for Timestamps
	//	Then minimum chunk allocation for each
	Ok(())
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
		(self.0.overflowing_shr(8).0 as u64) * 4096
	}

	pub fn size(&self) -> u64 {
		((self.0 & 0xFF) as u64) * 4096
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

impl Timestamp {

	pub fn to_datetime(&self) -> Option<DateTime<Utc>> {
		let naive = NaiveDateTime::from_timestamp_opt(self.0 as i64, 0);
		if let Some(naive) = naive {
			Some(DateTime::<Utc>::from_utc(naive, Utc))
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
	path: PathBuf,
	timestamps: Vec<Timestamp>,
	offsets: Vec<ChunkOffset>,
	metadata: std::fs::Metadata,
}

impl RegionFileInfo {

	pub fn load<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
		let file = File::open(path.as_ref())?;
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
		let metadata = std::fs::metadata(path.as_ref())?;
		Ok(Self {
			path: PathBuf::from(path.as_ref()),
			timestamps,
			offsets,
			metadata,
		})
	}

	pub fn path(&self) -> &Path {
		&self.path
	}

	pub fn open(&self) -> std::io::Result<File> {
		File::open(&self.path)
	}

	pub fn get_timestamp(&self, x: i64, z: i64) -> Timestamp {
		let index = RegionFile::get_index(x,z);
		self.timestamps[index as usize]
	}

	pub fn get_offset(&self, x: i64, z: i64) -> ChunkOffset {
		let index = RegionFile::get_index(x,z);
		self.offsets[index as usize]
	}

}

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

	/// This function gets the flat index (in an array of 1024 elements) from a 32x32 grid.
	/// This is a helper function to help find the offset in a region file where certain information is stored.
	pub(crate) const fn get_index(x: i64, z: i64) -> u64 {
		(x.rem_euclid(32) + z.rem_euclid(32) * 32) as u64
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

	pub(crate) fn read_chunk_offset<R: Read + Seek>(reader: &mut R, x: i64, z: i64) -> Result<ChunkOffset,RegionError> {
		let offset = RegionFile::get_index(x, z) * 4;
		reader.seek(SeekFrom::Start(offset))?;
		Ok(ChunkOffset::read(reader)?)
	}

	pub(crate) fn read_timestamp<R: Read + Seek>(reader: &mut R, x: i64, z: i64) -> Result<Timestamp, RegionError> {
		let offset = RegionFile::get_index(x, z) * 4;
		let mut buffer = [0u8; 4];
		reader.seek(SeekFrom::Start(offset + 4096))?;
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

	pub fn get_timestamp(&self, x: i64, z: i64) -> Result<Timestamp, RegionError> {
		let mut file = self.open()?;
		let mut reader = BufReader::with_capacity(4096, file);
		RegionFile::read_timestamp(&mut reader, x, z)
	}

	pub fn get_chunk_nbt(&self, x: i64, z: i64) -> Result<NamedTag,RegionError> {
		let mut file = self.open()?;
		let mut reader = BufReader::with_capacity(4 << 10, file);
		let offset = RegionFile::read_chunk_offset(&mut reader, x, z)?;
		// Seek to the offset
		reader.seek(SeekFrom::Start(offset.offset()))?;
		let mut buffer = [0u8; 4];
		// Read the length of the chunk.
		reader.read_exact(&mut buffer)?;
		// 1 is subtracted from the lenth that is read because there is 1 byte for the compression scheme, and the rest of the length is the data.
		let length = (u32::from_be_bytes(buffer) as u64) - 1;
		// Read compression scheme
		reader.read_exact(&mut buffer[..1])?;
		let compression_scheme = buffer[0];
		match compression_scheme {
			// GZip
			1 => {
				let mut dec = GzDecoder::new(reader.take(length));
				Ok(dec.read_nbt()?)
			}
			// ZLib
			2 => {
				let mut dec = ZlibDecoder::new(reader.take(length));
				Ok(dec.read_nbt()?)
			}
			// Uncompressed (since a version before 1.15.1)
			3 => {
				Ok(reader.take(length).read_nbt()?)
			}
			_ => return Err(RegionError::InvalidCompressionScheme(compression_scheme)),
		}
	}
}