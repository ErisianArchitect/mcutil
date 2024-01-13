use crate::math::coord::*;
use glam::i64::I64Vec3;

pub enum CubeDirection {
	Up,		// +Y
	Down,	// -Y
	East,	// +X
	West,	// -X
	South,	// +Z
	North,	// -Z
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