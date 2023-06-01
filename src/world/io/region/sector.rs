use std::ops::Range;
use crate::ioext::*;
use crate::{
	for_each_int_type,
	McResult,
};
use std::{
	io::{
		Read, Write,
		SeekFrom,
	},
	ops::{
		BitOr,
		Not,
	},
};

/// Offset and size are packed together.
/// Having these two values packed together saves 4KiB per RegionFile.
/// It just seems a little wasteful to use more memory than is necessary.
/// |Offset:3|Size:1|
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct RegionSector(u32);

impl RegionSector {
	/// Provide offset and size in 4KiB chunks.
	pub fn new(offset: u32, size: u8) -> Self {
		Self(offset.overflowing_shl(8).0.bitor(size as u32))
	}

	/// Creates a new empty RegionSector.
	pub const fn empty() -> Self {
		Self(0)
	}

	/// The 4KiB sector offset.
	/// Multiply this by `4096` to get the seek offset.
	pub fn sector_offset(&self) -> u64 {
		self.0.overflowing_shr(8).0 as u64
	}

	/// The 4KiB sector offset that marks the end of this sector and the start of
	/// the next.
	pub fn sector_end_offset(&self) -> u64 {
		self.sector_offset() + self.sector_count()
	}

	/// The 4KiB sector count.
	/// Multiply this by `4096` to get the sector size.
	pub fn sector_count(&self) -> u64 {
		(self.0 & 0xFF) as u64
	}

	/// The offset in bytes that this sector begins
	/// at in the region file.
	pub fn offset(&self) -> u64 {
		self.sector_offset() * 4096
	}

	/// The offset in bytes that this sector ends at in the region file.
	pub fn end_offset(&self) -> u64 {
		self.sector_end_offset() * 4096
	}

	/// The size in bytes that this sector occupies.
	pub fn size(&self) -> u64 {
		self.sector_count() * 4096
	}

	/// Determines if this is an "empty" sector.
	pub fn is_empty(&self) -> bool {
		self.0 == 0
	}

	/// Tests if two sectors intersect.
	pub fn intersects(self, rhs: Self) -> bool {
		(
			self.sector_end_offset() <= rhs.sector_offset()
			|| rhs.sector_end_offset() <= self.sector_offset()
		).not()
	}

	/// There may be cases where [RegionSector] is being used to
	/// represent an unused space in a region file, such as a
	/// deleted chunk. In those cases, the split method can be used
	/// to create two [RegionSector]s.
	/// In the tuple returned, the first sector is the sector being
	/// split from. The second sector is the one of the requested size.
	pub fn split(&self, sector_count: u8) -> Option<(Self, Self)> {
		if (sector_count as u64) <= self.sector_count() {
			let lhs_start = self.sector_offset();
			let lhs_count = (self.sector_count() as u8) - sector_count;
			let rhs_start = lhs_start + (lhs_count as u64);
			Some((
				RegionSector::new(lhs_start as u32, lhs_count),
				RegionSector::new(rhs_start as u32, sector_count)
			))
		} else {
			None
		}
	}

	/// Similar to the split function, splits a [RegionSector] into two sectors, one with
	/// the requested size, and the other with the remainder of the size from the split sector.
	/// Instead of splitting the right hand side, this function splits the left hand side.
	/// That means that the split comes from the lower bound of the sector, and is also the left-hand return value.
	pub fn split_left(&self, sector_count: u8) -> Option<(Self, Self)> {
		if (sector_count as u64) <= self.sector_count() {
			let lhs_start = self.sector_offset();
			let rhs_start = lhs_start + (sector_count as u64);
			let rhs_count = (self.sector_count() as u8) - sector_count;
			Some((
				RegionSector::new(lhs_start as u32, sector_count),
				RegionSector::new(rhs_start as u32, rhs_count)
			))
		} else {
			None
		}
	}
}

macro_rules! __regionsector_impls {
	($type:ty) => {
		impl From<Range<$type>> for RegionSector {
			fn from(value: Range<$type>) -> Self {
				RegionSector::new(value.start as u32, (value.end - value.start) as u8)
			}
		}
	};
}

for_each_int_type!(__regionsector_impls);

impl std::ops::BitAnd for RegionSector {
	type Output = bool;

	/// Checks if two sectors intersect.
	/// Note: If both sectors start at the same position,
	/// but one or both of them are size 0, this will
	/// return false.
	fn bitand(self, rhs: Self) -> Self::Output {
		// If the end offset of either of the sectors is less than or equal
		// to the start offset of the other, that means that they do not
		// intersect.
		self.intersects(rhs)
	}
}

impl Readable for RegionSector {
	fn read_from<R: Read>(reader: &mut R) -> McResult<Self> {
		Ok(Self(reader.read_value()?))
	}
}

impl Writable for RegionSector {
	fn write_to<W: Write>(&self, writer: &mut W) -> McResult<usize> {
		writer.write_value(self.0)
	}
}

impl Seekable for RegionSector {
	/// A [SeekFrom] that points to this [RegionSector]
	fn seeker(&self) -> SeekFrom {
		SeekFrom::Start(self.offset())
	}
}