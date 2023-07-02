//! The grid module is used for mathematics related to grids
//! like snapping.

pub trait Grid {
	type PointType;
	type IndexType;

	fn offset<RP: From<Self::PointType>>(&self) -> RP;
	fn cell_size<RP: From<Self::PointType>>(&self) -> RP;

	fn snap<P: Into<Self::PointType>, RP: From<Self::PointType>>(&self, pos: P) -> RP;
	fn index<P: Into<Self::PointType>, RI: From<Self::IndexType>>(&self, pos: P) -> RI;
	fn cell_min<I: Into<Self::IndexType>, RP: From<Self::PointType>>(&self, index: I) -> RP;
	fn cell<I: Into<Self::IndexType>, RP: From<Self::PointType>, R: From<[RP; 2]>>(&self, index: I) -> R;
}

pub trait Grid2Index<T> {
	fn x(&self) -> T;
	fn y(&self) -> T;
	fn xy(&self) -> (T, T);
	fn create(x: T, y: T) -> Self;
}

pub trait Grid3Index<T> {
	fn x(&self) -> T;
	fn y(&self) -> T;
	fn z(&self) -> T;
	fn xyz(&self) -> (T, T, T);
	fn create(x: T, y: T, z: T) -> Self;
}

pub trait Grid2Point<T> {
	type IndexType;
	fn x(&self) -> T;
	fn y(&self) -> T;
	fn xy(&self) -> (T, T);
	fn create(x: T, y: T) -> Self;
	fn index_point(x: T, y: T) -> Self::IndexType;
	fn from_index(index: Self::IndexType) -> Self;
}

pub trait Grid3Point<T> {
	type IndexType;
	fn x(&self) -> T;
	fn y(&self) -> T;
	fn z(&self) -> T;
	fn xyz(&self) -> (T, T, T);
	fn create(x: T, y: T, z: T) -> Self;
	fn index_point(x: T, y: T, z: T) -> Self::IndexType;
	fn from_index(index: Self::IndexType) -> Self;
}

impl Grid2Index<i32> for (i32, i32) {
	#[inline(always)]
	fn x(&self) -> i32 {
		self.0
	}

	#[inline(always)]
	fn y(&self) -> i32 {
		self.1
	}

	#[inline(always)]
	fn xy(&self) -> (i32, i32) {
		(self.0, self.1)
	}

	#[inline(always)]
	fn create(x: i32, y: i32) -> Self {
		(x, y)
	}
}

impl Grid3Index<i32> for (i32, i32, i32) {
	#[inline(always)]
    fn x(&self) -> i32 {
        self.0
    }

	#[inline(always)]
    fn y(&self) -> i32 {
        self.1
    }

	#[inline(always)]
    fn z(&self) -> i32 {
        self.2
    }

	#[inline(always)]
    fn xyz(&self) -> (i32, i32, i32) {
        (self.0, self.1, self.2)
    }

	#[inline(always)]
    fn create(x: i32, y: i32, z: i32) -> Self {
        (x, y, z)
    }
}

impl Grid2Point<f32> for (f32, f32) {
	type IndexType = (i32, i32);
	#[inline(always)]
	fn x(&self) -> f32 {
		self.0
	}

	#[inline(always)]
	fn y(&self) -> f32 {
		self.1
	}

	#[inline(always)]
	fn xy(&self) -> (f32, f32) {
		(self.0, self.1)
	}

	#[inline(always)]
	fn create(x: f32, y: f32) -> Self {
		(x, y)
	}

	#[inline(always)]
	fn index_point(x: f32, y: f32) -> Self::IndexType {
		(x as i32, y as i32)
	}

	#[inline(always)]
	fn from_index(index: Self::IndexType) -> Self {
		Self::create(index.0 as f32, index.1 as f32)
	}
}

impl Grid3Point<f32> for (f32, f32, f32) {
    type IndexType = (i32, i32, i32);

	#[inline(always)]
    fn x(&self) -> f32 {
        self.0
    }

	#[inline(always)]
    fn y(&self) -> f32 {
        self.1
    }

	#[inline(always)]
    fn z(&self) -> f32 {
        self.2
    }

	#[inline(always)]
    fn xyz(&self) -> (f32, f32, f32) {
        (self.0, self.1, self.2)
    }

	#[inline(always)]
    fn create(x: f32, y: f32, z: f32) -> Self {
        (x, y, z)
    }

	#[inline(always)]
    fn index_point(x: f32, y: f32, z: f32) -> Self::IndexType {
        (x as i32, y as i32, z as i32)
    }

	#[inline(always)]
    fn from_index(index: Self::IndexType) -> Self {
        (index.0 as f32, index.1 as f32, index.2 as f32)
    }
}

pub struct BasicGrid<P> {
	offset: P,
	cell_size: P,
}

impl<P> BasicGrid<P> {
	#[inline(always)]
	pub fn new<PP1: Into<P>, PP2: Into<P>>(offset: PP1, cell_size: PP2) -> Self {
		Self { offset: offset.into(), cell_size: cell_size.into() }
	}
}

impl BasicGrid<(f32, f32)> {
	pub fn square(size: f32) -> Self {
		Self::new((0.0, 0.0), (size, size))
	}

	pub fn offset_square<P: Into<(f32, f32)>>(offset: P, size: f32) -> Self {
		Self::new(offset, (size, size))
	}
}

impl BasicGrid<(f32, f32, f32)> {
	pub fn cubic(size: f32) -> Self {
		Self::new((0.0, 0.0, 0.0), (size, size, size))
	}

