use std::{
	ops::Range,
};

use super::{
	sector::*,
};

/// Similar to a RegionSector, but not constrained
/// to only 256 chunks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub struct ManagedSector {
	pub start: u32,
	pub end: u32,
}

impl From<Range<u32>> for ManagedSector{
    fn from(value: Range<u32>) -> Self {
        Self::new(value.start, value.end)
    }
}

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