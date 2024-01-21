use crate::math::coord::*;
use glam::i64::I64Vec3;

pub enum CubeDirection {
	East,	// +X
	West,	// -X
	South,	// +Z
	North,	// -Z
	Up,		// +Y
	Down,	// -Y
}

impl CubeDirection {
	#[inline(always)]
	pub fn coord(self) -> (i64, i64, i64) {
		match self {
			CubeDirection::Up => (0, 1, 0),
			CubeDirection::Down => (0, -1, 0),
			CubeDirection::North => (0, 0, -1),
			CubeDirection::West => (-1, 0, 0),
			CubeDirection::South => (0, 0, 1),
			CubeDirection::East => (1, 0, 0),
		}
	}

	#[inline(always)]
	pub fn i64vec3(self) -> I64Vec3 {
		self.into()
	}
}

impl From<Cardinal> for CubeDirection {
	#[inline(always)]
	fn from(value: Cardinal) -> Self {
		match value {
			Cardinal::East => Self::East,
			Cardinal::West => Self::West,
			Cardinal::South => Self::South,
			Cardinal::North => Self::North,
		}
	}
}

impl Into<I64Vec3> for CubeDirection {
	#[inline(always)]
	fn into(self) -> I64Vec3 {
		I64Vec3::from(self.coord())
	}
}

impl Into<Coord3> for CubeDirection {
	#[inline(always)]
	fn into(self) -> Coord3 {
		Coord3::from(self.coord())
	}
}

impl Into<(i64, i64, i64)> for CubeDirection {
	#[inline(always)]
	fn into(self) -> (i64, i64, i64) {
		self.coord()
	}
}

#[repr(u8)]
pub enum HeightmapFlag {
	MotionBlocking = 1,
	MotionBlockingNoLeaves = 2,
	OceanFloor = 4,
	WorldSurface = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeightmapFlags(u8);

impl HeightmapFlags {
	pub fn check(self, flag: HeightmapFlag) -> bool {
		let rhs = flag as u8;
		let lhs = self.0;
		(lhs & rhs) == rhs
	}
}

impl std::ops::BitOr<HeightmapFlag> for HeightmapFlag {
	type Output = HeightmapFlags;

	fn bitor(self, rhs: HeightmapFlag) -> Self::Output {
		let lhs = self as u8;
		let rhs = rhs as u8;
		HeightmapFlags(lhs | rhs)
	}
}

impl std::ops::BitOr<HeightmapFlag> for HeightmapFlags {
	type Output = HeightmapFlags;

	fn bitor(self, rhs: HeightmapFlag) -> Self::Output {
		let lhs = self.0;
		let rhs = rhs as u8;
		HeightmapFlags(lhs | rhs)
	}
}

impl std::ops::BitOr<HeightmapFlags> for HeightmapFlags {
	type Output = HeightmapFlags;

	fn bitor(self, rhs: HeightmapFlags) -> Self::Output {
		let lhs = self.0;
		let rhs = rhs.0;
		HeightmapFlags(lhs | rhs)
	}
}

impl std::fmt::Display for HeightmapFlag {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			HeightmapFlag::MotionBlocking => write!(f, "HeightmapFlag::MotionBlocking"),
			HeightmapFlag::MotionBlockingNoLeaves => write!(f, "HeightmapFlag::MotionBlockingNoLeaves"),
			HeightmapFlag::OceanFloor => write!(f, "HeightmapFlag::OceanFloor"),
			HeightmapFlag::WorldSurface => write!(f, "HeightmapFlag::WorldSurface"),
		}
	}
}

impl std::fmt::Display for HeightmapFlags {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "HeightmapFlags(")?;
		let mut add_separator = false;
		// let mut flags = Vec::new();
		if self.check(HeightmapFlag::MotionBlocking) {
			// flags.push(HeightmapFlag::MotionBlocking);
			write!(f, "{}", HeightmapFlag::MotionBlocking)?;
			add_separator = true;
		}
		if self.check(HeightmapFlag::MotionBlockingNoLeaves) {
			// flags.push(HeightmapFlag::MotionBlockingNoLeaves);
			if add_separator {
				write!(f, " | ")?;
			}
			write!(f, "{}", HeightmapFlag::MotionBlockingNoLeaves)?;
			add_separator = true;
		}
		if self.check(HeightmapFlag::OceanFloor) {
			// flags.push(HeightmapFlag::OceanFloor);
			if add_separator {
				write!(f, " | ")?;
			}
			write!(f, "{}", HeightmapFlag::OceanFloor)?;
			add_separator = true;
		}
		if self.check(HeightmapFlag::WorldSurface) {
			// flags.push(HeightmapFlag::WorldSurface);
			if add_separator {
				write!(f, " | ")?;
			}
			write!(f, "{}", HeightmapFlag::WorldSurface)?;
		}
		write!(f, ")")
	}
}

pub struct BlockInfo {
	heightmap_flags: HeightmapFlags,
	mesh_data: (),
	
}