#![allow(unused)]

use crate::*;
use crate::ioext::Readable;
use super::region::*;
use std::path::*;
use std::io::*;
use std::fs::File;

/*
My plan here is to create a new RegionRebuilder construct.
I can create a RegionManager that can open a region file for
editing and would allow writing and deleting chunks.

So now the new problem to solve is the problem of efficiently
finding unused sectors of the required size within a given
region file.
*/

// TODO: Documentation on this sucks.
/// Manages unused sectors in a region file so that
/// a [RegionManager] can store chunks in a region file without
/// intersection issues. Also manages the end-offset so that it can
/// determine where to start writing new sectors if it runs out of
/// unused chunks.
pub struct SectorManager {
	unused_sectors: Vec<ManagedSector>,
	/// the first offset that is past all the sectors.
	end_offset: u32,
}

/// Similar to a RegionSector, but not constrained
/// to only 256 chunks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ManagedSector {
	start: u32,
	end: u32,
}

impl From<RegionSector> for ManagedSector {
    fn from(value: RegionSector) -> Self {
        ManagedSector::new(
			value.sector_offset() as u32,
			value.sector_end_offset() as u32
		)
    }
}

impl From<(u32, u32)> for ManagedSector {
	fn from(value: (u32, u32)) -> Self {
		ManagedSector::new(value.0, value.1)
	}
}

impl From<ManagedSector> for (u32, u32) {
	fn from(value: ManagedSector) -> Self {
		(value.start, value.end)
	}
}

impl ManagedSector {
	pub const EMPTY: Self = Self { start: 0, end: 0 };
	pub fn new(start: u32, end: u32) -> Self {
		Self {
			start,
			end
		}
	}

	pub fn empty() -> Self {
		Self::EMPTY
	}

	pub fn size(self) -> u32 {
		self.end - self.start
	}

	pub fn start(self) -> u32 {
		self.start
	}

	pub fn end(self) -> u32 {
		self.end
	}

	/// Determines if this sector has a size of 0.
	pub fn is_empty(self) -> bool {
		self.start == self.end
	}
}

impl From<SectorTable> for SectorManager {
	/// Create a SectorManager from a [SectorTable].
	/// This will find all the unused space to initialize the
	/// [SectorManager].
	fn from(value: SectorTable) -> Self {
		use std::cmp::Ordering;
		// Remove empty sectors
		let mut filtered_sectors = value.take_array()
			.into_iter()
			.filter(|sector| !sector.is_empty())
			.collect::<Vec<RegionSector>>();
		// Sort
		filtered_sectors.sort_by(|a,b| {
			if a.sector_end_offset() <= b.sector_offset() {
				Ordering::Less
			} else if b.sector_end_offset() <= a.sector_offset() {
				Ordering::Greater
			} else {
				Ordering::Equal
			}
		});
		// Collect unused sectors
		let (unused_sectors, last) = filtered_sectors.iter().fold(
			( // Initialization of fold()
				Vec::<ManagedSector>::new(), // unused sectors
				// Initialized with the header sectors.
				RegionSector::new(0, 2), // previous state
			), |unused_and_previous, next_sector| {
				let mut unused_sectors = unused_and_previous.0;
				let previous_sector = unused_and_previous.1;
				let gap = next_sector.sector_offset() - previous_sector.sector_end_offset();
				if gap != 0 {
					unused_sectors.push(ManagedSector::new(
						previous_sector.sector_end_offset() as u32,
						gap as u32,
					));
				}
				(unused_sectors, *next_sector)
		});
		Self { 
			unused_sectors,
			end_offset: last.sector_end_offset() as u32,
		}
	}
}

impl SectorManager {
	fn new(unused_sectors: Vec<ManagedSector>, end_offset: u32) -> Self {
		// let mut unused_sectors = unused_sectors;
		// unused_sectors.sort_by(|a, b| {
		// 	use std::cmp::Ordering::*;
		// 	if a.end <= b.start {
		// 		Less
		// 	} else if b.end <= a.start {
		// 		Greater
		// 	} else {
		// 		Equal
		// 	}
		// });
		Self {
			unused_sectors,
			end_offset,
		}
	}

	/// Reads the sector table from a region file and finds all unused
	/// sectors, creating a new [SectorManager] instance in the process.
	pub fn from_file(region_file: impl AsRef<Path>) -> McResult<Self> {
		// Read the sector table from the file.
		let sectors = {
			let mut file = File::open(region_file.as_ref())?;
			SectorTable::read_from(&mut file)?
		};
		Ok(SectorManager::from(sectors))
	}

