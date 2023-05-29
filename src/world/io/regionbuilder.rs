#![allow(unused)]

use crate::*;
use crate::ioext::Readable;
use super::region_old::*;
use std::ops::Range;
use std::ops::RangeInclusive;
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

/// Similar to a RegionSector, but not constrained
/// to only 256 chunks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub struct ManagedSector {
	start: u32,
	end: u32,
}

// TODO: Documentation on this sucks.
/// Manages unused sectors in a region file so that
/// a [RegionManager] can store chunks in a region file without
/// intersection issues. Also manages the end-offset so that it can
/// determine where to start writing new sectors if it runs out of
/// unused chunks.
pub struct SectorManager {
	/// The unused sectors in a region file.
	/// Expect that this might not be sorted.
	unused_sectors: Vec<ManagedSector>,
	/// This represents all the occupyable space beyond all
	/// used sectors.
	/// This is where new or too large sectors will be allocated.
	end_sector: ManagedSector,
}

impl From<Range<u32>> for ManagedSector{
    fn from(value: Range<u32>) -> Self {
        Self::new(value.start, value.end)
    }
}

impl PartialOrd for ManagedSector {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		use std::cmp::Ordering::*;
        Some(if self.end <= other.start {
			Less
		} else if other.end <= self.start {
			Greater
		} else if self == other {
			Equal
		} else {
			return None;
		})
    }
}

// impl Ord for ManagedSector {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
// 		use std::cmp::Ordering::*;
//         if self.end <= other.start {
// 			Less
// 		} else if other.end <= self.start {
// 			Greater
// 		} else {
// 			Equal
// 		}
//     }
// }

impl std::fmt::Display for ManagedSector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(start: {}, end: {})", self.start, self.end)
    }
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
	pub const EMPTY: Self = Self::new(0, 0);
	pub const HEADER: Self = Self::new(0, 2);
	pub const FULL: Self = Self::new(u32::MIN, u32::MAX);
	pub const INACCESSIBLE: Self = Self::new(16777215, u32::MAX);
	const FULL_WITHOUT_HEADER: Self = Self::new(2, u32::MAX);
	/// Create a new [ManagedSector] from the start and end offsets.
	/// Ensure that `start` <= `end`.
	pub const fn new(start: u32, end: u32) -> Self {
		Self {
			start,
			end
		}
	}

	pub const fn from_bounds(a: u32, b: u32) -> Self {
		if a < b {
			Self::new(a, b)
		} else {
			Self::new(b, a)
		}
	}

	/// Creates a new [ManagedSector] that starts at `end_offset` and
	/// ends at `u32::MAX`.
	pub const fn end_sector(end_offset: u32) -> Self {
		Self {
			start: end_offset,
			end: u32::MAX
		}
	}

	/// Returns an empty [ManagedSector]
	pub const fn empty() -> Self {
		Self::EMPTY
	}

	/// Returns a [ManagedSector] that represents the header of
	/// a region file.
	pub const fn header() -> Self {
		Self::HEADER
	}

	pub fn size(&self) -> u32 {
		if self.end < self.start {
			println!("Corrupt: Start = {} End = {}", self.start, self.end);
		}
		self.end - self.start
	}

	pub const fn start(&self) -> u32 {
		self.start
	}

	pub const fn end(&self) -> u32 {
		self.end
	}
	
	/// Returns a [SeekFrom] that will seek to the start of this sector.
	pub const fn seeker(&self) -> std::io::SeekFrom {
		std::io::SeekFrom::Start((self.start * 4096) as u64)
	}

	/// Determines if this sector has a logical start and end.
	/// (sector.start <= sector.end)
	pub const fn is_valid(&self) -> bool {
		self.start <= self.end
	}

	/// Determines if this sector has a size of 0.
	pub const fn is_empty(&self) -> bool {
		self.start == self.end
	}

	/// Determines if this sector has a size greater than 0.
	pub const fn not_empty(&self) -> bool {
		self.start < self.end
	}

	/// Measures the gap between two sectors. Order does not matter.
	/// Returns None if there is no gap.
	pub fn gap(&self, other: &Self) -> Option<u32> {
		if self.end < other.start {
			Some(other.start - self.end)
		} else if other.end < self.start {
			Some(self.start - other.end)
		} else {
			None
		}
	}

	/// Absorbs the other [ManagedSector] and all space in between into
	/// this [ManagedSector].
	pub fn absorb(&mut self, other: Self) {
		self.start = self.start.min(other.start);
		self.end = self.end.max(other.end);
	}

	/// Allocates a [RegionSector] from this [ManagedSector], reducing
	/// the size in the process. Returns `None` if there isn't enough
	/// space.
	pub fn allocate(&mut self, size: u8) -> Option<RegionSector> {
		let new_start = self.start + (size as u32);
		// Not enough space.
		if new_start > self.end {
			return None
		}
		let start = self.start;
		self.start = new_start;
		Some(RegionSector::new(start, size))
	}

	/// Attempts to reduce the size of a sector by moving the start
	/// offset.
	pub fn reduce(&self, size: u32) -> Option<Self> {
		let new_start = self.start + size;
		// Not enough space.
		if new_start > self.end {
			return None;
		}
		Some(ManagedSector::new(
			new_start,
			self.end
		))
	}

	/// Checks intersection between two sectors.
	pub fn intersects(&self, other: &Self) -> bool {
		self.start < other.end
		&& other.start < self.end
	}
}

