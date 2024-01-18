use glam::I64Vec3;

use crate::world::block::CubeDirection;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Dimension {
	Overworld,
	Nether,
	TheEnd,
	Other(u32),
}

pub enum Cardinal {
	East,	// +X
	West,	// -X
	South,	// +Z
	North,	// -Z
}

impl Cardinal {
	#[inline(always)]
	pub fn coord(self) -> (i64, i64) {
		match self {
			Cardinal::East => (1, 0),
			Cardinal::West => (-1, 0),
			Cardinal::South => (0, 1),
			Cardinal::North => (0, -1),
		}
	}
}

impl Dimension {
	#[inline(always)]
	pub fn blockcoord(self, x: i64, y: i64, z: i64) -> BlockCoord {
		BlockCoord::new(x, y, z, self)
	}

	#[inline(always)]
	pub fn worldcoord(self, x: i64, z: i64) -> WorldCoord {
		WorldCoord::new(x, z, self)
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Coord2 {
	pub x: i64,
	pub y: i64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Coord3 {
	pub x: i64,
	pub y: i64,
	pub z: i64,
}

impl Coord2 {
	#[inline(always)]
	pub fn new(x: i64, y: i64) -> Self {
		Self {
			x,
			y
		}
	}

	#[inline(always)]
	pub fn map<F: Fn(i64)->i64>(self, map: F) -> Self {
		Self {
			x: map(self.x),
			y: map(self.y),
		}
	}

	#[inline(always)]
	pub fn xy(self) -> (i64, i64) {
		(self.x, self.y)
	}

	pub fn worldcoord(self, dimension: Dimension) -> WorldCoord {
		WorldCoord::new(
			self.x,
			self.y,
			dimension
		)
	}

	pub fn overworld(self) -> WorldCoord {
		self.worldcoord(Dimension::Overworld)
	}

	pub fn nether(self) -> WorldCoord {
		self.worldcoord(Dimension::Nether)
	}
}

impl From<(i64, i64)> for Coord2 {
	#[inline(always)]
	fn from(value: (i64, i64)) -> Self {
		Coord2::new(value.0, value.1)
	}
}

impl From<WorldCoord> for Coord2 {
	fn from(value: WorldCoord) -> Self {
		Coord2::new(
			value.x,
			value.z
		)
	}
}

impl Into<(i64, i64)> for Coord2 {
	#[inline(always)]
	fn into(self) -> (i64, i64) {
		(self.x, self.y)
	}
}

impl Coord3 {
	#[inline(always)]
	pub fn new(x: i64, y: i64, z: i64) -> Self {
		Self {
			x,
			y,
			z
		}
	}

	#[inline(always)]
	pub fn map<F: Fn(i64)->i64>(self, map: F) -> Self {
		Self {
			x: map(self.x),
			y: map(self.y),
			z: map(self.z)
		}
	}

	#[inline(always)]
	pub fn xyz(self) -> (i64, i64, i64) {
		(
			self.x,
			self.y,
			self.z
		)
	}

	#[inline(always)]
	pub fn blockcoord(self, dimension: Dimension) -> BlockCoord {
		BlockCoord::new(self.x, self.y, self.z, dimension)
	}

	#[inline(always)]
	pub fn overworld(self) -> BlockCoord {
		self.blockcoord(Dimension::Overworld)
	}

	#[inline(always)]
	pub fn nether(self) -> BlockCoord {
		self.blockcoord(Dimension::Nether)
	}
}

impl From<(i64, i64, i64)> for Coord3 {
	fn from(value: (i64, i64, i64)) -> Self {
		Coord3::new(
			value.0,
			value.1,
			value.2
		)
	}
}

impl From<BlockCoord> for Coord3 {
	fn from(value: BlockCoord) -> Self {
		Coord3::new(
			value.x,
			value.y,
			value.z
		)
	}
}

impl Into<(i64, i64, i64)> for Coord3 {
	fn into(self) -> (i64, i64, i64) {
		(self.x, self.y, self.z)
	}
}

impl Default for Dimension {
	fn default() -> Self {
		Dimension::Overworld
	}
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub struct WorldCoord {
	pub x: i64,
	pub z: i64,
	pub dimension: Dimension,
}

impl WorldCoord {
	#[inline(always)]
	pub fn new(x: i64, z: i64, dimension: Dimension) -> Self {
		Self {
			x,
			z,
			dimension
		}
	}

	#[inline(always)]
	pub fn xz(self) -> (i64, i64) {
		(
			self.x,
			self.z
		)
	}

	#[inline(always)]
	pub fn overworld(x: i64, z: i64) -> Self {
		Self::new(x, z, Dimension::Overworld)
	}

	#[inline(always)]
	pub fn nether(x: i64, z: i64) -> Self {
		Self::new(x, z, Dimension::Nether)
	}

	#[inline(always)]
	pub fn the_end(x: i64, z: i64) -> Self {
		Self::new(x, z, Dimension::TheEnd)
	}

	/// Converts a chunk coordinate into a region coordinate.
	#[inline(always)]
	pub fn region_coord(self) -> Self {
		Self {
			x: self.x.div_euclid(32),
			z: self.z.div_euclid(32),
			dimension: self.dimension,
		}
	}

	#[inline(always)]
	pub fn neighbor(self, direction: Cardinal) -> Self {
		self + direction
	}
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub struct BlockCoord {
	pub x: i64,
	pub y: i64,
	pub z: i64,
	pub dimension: Dimension,
}

impl BlockCoord {
	#[inline(always)]
	pub fn new(x: i64, y: i64, z: i64, dimension: Dimension) -> Self {
		Self {
			x,
			y,
			z,
			dimension,
		}
	}

	#[inline(always)]
	pub fn xyz(self) -> (i64, i64, i64) {
		(
			self.x,
			self.y,
			self.z,
		)
	}

	#[inline(always)]
	pub fn overworld(x: i64, y: i64, z: i64) -> Self {
		Self::new(x, y, z, Dimension::Overworld)
	}

	#[inline(always)]
	pub fn nether(x: i64, y: i64, z: i64) -> Self {
		Self::new(x, y, z, Dimension::Nether)
	}

	#[inline(always)]
	pub fn the_end(x: i64, y: i64, z: i64) -> Self {
		Self::new(x, y, z, Dimension::TheEnd)
	}

	#[inline(always)]
	pub fn subchunk_coord(self) -> Self {
		BlockCoord {
			x: self.x.rem_euclid(16),
			y: self.y.rem_euclid(16),
			z: self.z.rem_euclid(16),
			dimension: self.dimension,
		}
	}

	#[inline(always)]
	pub fn chunk_coord(self) -> WorldCoord {
		WorldCoord {
			x: self.x.div_euclid(16),
			z: self.z.div_euclid(16),
			dimension: self.dimension,
		}
	}

	#[inline(always)]
	pub fn region_coord(self) -> WorldCoord {
		WorldCoord {
			x: self.x.div_euclid(512),
			z: self.z.div_euclid(512),
			dimension: self.dimension,
		}
	}

	#[inline(always)]
	pub fn neighbor(self, direction: CubeDirection) -> Self {
		let (x,y,z) = direction.coord();
		Self::new(self.x + x, self.y + y, self.z + z, self.dimension)
	}
}

impl std::ops::Add<(i64, i64)> for WorldCoord {
	type Output = WorldCoord;

	#[inline(always)]
	fn add(self, rhs: (i64, i64)) -> Self::Output {
		let (x,z) = rhs;
		Self::new(self.x + x, self.z + z, self.dimension)
	}
}

impl std::ops::Sub<(i64,i64)> for WorldCoord {
	type Output = WorldCoord;

	#[inline(always)]
	fn sub(self, rhs: (i64,i64)) -> Self::Output {
		let (x,z) = rhs;
		Self::new(self.x - x, self.z - z, self.dimension)
	}
}

impl std::ops::Add<Cardinal> for WorldCoord {
	type Output = WorldCoord;

	#[inline(always)]
	fn add(self, rhs: Cardinal) -> Self::Output {
		let (x,z) = rhs.coord();
		Self::new(self.x + x, self.z + z, self.dimension)
	}
}

impl std::ops::Sub<Cardinal> for WorldCoord {
	type Output = WorldCoord;

	#[inline(always)]
	fn sub(self, rhs: Cardinal) -> Self::Output {
		let (x,z) = rhs.coord();
		Self::new(self.x - x, self.z - z, self.dimension)
	}
}

impl std::ops::Add<(i64, i64, i64)> for BlockCoord {
	type Output = BlockCoord;

	#[inline(always)]
	fn add(self, rhs: (i64, i64, i64)) -> Self::Output {
		let (x,y,z) = rhs;
		Self::new(self.x + x, self.y + y, self.z + z, self.dimension)
	}
}

impl std::ops::Sub<(i64,i64,i64)> for BlockCoord {
	type Output = BlockCoord;

	#[inline(always)]
	fn sub(self, rhs: (i64,i64,i64)) -> Self::Output {
		let (x,y,z) = rhs;
		Self::new(self.x - x, self.y - y, self.z - z, self.dimension)
	}
}

impl std::ops::Add<Coord3> for BlockCoord {
	type Output = BlockCoord;

	#[inline(always)]
	fn add(self, rhs: Coord3) -> Self::Output {
		let (x,y,z) = rhs.xyz();
		Self::new(self.x + x, self.y + y, self.z + z, self.dimension)
	}
}

impl std::ops::Sub<Coord3> for BlockCoord {
	type Output = BlockCoord;

	#[inline(always)]
	fn sub(self, rhs: Coord3) -> Self::Output {
		let (x,y,z) = rhs.xyz();
		Self::new(self.x - x, self.y - y, self.z - z, self.dimension)
	}
}

impl std::ops::Add<I64Vec3> for BlockCoord {
	type Output = BlockCoord;

	#[inline(always)]
	fn add(self, rhs: I64Vec3) -> Self::Output {
		let (x,y,z):(i64,i64,i64) = rhs.into();
		Self::new(self.x + x, self.y + y, self.z + z, self.dimension)
	}
}

impl std::ops::Sub<I64Vec3> for BlockCoord {
	type Output = BlockCoord;

	#[inline(always)]
	fn sub(self, rhs: I64Vec3) -> Self::Output {
		let (x,y,z):(i64,i64,i64) = rhs.into();
		Self::new(self.x - x, self.y - y, self.z - z, self.dimension)
	}
}

impl std::ops::Add<CubeDirection> for BlockCoord {
	type Output = BlockCoord;

	#[inline(always)]
	fn add(self, rhs: CubeDirection) -> Self::Output {
		let (x,y,z) = rhs.coord();
		Self::new(self.x + x, self.y + y, self.z + z, self.dimension)
	}
}

impl std::ops::Sub<CubeDirection> for BlockCoord {
	type Output = BlockCoord;

	#[inline(always)]
	fn sub(self, rhs: CubeDirection) -> Self::Output {
		let (x,y,z) = rhs.coord();
		Self::new(self.x - x, self.y - y, self.z - z, self.dimension)
	}
}