	/// Frees a sector, allowing it to be reused.
	pub fn free(&mut self, sector: RegionSector) {
		// This method should search through the unused_sectors
		// if there are any and expand the boundaries of any that
		// the given sector is adjacent to.
		// If the given sector is not adjacent to any of the unused
		// sectors, add the sector to the unused sectors.
		// If, for example, this sector fills the space between two
		// unused sectors, those sectors can become a single sector.
		let freed_sector = ManagedSector::new(
			sector.sector_offset() as u32,
			sector.sector_end_offset() as u32,
		);
		let mut left_neighbor: Option<usize> = None;
		let mut right_neighbor: Option<usize> = None;
		// Get neighboring unused sectors if they exist.
		self.unused_sectors
			.iter()
			.enumerate()
			.for_each(|(index, sect)| {
				// Check left side
				if sect.end == freed_sector.start {
					left_neighbor = Some(index);
				// Check right side
				} else if freed_sector.end == sect.start {
					right_neighbor = Some(index);
				}
			});
		if let Some(left_index) = left_neighbor {
			let left_sector = self.unused_sectors[left_index];
			// We are modifying the left neighbor to reflect the new size,
			// so we need to check if there is also a right neighbor
			// that is adjacent, then you combine all three.
			// If there is a right neighbor, the rightmost offset is
			// going to be the right neighbor's end offset.
			// Otherwise it will be the freed sector's end offset.
			let rightmost = if let Some(right_index) = right_neighbor {
				// Since the right neighbor will be absorbed, we must
				// remove it from unused_sectors
				let right_sector = self.unused_sectors.remove(right_index);
				right_sector.end
			} else {
				freed_sector.end
			};
			self.unused_sectors[left_index] = ManagedSector::new(
				left_sector.start,
				rightmost
			);
		} else if let Some(right_index) = right_neighbor {
			let right_sector = self.unused_sectors[right_index];
			// There is a right neighbor, so the right neighbor needs to
			// absorb the sector.
			self.unused_sectors[right_index] = ManagedSector::new(
				freed_sector.start,
				right_sector.end
			);
		} else {
			// There is neither a left nor a right neighbor, so we will
			// So we just add a new sector.
			self.unused_sectors.push(freed_sector);
		}
	}

	/// Allocate a sector of a specified size.
	pub fn allocate(&mut self, size: u8) -> RegionSector {
		// There are no unused_sectors, so we'll just need to create a
		// new one at the end of the file.
		if self.unused_sectors.is_empty() {
			let new_sector = RegionSector::new(
				self.end_offset,
				size,
			);
			self.end_offset = new_sector.sector_end_offset() as u32;
			// return
			new_sector
		} else {
			let index_of_usable_sector = self
				.unused_sectors
				.iter()
				.enumerate()
				.find(|(_, sector)| sector.size() >= (size as u32))
				.map(|(index, _)| index);
			if let Some(index) = index_of_usable_sector {
				let sector = self.unused_sectors[index];
				let reduced_sector = ManagedSector::new(
					// Since we prefer to take the bottom of the sector,
					// we add size to sector.start and keep the end the
					// the same.
					sector.start + (size as u32),
					sector.end
				);
				// If the size is zero, there's nothing left, so just
				// remove it from the unused_sectors cache.
				if reduced_sector.size() == 0 {
					self.unused_sectors.remove(index);
				} else {
					self.unused_sectors[index] = reduced_sector;
				}
				// return
				RegionSector::new(
					sector.start,
					size
				)
			} else {
				let new_sector = RegionSector::new(
					self.end_offset,
					size
				);
				self.end_offset = new_sector.sector_end_offset() as u32;
				// return
				new_sector
			}
		}
	}
}

pub struct RegionManager {
	/// Marks chunks to be copied after the ChunkBuilder is finished
	/// writing/deleting chunks.
	writer: RegionWriter<BufWriter<tempfile::NamedTempFile>>,
	reader: RegionReader<BufReader<File>>,
	header: RegionHeader,
	compression: Compression,
	timestamp: Timestamp,
	copy_bits: RegionBitmask,
}

pub trait ChunkBuilder2 {
	fn build(&mut self, region_file: &mut RegionManager) -> McResult<()>;
}

pub struct RegionBuilder2 {
	origin: PathBuf,
}

impl RegionBuilder2 {
	pub fn build() -> McResult<u64> {
		todo!()
	}
}