use std::ops::Not;

use crate::math::coord::*;
use glam::i64::I64Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CubeDirection {
    East,	// +X
    West,	// -X
    South,	// +Z
    North,	// -Z
    Up,		// +Y
    Down,	// -Y
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CubeFace {
    East = 1,
    West = 2,
    South = 4,
    North = 8,
    Top = 16,
    Bottom = 32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CubeFaces(u8);

impl CubeFaces {
    #[inline(always)]
    pub fn check(self, face: CubeFace) -> bool {
        let rhs = face as u8;
        (self.0 & rhs) == rhs
    }

    #[inline(always)]
    pub fn apply<T: Into<CubeFaces>>(&mut self, faces: T) {
        let faces: CubeFaces = faces.into();
        self.0 = self.0 | faces.0;
    }

    #[inline(always)]
    pub fn remove<T: Into<CubeFaces>>(&mut self, faces: T) {
        let faces: CubeFaces = faces.into();
        self.0 = self.0 & faces.0.not();
    }
}

// #[test]
// fn qtest() {
// 	let mut faces = CubeFaces::default();
// 	faces.apply(CubeFace::Top | CubeFace::West);
// 	faces.apply(CubeFace::North);
// 	faces.remove(CubeFace::Top);
// 	macro_rules! check {
// 		($($face:ident),+) => {
// 			$(
// 				println!("{:>6}: {}", stringify!($face), faces.check(CubeFace::$face));
// 			)+
// 		};
// 		() => {
// 			check!(East, West, South, North, Top, Bottom);
// 		};
// 	}
// 	check!();
// }

impl From<CubeFace> for CubeFaces {
    #[inline(always)]
    fn from(value: CubeFace) -> Self {
        CubeFaces(value as u8)
    }
}

impl std::ops::BitOr<CubeFace> for CubeFace {
    type Output = CubeFaces;

    #[inline(always)]
    fn bitor(self, rhs: CubeFace) -> Self::Output {
        let lhs = self as u8;
        let rhs = rhs as u8;
        CubeFaces(lhs | rhs)
    }
}

impl std::ops::BitOr<CubeFace> for CubeFaces {
    type Output = CubeFaces;

    #[inline(always)]
    fn bitor(self, rhs: CubeFace) -> Self::Output {
        let lhs = self.0;
        let rhs = rhs as u8;
        CubeFaces(lhs | rhs)
    }
}

impl std::ops::BitOr<CubeFaces> for CubeFaces {
    type Output = CubeFaces;

    #[inline(always)]
    fn bitor(self, rhs: CubeFaces) -> Self::Output {
        let lhs = self.0;
        let rhs = rhs.0;
        CubeFaces(lhs | rhs)
    }
}

impl From<CubeDirection> for CubeFace {
    #[inline(always)]
    fn from(value: CubeDirection) -> Self {
        match value {
            CubeDirection::East => CubeFace::East,
            CubeDirection::West => CubeFace::West,
            CubeDirection::South => CubeFace::South,
            CubeDirection::North => CubeFace::North,
            CubeDirection::Up => CubeFace::Top,
            CubeDirection::Down => CubeFace::Bottom,
        }
    }
}

impl From<CubeFace> for CubeDirection {
    #[inline(always)]
    fn from(value: CubeFace) -> Self {
        match value {
            CubeFace::East => CubeDirection::East,
            CubeFace::West => CubeDirection::West,
            CubeFace::South => CubeDirection::South,
            CubeFace::North => CubeDirection::North,
            CubeFace::Top => CubeDirection::Up,
            CubeFace::Bottom => CubeDirection::Down,
        }
    }
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
    #[inline(always)]
    pub fn check<T: Into<HeightmapFlags>>(self, flags: T) -> bool {
        let flags: HeightmapFlags = flags.into();
        (self.0 & flags.0) == flags.0
    }
}

impl From<HeightmapFlag> for HeightmapFlags {
    #[inline(always)]
    fn from(value: HeightmapFlag) -> Self {
        HeightmapFlags(value as u8)
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

// What info do I need for a Block?
// I guess that depends on what the implementation needs
// to be able to do.
// resource_id is the id for the block. Example: "minecraft:bedrock"
// heightmap_flags stores information about which heightmaps
// are affected by this block.
// I was thinking of also storing mesh data, but I might save
// that for the application implementation.
#[allow(unused)]
pub struct BlockInfo {
    resource_id: String,
    heightmap_flags: HeightmapFlags,
    mesh_data: (),
}

impl BlockInfo {
    pub fn new<S: AsRef<str>>(resource_id: S, heightmap_flags: HeightmapFlags) -> Self {
        Self {
            resource_id: resource_id.as_ref().to_owned(),
            heightmap_flags,
            mesh_data: (),
        }
    }
}