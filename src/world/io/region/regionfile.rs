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
	file_handle: Option<File>,
}

impl RegionFile {
	/// Creates a new [RegionFile] object, opening or creating a Minecraft region file at the given path.
	pub fn new<P: AsRef<Path>>(path: P) -> McResult<Self> {
		let path = path.as_ref();
		if path.is_file() {
			let mut reader = BufReader::new(File::open(path)?);
			let header = RegionHeader::read_from(&mut reader)?;
			let sector_manager = SectorManager::from_table(header.sectors.clone());
			Ok(Self {
				header,
				sector_manager,
				file_handle: None,
			})
		} else {
			let mut writer = BufWriter::new(File::open(path)?);
			// Create empty region file.
			writer.write_zeroes(8192)?;
			let header = RegionHeader::default();
			let sector_manager = SectorManager::new();
			Ok(Self {
				header,
				sector_manager,
				file_handle: None,
			})
		}
	}

	pub fn write_timestamped<C: Into<RegionCoord>, T: Writable, Ts: Into<Timestamp>>(&mut self, coord: C, value: &T, timestamp: Ts) -> McResult<RegionSector> {
		todo!()
	}

	pub fn write_without_timestamp<C: Into<RegionCoord>, T: Writable>(&mut self, coord: C, value: &T) -> McResult<RegionSector> {
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