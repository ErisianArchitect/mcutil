use std::{
	fs::{
		File,
	},
	path::{
		Path,
	},
	io::{
		BufReader,
		Read, Write,
		Seek, SeekFrom,
	},
};
use crate::{
	McResult, McError,
	ioext::*,
};

use super::{
	coord::*,
	sector::*,
	timestamp::*,
	compressionscheme::*,
};

use flate2::{
	read::GzDecoder,
	read::ZlibDecoder,
};

/// An abstraction for reading Region files.
/// You open a region file, pass the reader over to this
/// struct, then you read the offsets/timestamps/chunks
/// that you need. When you're done reading, you can
/// call `.finish()` to take the reader back.
pub struct RegionReader<R: Read + Seek> {
	/// The reader that this [RegionReader] is bound to.
	pub(crate) reader: R,
}

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
		let mut table = Box::new([RegionSector::empty(); 1024]);
		let original_position = self.reader.stream_position()?;
		// Make sure that we aren't already at the beginning of the offset table.
		if original_position != 0 {
			self.reader.seek(SeekFrom::Start(0))?;
		}
		for i in 0..1024 {
			table[i] = self.reader.read_value()?;
		}
		self.reader.seek(SeekFrom::Start(original_position))?;
		Ok(table)
	}

	/// Read entire [Timestamp] table from region file.
	pub fn read_timestamp_table(&mut self) -> McResult<Box<[Timestamp; 1024]>> {
		let mut table = Box::new([Timestamp::default(); 1024]);
		let original_position = self.reader.stream_position()?;
		// Make sure that we aren't already at the beginning of the timestamp table.
		if original_position != 4096 {
			self.reader.seek(SeekFrom::Start(4096))?;
		}
		let mut buffer = [0u8; 4];
		for i in 0..1024 {
			self.reader.read_exact(&mut buffer)?;
			table[i] = Timestamp::from(u32::from_be_bytes(buffer));
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
			// let compression_scheme = buffer[0];
			let compression_scheme = CompressionScheme::read_from(reader)?;
			Ok(match compression_scheme {
				// GZip
				CompressionScheme::GZip => {
					let mut dec = GzDecoder::new(reader.take(length - 1)); // Subtract 1 from length for compression scheme.
					std::io::copy(&mut dec, writer)?
				}
				// ZLib
				CompressionScheme::ZLib => {
					let mut dec = ZlibDecoder::new(reader.take(length - 1)); // Subtract 1 from length for compression scheme.
					std::io::copy(&mut dec, writer)?
				}
				// Uncompressed (since a version before 1.15.1)
				CompressionScheme::Uncompressed => {
					std::io::copy(&mut reader.take(length - 1), writer)?
				}
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

impl<R: Read + Seek> Seek for RegionReader<R> {
	fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
		self.reader.seek(pos)
	}
}