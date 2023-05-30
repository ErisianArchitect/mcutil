// TODO: Remove this when you no longer want to silence the warnings.
#![allow(unused)]

use std::{
	io::{
		BufReader, BufWriter,
		Read, Write,
		Seek, SeekFrom,
	},
	fs::{
		File,
	},
	path::{
		Path,
	},
};

use crate::{
	McResult,
	ioext::*,
};

use super::{
	timestamp::*,
	sector::*,
	coord::*,
	sectormanager::*,
	header::*,
};

/// A construct for working with RegionFiles.
/// Allows for reading and writing data from a RegionFile.
pub struct RegionFile {
	header: RegionHeader,
	sector_manager: SectorManager,
	write_handle: BufWriter<File>,
	read_handle: BufReader<File>
}

impl RegionFile {
	/// Creates a new [RegionFile] object, opening or creating a Minecraft region file at the given path.
	pub fn new<P: AsRef<Path>>(path: P) -> McResult<Self> {
		let path = path.as_ref();
		if path.is_file() {
			let mut writer = BufWriter::new(File::options().write(true).open(path)?);
			let mut reader = BufReader::new(File::options().read(true).open(path)?);
			let header = RegionHeader::read_from(&mut reader)?;
			let sector_manager = SectorManager::from(header.sectors.iter());
			Ok(Self {
				header,
				sector_manager,
				write_handle: writer,
				read_handle: reader,
			})
		} else {
			// Create region file with empty header.
			let mut writer = BufWriter::new(File::create(path)?);
			// Write empty header.
			writer.write_zeroes(8192)?;
			// Don't forget to flush
			writer.flush()?;
			let mut reader = BufReader::new(File::options().read(true).open(path)?);
			let header = RegionHeader::default();
			let sector_manager = SectorManager::new();
			Ok(Self {
				header,
				sector_manager,
				write_handle: writer,
				read_handle: reader,
			})
		}
	}

	pub fn write_without_timestamp<C: Into<RegionCoord>, T: Writable>(&mut self, coord: C, value: &T) -> McResult<RegionSector> {
		// Get the sector from sector table at coord
		// If the sector is not empty, free it in the SectorManager.
		// Then write the value into the pre-write buffer
		// Once the value is written into the pre-write buffer, the
		// write size is now known, so we can determine how many
		// 4KiB blocks are needed for this sector, allowing us to 
		// allocate it from the SectorManager.
		// Then we flush the data to the writer.
		todo!()
	}

	pub fn write_timestamped<C: Into<RegionCoord>, T: Writable, Ts: Into<Timestamp>>(&mut self, coord: C, value: &T, timestamp: Ts) -> McResult<RegionSector> {
		todo!()
	}

	pub fn write_with_utcnow<C: Into<RegionCoord>, T: Writable>(&mut self, coord: C, value: &T) -> McResult<RegionSector> {
		todo!()
	}
}

impl Drop for RegionFile {
	fn drop(&mut self) {
		// TODO: When a RegionFile is dropped, it should flush the header to the file.
		todo!()
	}
}