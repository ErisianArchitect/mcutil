use crate::for_each_int_type;
use std::io::SeekFrom;

/// A region file contains up to 1024 chunks, which is 32x32 chunks.
/// This struct represents a chunk coordinate within a region file.
/// The coordinate can be an absolute coordinate and it will be
/// normalized to relative coordinates.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct RegionCoord(u16);

impl RegionCoord {
	/// Create a new RegionCoord.
	/// The x and z will be mathematically transformed into relative coordinates.
	/// So if the coordinate given to `new()` is `(32, 32)`, the result will be
	/// `(0, 0)`.
	pub fn new(x: u16, z: u16) -> Self {
		let xmod = x & 31;
		let zmod = z & 31;
		Self(xmod | zmod.overflowing_shl(5).0)
	}

	pub fn index(&self) -> usize {
		self.0 as usize
	}

	pub fn x(&self) -> i32 {
		(self.0 & 31) as i32
	}

	pub fn z(&self) -> i32 {
		(self.0.overflowing_shr(5).0 & 31) as i32
	}

	pub fn tuple<T>(self) -> (T, T) // <- they are very sad.
	where
	(T, T): From<Self> {
		self.into()
	}

	/// Get a [SeekFrom] value that can be used to seek to the location where
	/// this chunk's sector offset is stored in the sector offset table.
	pub fn sector_table_offset(&self) -> SeekFrom {
		SeekFrom::Start(self.0 as u64 * 4)
	}

	/// Get a [SeekFrom] value that can be used to seek to the location where
	/// this chunk's timestamp is stored in the timestamp table.
	pub fn timestamp_table_offset(&self) -> SeekFrom {
		SeekFrom::Start(self.0 as u64 * 4 + 4096)
	}
}

macro_rules! __regioncoord_impl {
	($type:ty) => {

		impl From<($type, $type)> for RegionCoord {
			fn from(value: ($type, $type)) -> Self {
				Self::new(value.0 as u16, value.1 as u16)
			}
		}

		impl From<$type> for RegionCoord {
			fn from(value: $type) -> Self {
				Self(value as u16)
			}
		}

		impl From<RegionCoord> for ($type, $type) {
			fn from(value: RegionCoord) -> Self {
				(value.x() as $type, value.z() as $type)
			}
		}

		impl From<RegionCoord> for $type {
			fn from(value: RegionCoord) -> Self {
				value.0 as $type
			}
		}
	};
}

for_each_int_type!(__regioncoord_impl);

impl<T: Into<RegionCoord> + Copy> From<&T> for RegionCoord {
    fn from(value: &T) -> Self {
		T::into(*value)
    }
}

impl std::fmt::Display for RegionCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x(), self.z())
    }
}