impl<'a> IntoIterator for &'a SectorManager {

    type Item = &'a ManagedSector;
	// type IntoIter = std::iter::Map<std::slice::Iter<'a, ManagedSector>, fn(&ManagedSector) -> ManagedSector>;
	type IntoIter = std::slice::Iter<'a, ManagedSector>;

    fn into_iter(self) -> Self::IntoIter {
		self.unused_sectors.iter()
    }
}

impl<'a> IntoIterator for &'a mut SectorManager {

    type Item = &'a mut ManagedSector;
	type IntoIter = std::slice::IterMut<'a, ManagedSector>;

    fn into_iter(self) -> Self::IntoIter {
		self.unused_sectors.iter_mut()
    }
}

/// Create [SectorManager] from an Iterator.
impl<T: Into<ManagedSector>, It: IntoIterator<Item = T>> From<It> for SectorManager {
	/// Try not to feed the [SectorManager] collections that
	/// have intersecting sectors.
    fn from(value: It) -> Self {
		use std::cmp::Ordering;
		// Filter out empty sectors.
		let mut filtered_sectors = value.into_iter()
			.map(T::into)
			.filter(ManagedSector::not_empty)
			.collect::<Vec<ManagedSector>>();
		// In order to measure the gap between sectors, they must
		// be sorted.
		filtered_sectors.sort_by(|a,b| {
			if a.end <= b.start {
				Ordering::Less
			} else if b.end <= a.start {
				Ordering::Greater
			// Non-equal sectors can evaluate to equal.
			} else {
				Ordering::Equal
			}
		});
		let initial_state = (
			Vec::<ManagedSector>::new(),
			// Initialized with the header sectors.
			ManagedSector::header(),
		);
		// Collect unused sectors
		let (
			unused_sectors,
			// Since the sectors are ordered, the last sector in the fold
			// will be the caboose.
			end_sector
		) = filtered_sectors.into_iter()
			.fold(initial_state,|(mut unused_sectors, previous), sector| {	
				if let Some(gap) = previous.gap(&sector) {
					unused_sectors.push(ManagedSector::new(
						previous.end,
						sector.start
					));
				}
				// Initialize the state for the next iteration.
				( 
					unused_sectors,
					sector
				)
			});
		Self { 
			unused_sectors,
			end_sector: ManagedSector::end_sector(end_sector.end)
		}
    }
}


