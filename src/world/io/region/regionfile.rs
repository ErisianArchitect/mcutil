// TODO: Remove this when you no longer want to silence the warnings.
#![allow(unused)]

use std::{
	io::{
		BufReader, BufWriter,
		Read, Write,
		Seek, SeekFrom, Cursor,
	},
	fs::{
		File,
	},
	path::{
		Path,
	},
};

use chumsky::chain::Chain;
use flate2::{
	write::ZlibEncoder,
	Compression,
};

use crate::{
	McResult,
	ioext::*, world::io::region::{required_sectors, pad_size},
};

use super::{
	timestamp::*,
	sector::*,
	coord::*,
	sectormanager::*,
	header::*, prelude::CompressionScheme,
};

/// A construct for working with RegionFiles.
/// Allows for reading and writing data from a RegionFile.
pub struct RegionFile {
	header: RegionHeader,
	sector_manager: SectorManager,
	/// This file handle is for both reading and writing.
	file_handle: File,
	/// Because the write size of a value sometimes can't quite be known until
	/// after it has been written, it will be helpful to have a buffer to write
	/// to before writing to the file. This will allow us to know exactly how
	/// many 4KiB blocks are needed to write this data so that a sector can be
	/// allocated.
	write_buf: Cursor<Vec<u8>>,
}

impl RegionFile {
	/// Makes sure that the file located at the path is a valid region file. (must exist and have a size >= 8192).
	fn is_valid_region_file<P: AsRef<Path>>(path: P) -> McResult<bool> {
		let path = path.as_ref();
		// Must be file and the (length + 1) of the file must be large enough
		// to fit the header (8192 bytes). There doesn't need to be any
		// data beyond the header.
		Ok(path.is_file() && path.metadata()?.len() >= (4096*2))
	}

	/// Creates a new [RegionFile] object, opening or creating a Minecraft region file at the given path.
	pub fn new<P: AsRef<Path>>(path: P) -> McResult<Self> {
		let path = path.as_ref();
		if RegionFile::is_valid_region_file(path)? {
			let mut file_handle = File::options().write(true).read(true).open(path)?;
			let header = {				
				let mut temp_reader = BufReader::new((&mut file_handle).take(4096*2));
				RegionHeader::read_from(&mut temp_reader)?
			};
			let sector_manager = SectorManager::from(header.sectors.iter());
			Ok(Self {
				header,
				sector_manager,
				file_handle,
				write_buf: Cursor::new(Vec::with_capacity(4096*2)),
			})
		} else {
			// Create region file with empty header.
			let mut file_handle = File::options().write(true).read(true).create(true).open(path)?;
			// Write empty header.
			{
				let mut temp_writer = BufWriter::new(&mut file_handle);
				temp_writer.write_zeroes(4096*2)?;
			}
			let header = RegionHeader::default();
			let sector_manager = SectorManager::new();
			Ok(Self {
				header,
				sector_manager,
				file_handle,
				write_buf: Cursor::new(Vec::with_capacity(4096*2)),
			})
		}
	}

	pub fn write_data<C: Into<RegionCoord>, T: Writable>(&mut self, coord: C, value: &T) -> McResult<RegionSector> {
		// Get the sector from sector table at coord
		// If the sector is not empty, free it in the SectorManager.
		// Then write the value into the pre-write buffer
		// Once the value is written into the pre-write buffer, the
		// write size is now known, so we can determine how many
		// 4KiB blocks are needed for this sector, allowing us to 
		// allocate it from the SectorManager.
		// Then we flush the data to the writer.
		let coord: RegionCoord = coord.into();
		let sector = self.header.sectors[coord.index()];
		// Clear the buffer for writing
		self.write_buf.get_mut().clear();
		// Gotta write 5 bytes to the buffer so that there's room for the length and the compression scheme.
		// To kill two birds with one stone, I'll write all 2s so that I don't have to go back and write the
		// compression scheme after writing the length.
		self.write_buf.write_all(&[2u8; 5])?;
		// TODO: Allow tweaking the compression.
		// Now we'll write the data to the compressor.
		let mut encoder = ZlibEncoder::new(&mut self.write_buf, Compression::best());
		value.write_to(&mut encoder)?;
		encoder.finish()?;
		// Get the length of the written data by getting the length of the buffer and subtracting 5 (for
		// the bytes that were pre-written in a previous step)
		let length = self.write_buf.get_ref().len() - 5;
		// Get sectors required to accomodate the buffer.
		// + 5 because you need to add the (length_bytes + CompressionScheme)
		let required_sectors = required_sectors((length + 5) as u32);
		// If there is an overflow, return an error because there's no way to write it to the file.
		if required_sectors > 255 {
			return Err(crate::McError::ChunkTooLarge);
		}
		// Write pad zeroes
		// + 5 because you need to add the (length_bytes + CompressionScheme)
		let pad_bytes = pad_size((length + 5) as u64);
		self.write_buf.write_zeroes(pad_bytes)?;
		// Seek back to the beginning to write the length.
		self.write_buf.set_position(0);
		// Add 1 to the length because the specification requires that the compression scheme is included in the length for some reason.
		self.write_buf.write_value((length + 1) as u32)?;
		// Allocation
		let allocation = match self.sector_manager.allocate(required_sectors as u8) {
			Some(value) => value,
			None => return Err(crate::McError::RegionAllocationFailure),
		};
		// Writing to file
		self.file_handle.seek(SeekFrom::Start(allocation.offset()))?;
		self.file_handle.write_all(self.write_buf.get_ref().as_slice())?;
		// Sector
		self.header.sectors[coord.index()] = allocation;
		// Seek to where sector is stored in the header and write the sector.
		let seek_offset = coord.index() * 4;
		self.file_handle.seek(SeekFrom::Start(seek_offset as u64))?;
		self.file_handle.write_value(allocation)?;
		// I'm pretty sure that flush() doesn't do anything, but I'll put it here just in case.
		self.file_handle.flush()?;
		Ok(allocation)
	}

	pub fn write_timestamped<C: Into<RegionCoord>, T: Writable, Ts: Into<Timestamp>>(&mut self, coord: C, value: &T, timestamp: Ts) -> McResult<RegionSector> {
		let coord: RegionCoord = coord.into();
		let allocation = self.write_data(coord, value)?;
		self.header.timestamps[coord.index()] = timestamp.into();
		Ok(allocation)
	}

	pub fn write_with_utcnow<C: Into<RegionCoord>, T: Writable>(&mut self, coord: C, value: &T) -> McResult<RegionSector> {
		self.write_timestamped(coord, value, Timestamp::utc_now())
	}

	pub fn get_sector<C: Into<RegionCoord>>(&self, coord: C) -> RegionSector {
		let coord: RegionCoord = coord.into();
		self.header.sectors[coord.index()]
	}

	pub fn get_timestamp<C: Into<RegionCoord>>(&self, coord: C) -> Timestamp {
		let coord: RegionCoord = coord.into();
		self.header.timestamps[coord.index()]
	}

	pub fn read_data<C: Into<RegionCoord>, T: Readable>(&mut self, coord: C) -> McResult<T> {
		todo!()
	}
}