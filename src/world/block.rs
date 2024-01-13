use crate::math::coord::*;

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
}

impl Into<Coord3> for CubeDirection {
    fn into(self) -> Coord3 {
        Coord3::from(self.coord())
    }
}