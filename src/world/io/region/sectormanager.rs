use std::{
	path::{
		Path
	},
	fs::{
		File,
	},
};

use crate::{
	McResult, McError,
	ioext::*,
};

use super::{
	prelude::*,
};

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

impl SectorManager {
	pub fn new() -> Self {
		Self {
			unused_sectors: Vec::new(),
			// Initialize the end_sector to the accessible range (24-bits).
			end_sector: ManagedSector::new(2, 0x1000000),
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
	pub fn from_table(table: &SectorTable) -> Self {
		Self::from(table.iter())
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

	/// This will allocate a new sector, and if succesful, free the old one.
	#[must_use]
	pub fn reallocate_err(&mut self, free: RegionSector, new_size: u8) -> McResult<RegionSector> {
		let result = self.allocate_err(new_size);
		if result.is_ok() {
			self.free(free);
		}
		result
	}

	/// This will allocate a new sector, and if successful, free the old one.
	#[must_use]
	pub fn reallocate(&mut self, free: RegionSector, new_size: u8) -> Option<RegionSector> {
		let result = self.allocate(new_size);
		if result.is_some() {
			self.free(free);
		}
		result
	}

	/// A version of allocate that returns a result rather than an option.
	#[must_use]
	pub fn allocate_err(&mut self, size: u8) -> McResult<RegionSector> {
		self.allocate(size).ok_or(McError::RegionAllocationFailure)
	}

	/// Allocate a sector of a specified size.
	#[must_use]
	pub fn allocate(&mut self, size: u8) -> Option<RegionSector> {
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
			.or_else(||{
				// Since we know that the end_sector will have enough
				// space, we'll just call expect.
				self.end_sector
					.allocate(size)
					// In the unlikely scenario that the SectorManager runs
					// out of space, we'll just do this.
					// .expect("The SectorManager's end_sector was not large enough for the allocation. This kind of failure should not happen.")
			})
			// .ok_or(crate::McError::RegionAllocationFailure)
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

/// Helper trait for working with iterators converting them into SectorManagers.
pub trait ManagedSectorIteratorItem {
	fn convert(self) -> ManagedSector;
}

impl ManagedSectorIteratorItem for RegionSector {
	fn convert(self) -> ManagedSector {
		ManagedSector::from(self)
	}
}

impl ManagedSectorIteratorItem for &RegionSector {
	fn convert(self) -> ManagedSector {
		ManagedSector::from(*self)
	}
}

impl ManagedSectorIteratorItem for ManagedSector {
	fn convert(self) -> ManagedSector {
		self
	}
}

impl ManagedSectorIteratorItem for &ManagedSector {
	fn convert(self) -> ManagedSector {
		*self
	}
}

impl<'a,T: ManagedSectorIteratorItem, It: IntoIterator<Item = T>> From<It> for SectorManager {
	fn from(value: It) -> Self {
		use std::cmp::Ordering;
		let mut filtered_sectors = value.into_iter()
			.map(ManagedSectorIteratorItem::convert)
			.filter(ManagedSector::not_empty)
			.collect::<Vec<ManagedSector>>();
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
				if let Some(_) = previous.gap(&sector) {
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

// /// Create [SectorManager] from an Iterator.
// impl<T: Into<ManagedSector>, It: IntoIterator<Item = T>> From<It> for SectorManager {
// 	/// Try not to feed the [SectorManager] collections that
// 	/// have intersecting sectors.
//     fn from(value: It) -> Self {
// 		use std::cmp::Ordering;
// 		// Filter out empty sectors.
// 		let mut filtered_sectors = value.into_iter()
// 			.map(T::into)
// 			.filter(ManagedSector::not_empty)
// 			.collect::<Vec<ManagedSector>>();
// 		// In order to measure the gap between sectors, they must
// 		// be sorted.
// 		filtered_sectors.sort_by(|a,b| {
// 			if a.end <= b.start {
// 				Ordering::Less
// 			} else if b.end <= a.start {
// 				Ordering::Greater
// 			// Non-equal sectors can evaluate to equal.
// 			} else {
// 				Ordering::Equal
// 			}
// 		});
// 		let initial_state = (
// 			Vec::<ManagedSector>::new(),
// 			// Initialized with the header sectors.
// 			ManagedSector::header(),
// 		);
// 		// Collect unused sectors
// 		let (
// 			unused_sectors,
// 			// Since the sectors are ordered, the last sector in the fold
// 			// will be the caboose.
// 			end_sector
// 		) = filtered_sectors.into_iter()
// 			.fold(initial_state,|(mut unused_sectors, previous), sector| {	
// 				if let Some(_) = previous.gap(&sector) {
// 					unused_sectors.push(ManagedSector::new(
// 						previous.end,
// 						sector.start
// 					));
// 				}
// 				// Initialize the state for the next iteration.
// 				( 
// 					unused_sectors,
// 					sector
// 				)
// 			});
// 		Self { 
// 			unused_sectors,
// 			end_sector: ManagedSector::end_sector(end_sector.end)
// 		}
//     }
// }