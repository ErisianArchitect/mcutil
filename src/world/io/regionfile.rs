//!	Contains the abstractions for working with Minecraft Region Files
//! that makes it easier to load and save chunks.

use std::path::{
	Path,
	PathBuf,
};

#[allow(unused)]
use super::region_old::{
	self,
	RegionHeader,
	RegionSector, SectorTable,
	Timestamp, TimestampTable,
};

pub struct RegionFile {
	path: PathBuf,
}

impl RegionFile {
	pub fn new(path: impl AsRef<Path>) -> Self {
		Self {
			path: path.as_ref().to_owned()
		}
	}

	pub fn path(&self) -> &Path {
		&self.path
	}

	pub fn exists(&self) -> bool {
		self.path.is_file()
	}

	

}