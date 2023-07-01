//! The grid module is used for mathematics related to grids
//! like snapping.

pub trait Grid {
	type PointType;
	type IndexType;

	fn offset(&self) -> Self::PointType;
	fn cell_size(&self) -> Self::PointType;

	fn snap(&self, pos: Self::PointType) -> Self::PointType;
	fn index(&self, pos: Self::PointType) -> Self::IndexType;
	fn cell(&self, index: Self::IndexType) -> Self::PointType;
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
	pub fn new(offset: P, cell_size: P) -> Self {
		Self { offset, cell_size }
	}
}

impl BasicGrid<(f32, f32)> {
	pub fn square(size: f32) -> Self {
		Self::new((0.0, 0.0), (size, size))
	}

	pub fn offset_square(offset: (f32, f32), size: f32) -> Self {
		Self::new(offset, (size, size))
	}
}

impl BasicGrid<(f32, f32, f32)> {
	pub fn cubic(size: f32) -> Self {
		Self::new((0.0, 0.0, 0.0), (size, size, size))
	}

	pub fn offset_cubic(offset: (f32, f32, f32), size: f32) -> Self {
		Self::new(offset, (size, size, size))
	}
}

impl Grid for BasicGrid<(f32, f32)> {
	type IndexType = (i32, i32);
	type PointType = (f32, f32);

	#[inline(always)]
	fn offset(&self) -> Self::PointType {
		self.offset
	}

	#[inline(always)]
	fn cell_size(&self) -> Self::PointType {
		self.cell_size
	}

	fn snap(&self, pos: Self::PointType) -> Self::PointType {
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
		Self::PointType::create(result_x, result_y)
	}

	fn index(&self, pos: Self::PointType) -> Self::IndexType {
		let norm_x = pos.x() - self.offset.x();
		let norm_y = pos.y() - self.offset.y();
		let ix = (norm_x / self.cell_size.x()).floor();
		let iy = (norm_y / self.cell_size.y()).floor();
		Self::PointType::index_point(ix, iy)
	}

	fn cell(&self, index: Self::IndexType) -> Self::PointType {
		let index_point = Self::PointType::from_index(index);
		let new_x = self.cell_size.x() * index_point.x() + self.offset.x();
		let new_y = self.cell_size.y() * index_point.y() + self.offset.y();
		Self::PointType::create(new_x, new_y)
	}
}

impl Grid for BasicGrid<(f32, f32, f32)> {
    type IndexType = (i32, i32, i32);
	type PointType = (f32, f32, f32);
	#[inline(always)]
	fn offset(&self) -> Self::PointType {
		self.offset
	}

	#[inline(always)]
	fn cell_size(&self) -> Self::PointType {
		self.cell_size
	}

    fn snap(&self, pos: Self::PointType) -> Self::PointType {
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
		Self::PointType::create(result_x, result_y, result_z)
    }

    fn index(&self, pos: Self::PointType) -> Self::IndexType {
        let norm_x = pos.x() - self.offset.x();
		let norm_y = pos.y() - self.offset.y();
		let norm_z = pos.z() - self.offset.z();
		let ix = (norm_x / self.cell_size.x()).floor();
		let iy = (norm_y / self.cell_size.y()).floor();
		let iz = (norm_z / self.cell_size.z()).floor();
		Self::PointType::index_point(ix, iy, iz)
    }

    fn cell(&self, index: Self::IndexType) -> Self::PointType {
        let index_point = Self::PointType::from_index(index);
		let new_x = self.cell_size.x() * index_point.x() + self.offset.x();
		let new_y = self.cell_size.y() * index_point.y() + self.offset.y();
		let new_z = self.cell_size.z() * index_point.z() + self.offset.z();
		Self::PointType::create(new_x, new_y, new_z)
    }
}

pub type BasicGrid2 = BasicGrid<(f32, f32)>;
pub type BasicGrid3 = BasicGrid<(f32, f32, f32)>;