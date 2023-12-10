use std::{
	path::Path,
	fs::File, io::BufReader,
};

use crate::{
	McResult, McError,
	ioext::*,
};

use super::prelude::*;

pub trait SectorAllocator {
	fn free(&mut self, sector: RegionSector);
	#[must_use]
	fn allocate(&mut self, size: u8) -> Option<RegionSector>;
	#[must_use]
	fn reallocate(&mut self, free: RegionSector, new_size: u8) -> Option<RegionSector>;

	#[must_use]
	#[inline(always)]
	fn allocate_err(&mut self, size: u8) -> McResult<RegionSector> {
		self.allocate(size).ok_or(McError::RegionAllocationFailure)
	}

	#[must_use]
	#[inline(always)]
	fn reallocate_err(&mut self, free: RegionSector, new_size: u8) -> McResult<RegionSector> {
		self.reallocate(free, new_size).ok_or(McError::RegionAllocationFailure)
	}
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
	pub(super) unused_sectors: Vec<ManagedSector>,
	/// This represents all the occupyable space beyond all
	/// used sectors.
	/// This is where new or too large sectors will be allocated.
	pub(super) end_sector: ManagedSector,
}

impl SectorAllocator for SectorManager {
	

	// TODO: I'm pretty sure that this will cause problems
	//       if the given sector intersects with an unused sector.
	//       It's best if you only supply RegionSectors supplied by
	//       the same instance of a sector manager.
	/// Frees a sector, allowing it to be reused.
	fn free(&mut self, sector: RegionSector) {
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
		#[derive(Debug, Default)]
		struct Finder {
			left: Option<usize>,
			right: Option<usize>,
		}
		let mut finder = Finder::default();
		// Get neighboring unused sectors if they exist.
		self.unused_sectors
			.iter()
			.map(|&s| s)
			.enumerate()
			.find_map(|(index, sector)| {
				match (finder.left, finder.right) {
					(None, Some(_)) => {
						if sector.end == freed_sector.start {
							finder.left = Some(index);
							return Some(());
						}
						None
					}
					(Some(_), None) => {
						if freed_sector.end == sector.start {
							finder.right = Some(index);
							return Some(());
						}
						None
					}
					(None, None) => {
						if sector.end == freed_sector.start {
							finder.left = Some(index);
						} else if freed_sector.end == sector.start {
							finder.right = Some(index);
						}
						None
					}
					_ => Some(())
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
		match (finder.left, finder.right) {
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
	#[must_use]
	fn allocate(&mut self, size: u8) -> Option<RegionSector> {
		self.unused_sectors
			.iter()
			// Dereference the sector to satisfy borrow checker.
			.map(|sector| *sector)
			// We'll need the index of the found sector.
			.enumerate()
			// Find a sector that is at least as large as the requested
			// size.
			.find(|(_, sector)| sector.size() >= (size as u32))
			// If a sector is found, we can reduce the size of it by
			// the requested size (removing it if the size becomes 0).
			.and_then(|(index, sector)| {
				let (new_sector, old_sector) = sector.split_left(size as u32).unwrap();
				if old_sector.is_empty() {
					self.unused_sectors.swap_remove(index);
				} else {
					self.unused_sectors[index] = old_sector.into();
				}
				Some(RegionSector::from(new_sector))
			})
			// If there was no sector found of the appropriate size,
			// create a new sector at the end and move the end_offset
			// to the end of that sector.
			.or_else(||{
				// Since we know that the end_sector will have enough
				// space, we'll just call expect.
				self.end_sector
					.allocate(size)
			})
	}

	/// This will allocate a new sector, and if successful (and necessary), free the old one.
	/// This method will return the sector passed to it if the requested size is the same as
	/// the size of the sector. If the new size is smaller than the requested sector, then
	/// the new sector will be split from the old sector and the old sector will be freed.
	/// Most sectors will be 1 block in size, so this function will probably return the
	/// sector passed to it in most cases.
	#[must_use]
	fn reallocate(&mut self, free: RegionSector, new_size: u8) -> Option<RegionSector> {
		// There's no need to free the sector if there is no reallocation happening.
		if new_size == 0 {
			return None;
		}
		// We don't need to do an allocation if our freed sector is big enough to accomodate the new size.
		if free.sector_count() >= (new_size as u64) {
			// No need to reallocate.
			if free.sector_count() == (new_size as u64) {
				Some(free)
			} else {
				// Use split_left so that when the right side is freed, it can be absorbed
				// into the end_sector if they are adjacent.
				let (new, old) = free.split_left(new_size).unwrap();
				self.free(old);
				Some(new)
			}
		} else if free.is_empty() {
			// The sector is empty, so there's nothing to free.
			self.allocate(new_size)
		} else {
			self.reallocate_unchecked(free, new_size)
		}
	}
}

impl SectorManager {
	pub fn new() -> Self {
		Self {
			unused_sectors: Vec::new(),
			// Initialize the end_sector to the accessible range (24-bits).
			end_sector: ManagedSector::new(2, u32::MAX),
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
			let mut reader = BufReader::new(File::open(region_file.as_ref())?);
			SectorTable::read_from(&mut reader)?
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

	/// Counts the number of unused 4KiB blocks. This is helpful for determining
	/// if the region file needs to be optimized.
	pub fn count_unused_blocks(&self) -> u32 {
		self.unused_sectors.iter()
			.map(|sect| sect.size())
			.sum()
	}

	/// This function will only cause the [SectorManager] to change its state if it succeeds in allocating a sector.
	/// Failure is unlikely because you would need a ridiculously large file (which is possible, but unlikely).
	/// This function does not check if the sector being freed is big enough to hold the requested size (hence the `unchecked`).
	#[must_use]
	#[inline(always)]
	fn reallocate_unchecked(&mut self, free: RegionSector, new_size: u8) -> Option<RegionSector> {
		#[derive(Default)]
		struct Finder {
			left: Option<usize>,
			right: Option<usize>,
			alloc: Option<usize>,
		}
		let mut freed_sector = ManagedSector::from(free);
		let mut finder = Finder::default();
		/// Checks that the supplied option is none and that the condition is met.
		/// If the conditions are met, the option is set to the supplied value.
		/// Returns the result of the conditions.
		macro_rules! apply_some_condition {
			($opt:expr, $condition:expr, $value:expr) => {
				if $opt.is_none() && ($condition) {
					$opt = Some($value);
					true
				} else {
					false
				}
			};
		}
		self.unused_sectors
			.iter()
			.map(|s| *s)
			.enumerate()
			.find_map(|(index, sector)| {
				if apply_some_condition!(finder.alloc,	sector.size() >= (new_size as u32),	index)
				|| apply_some_condition!(finder.left,	sector.end == freed_sector.start,	index)
				|| apply_some_condition!(finder.right,	sector.start == freed_sector.end,	index) {
					if let (Some(_), Some(_), Some(_)) = (finder.alloc, finder.left, finder.right) {
						return Some(());
					}
				}
				None
			});
		// In order to preserve state upon failure, I've created a temporary enum type to
		// store values for success actions.
		enum SuccessAction {
			/// Replace the sector at index.
			Replace(usize, ManagedSector),
			/// Remove sector at index.
			Remove(usize),
			/// No action.
			None,
		}
		finder.alloc.map(|index| {
			let result = self.unused_sectors[index];
			if result.size() > (new_size as u32) {
				let (new, old) = result.split_left(new_size as u32).unwrap();
				(
					RegionSector::from(new),
					SuccessAction::Replace(index, old)
				)
			} else {
				(
					RegionSector::from(result),
					SuccessAction::Remove(index)
				)
			}
		})
		.or_else(|| {
			self.end_sector
				.allocate(new_size)
				.map(|sector| (sector, SuccessAction::None))
		})
		.map(|(sector, action)| {
			match (finder.left, finder.right) {
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
			if freed_sector.end >= self.end_sector.start {
				self.end_sector.absorb(freed_sector);
			} else {
				self.unused_sectors.push(freed_sector);
			}
			match action {
				SuccessAction::Replace(index, old) => {
					self.unused_sectors[index] = old;
				}
				SuccessAction::Remove(index) => {
					self.unused_sectors.swap_remove(index);
				}
				SuccessAction::None => ()
			}
			sector
		})
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
		let mut filtered_sectors = value.into_iter()
			.map(ManagedSectorIteratorItem::convert)
			.filter(ManagedSector::not_empty)
			.collect::<Vec<ManagedSector>>();
		filtered_sectors.sort();
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