impl SectorManager {
	pub fn new() -> Self {
		Self {
			unused_sectors: Vec::new(),
			// Initialize the end_sector to the accessible range (24-bits).
			end_sector: ManagedSector::new(2, 0xFFFFFF),
		}
	}
	/// Creates a new [SectorManager] with the specified unused sectors.
	/// Please provide only valid and non-empty sectors. Also, avoid
	/// adding sectors that intersect. I'm putting a lot of trust into
	/// you to not give this function bad data!
	pub fn with_unused(end_sector: ManagedSector, unused_sectors: Vec<ManagedSector>) -> Self {
		Self {
			unused_sectors,
			end_sector,
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

	/// Creates a [SectorManager] from a [SectorTable].
	pub fn from_table(table: SectorTable) -> Self {
		Self::from(table)
	}

	pub fn unused_sectors(&self) -> &Vec<ManagedSector> {
		&self.unused_sectors
	}

	pub fn end_sector(&self) -> &ManagedSector {
		&self.end_sector
	}

	pub fn unused_count(&self) -> usize {
		self.unused_sectors.len()
	}

	// TODO: I'm pretty sure that this will cause problems
	//       if the given sector intersects with an unused sector.
	//       It's best if you only supply RegionSectors supplied by
	//       the same instance of a sector manager.
	/// Frees a sector, allowing it to be reused.
	pub fn free(&mut self, sector: RegionSector) {
		// Early return if the sector is empty (nothing to free)
		if sector.size() == 0 {
			return;
		}
		// This method should search through the unused_sectors
		// if there are any and expand the boundaries of any that
		// the given sector is adjacent to.
		// If the given sector is not adjacent to any of the unused
		// sectors, add the sector to the unused sectors.
		// If, for example, this sector fills the space between two
		// unused sectors, those sectors can become a single sector.
		let mut freed_sector = ManagedSector::from(sector);
		let mut left_neighbor: Option<usize> = None;
		let mut right_neighbor: Option<usize> = None;
		// Get neighboring unused sectors if they exist.
		self.unused_sectors
			.iter()
			.enumerate()
			// The .filter step feels a little extraneous.
			// .filter(|(index, sector)| {
			// 	sector.end == freed_sector.start 
			// 	|| freed_sector.end == sector.start
			// })
			.for_each(|(index, sector)| {
				// Check left side
				if sector.end == freed_sector.start {
					left_neighbor = Some(index);
				// Check right side
				} else if freed_sector.end == sector.start {
					right_neighbor = Some(index);
				}
			});
		// I'm using Vec::swap_remove to remove items, which
		// means that I'll want to remove the items from right
		// to left
		// If you'd like to know why, I'll give a brief explanation.
		// Let's say you have a collection like this:
		// ["Zero", "One", "Two", "Three", "Four"]
		// If you call swap_remove on the item at index 1 ("One"),
		// It will take the item at the end ("Four") and place it
		// at index 1.
		// Now if you wanted to remove the item that was previously
		// at the end, that item is now at index 1, which is not
		// the end index.
		// If you do this from right to left, you get a different
		// result.
		// You would first remove the item at index 4, it would simply
		// reduce the size of the collection by one. Then you could
		// remove the item at index 1 and it would swap it with the
		// item at the end (index 3 "Three").
		match (left_neighbor, right_neighbor) {
			(Some(left), Some(right)) => {
				freed_sector.absorb(
					self.unused_sectors.swap_remove(right.max(left))
				);
				freed_sector.absorb(
					self.unused_sectors.swap_remove(left.min(right))
				);
			}
			(Some(index), None) => {
				// You do not need to absorb the end sector, that is
				// done in the next step.
				freed_sector.absorb(
					self.unused_sectors.swap_remove(index)
				);
			}
			(None, Some(index)) => {
				freed_sector.absorb(
					self.unused_sectors.swap_remove(index)
				);
			}
			_ => ()
		}
		// If the freed sector borders the end_sector, absorb it into
		// the end_sector
		if freed_sector.end >= self.end_sector.start {
			self.end_sector.absorb(freed_sector);
		// otherwise add the freed sector to the unused_sectors.
		} else {
			self.unused_sectors.push(freed_sector);
		}
	}

	/// Allocate a sector of a specified size.
	pub fn allocate(&mut self, size: u8) -> RegionSector {
		self.unused_sectors.iter()
			// Dereference the sector to satisfy borrow checker.
			.map(|sector| *sector)
			// We'll need the index of the found sector.
			.enumerate()
			// Find a sector that is at least as large as the requested
			// size.
			.find(|(_, sector)| sector.size() >= (size as u32))
			// If a sector is found, we can reduce the size of it by
			// the requested size (removing it if the size becomes 0).
			.and_then(|(index, mut sector)| {
				let result = sector.allocate(size).unwrap();
				if sector.is_empty() {
					self.unused_sectors.swap_remove(index);
				} else {
					self.unused_sectors[index] = sector;
				}
				// return
				Some(result)
			})
			// If there was no sector found of the appropriate size,
			// create a new sector at the end and move the end_offset
			// to the end of that sector.
			.unwrap_or_else(||{
				// Since we know that the end_sector will have enough
				// space, we'll just call expect.
				self.end_sector
					.allocate(size)
					// In the unlikely scenario that the SectorManager runs
					// out of space, we'll just do this.
					.expect("The SectorManager's end_sector was not large enough for the allocation.")
			})
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