	pub fn offset_cubic<P: Into<(f32, f32, f32)>>(offset: P, size: f32) -> Self {
		Self::new(offset, (size, size, size))
	}
}

impl Grid for BasicGrid<(f32, f32)> {
	type IndexType = (i32, i32);
	type PointType = (f32, f32);

	#[inline(always)]
	fn offset<RP: From<Self::PointType>>(&self) -> RP {
		RP::from(self.offset)
	}

	#[inline(always)]
	fn cell_size<RP: From<Self::PointType>>(&self) -> RP {
		RP::from(self.cell_size)
	}

	fn snap<P: Into<Self::PointType>, RP: From<Self::PointType>>(&self, pos: P) -> RP {
		let pos = pos.into();
		// In order to snap, first you must subtract the offset
		// Then divrem width/height and subtract those values from pos (which has had offset subtracted from it)
		// Then add offset back to the result
		let norm_x = pos.x() - self.offset.x();
		let norm_y = pos.y() - self.offset.y();
		let norm_snap_x = (norm_x / self.cell_size.x()).floor();
		let norm_snap_y = (norm_y / self.cell_size.y()).floor();
		let snap_mul_x = norm_snap_x * self.cell_size.x();
		let snap_mul_y = norm_snap_y * self.cell_size.y();
		let result_x = snap_mul_x + self.offset.x();
		let result_y = snap_mul_y + self.offset.y();
		RP::from((result_x, result_y))
	}

	fn index<P: Into<Self::PointType>, RI: From<Self::IndexType>>(&self, pos: P) -> RI {
		let pos = pos.into();
		let norm_x = pos.x() - self.offset.x();
		let norm_y = pos.y() - self.offset.y();
		let ix = (norm_x / self.cell_size.x()).floor();
		let iy = (norm_y / self.cell_size.y()).floor();
		RI::from((ix as i32, iy as i32))
	}

	fn cell_min<I: Into<Self::IndexType>, RP: From<Self::PointType>>(&self, index: I) -> RP {
		let index = index.into();
		let index_point = Self::PointType::from_index(index);
		let new_x = self.cell_size.x() * index_point.x() + self.offset.x();
		let new_y = self.cell_size.y() * index_point.y() + self.offset.y();
		RP::from((new_x, new_y))
	}

	fn cell<I: Into<Self::IndexType>, RP: From<Self::PointType>, R: From<[RP; 2]>>(&self, index: I) -> R {
		let min: Self::PointType = self.cell_min(index);
		let max = (
			min.0 + self.cell_size.x(),
			min.1 + self.cell_size.y()
		);
		R::from([RP::from(min), RP::from(max)])
	}
}

impl Grid for BasicGrid<(f32, f32, f32)> {
    type IndexType = (i32, i32, i32);
	type PointType = (f32, f32, f32);

	#[inline(always)]
	fn offset<RP: From<Self::PointType>>(&self) -> RP {
		RP::from(self.offset)
	}

	#[inline(always)]
	fn cell_size<RP: From<Self::PointType>>(&self) -> RP {
		RP::from(self.cell_size)
	}

    fn snap<P: Into<Self::PointType>, RP: From<Self::PointType>>(&self, pos: P) -> RP {
		let pos = pos.into();
        let norm_x = pos.x() - self.offset.x();
		let norm_y = pos.y() - self.offset.y();
		let norm_z = pos.z() - self.offset.z();
		let norm_snap_x = (norm_x / self.cell_size.x()).floor();
		let norm_snap_y = (norm_y / self.cell_size.y()).floor();
		let norm_snap_z = (norm_z / self.cell_size.z()).floor();
		let snap_mul_x = norm_snap_x * self.cell_size.x();
		let snap_mul_y = norm_snap_y * self.cell_size.y();
		let snap_mul_z = norm_snap_z * self.cell_size.z();
		let result_x = snap_mul_x + self.offset.x();
		let result_y = snap_mul_y + self.offset.y();
		let result_z = snap_mul_z + self.offset.z();
		RP::from((result_x, result_y, result_z))
    }

    fn index<P: Into<Self::PointType>, RI: From<Self::IndexType>>(&self, pos: P) -> RI {
		let pos = pos.into();
        let norm_x = pos.x() - self.offset.x();
		let norm_y = pos.y() - self.offset.y();
		let norm_z = pos.z() - self.offset.z();
		let ix = (norm_x / self.cell_size.x()).floor();
		let iy = (norm_y / self.cell_size.y()).floor();
		let iz = (norm_z / self.cell_size.z()).floor();
		RI::from((ix as i32, iy as i32, iz as i32))
    }

    fn cell_min<I: Into<Self::IndexType>, RP: From<Self::PointType>>(&self, index: I) -> RP {
		let index = index.into();
        let index_point = Self::PointType::from_index(index);
		let new_x = self.cell_size.x() * index_point.x() + self.offset.x();
		let new_y = self.cell_size.y() * index_point.y() + self.offset.y();
		let new_z = self.cell_size.z() * index_point.z() + self.offset.z();
		RP::from((new_x, new_y, new_z))
    }

	fn cell<I: Into<Self::IndexType>, RP: From<Self::PointType>, R: From<[RP; 2]>>(&self, index: I) -> R {
		let min: Self::PointType = self.cell_min(index);
		let max = (
			min.0 + self.cell_size.x(),
			min.1 + self.cell_size.y(),
			min.2 + self.cell_size.z()
		);
		R::from([RP::from(min), RP::from(max)])
	}
}

pub type BasicGrid2 = BasicGrid<(f32, f32)>;
pub type BasicGrid3 = BasicGrid<(f32, f32, f32)>;