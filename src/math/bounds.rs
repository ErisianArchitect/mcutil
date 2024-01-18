#![allow(unused)]
use std::{borrow::Borrow, num::NonZeroU64};

use super::coord::*;
use glam::{i64::I64Vec2, i64vec2, I64Vec3, i64vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds2 {
	pub min: I64Vec2,
	pub max: I64Vec2,
}

impl Bounds2 {
	pub fn new<T: Into<I64Vec2>>(a: T, b: T) -> Self {
		let a: I64Vec2 = a.into();
		let b: I64Vec2 = b.into();
		let min_x = a.x.min(b.x);
		let min_y = a.y.min(b.y);
		let max_x = a.x.max(b.x);
		let max_y = a.y.max(b.y);
		Self {
			min: i64vec2(min_x, min_y),
			max: i64vec2(max_x, max_y)
		}
	}

	/// A Radius of 1 will result in a 3x3 Bounds2.
	pub fn radius<T: Into<I64Vec2>>(center: T, radius: u64) -> Self {
		let center: I64Vec2 = center.into();
		let r = radius as i64;
		let min_x = center.x - r;
		let min_y = center.y - r;
		let max_x = center.x + r;
		let max_y = center.y + r;
		Self {
			min: i64vec2(min_x, min_y),
			max: i64vec2(max_x, max_y)
		}
	}

	pub fn min<R: From<I64Vec2>>(&self) -> R {
		R::from(self.min)
	}

	pub fn max<R: From<I64Vec2>>(&self) -> R {
		R::from(self.max)
	}

	pub fn size<R: From<I64Vec2>>(&self) -> R {
		let x = self.max.x - self.min.x + 1;
		let y = self.max.y - self.min.y + 1;
		R::from(i64vec2(x, y))
	}

	pub fn for_each<F: FnMut(I64Vec2) -> ()>(&self, mut f: F) {
		(self.min.y..self.max.y).for_each(|y| {
			(self.min.x..self.max.x).for_each(|x| {
				f(i64vec2(x, y));
			})
		})
	}
}

impl<T: Into<I64Vec2>> From<(T, T)> for Bounds2 {
	fn from(value: (T, T)) -> Self {
		Self::new(value.0, value.1)
	}
}

impl<T: Into<I64Vec2> + Copy> From<[T; 2]> for Bounds2 {
	fn from(value: [T; 2]) -> Self {
		Self::new(value[0], value[1])
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds3 {
	pub min: I64Vec3,
	pub max: I64Vec3,
}

impl Bounds3 {
	/// Creates a [Bounds3] which encompasses the area from `a` to `b` (inclusive).
	pub fn new<T: Into<I64Vec3>>(a: T, b: T) -> Self {
		let a: I64Vec3 = a.into();
		let b: I64Vec3 = b.into();
		let min_x = a.x.min(b.x);
		let min_y = a.y.min(b.y);
		let min_z = a.z.min(b.z);
		let max_x = a.x.max(b.x);
		let max_y = a.y.max(b.y);
		let max_z = a.z.max(b.z);
		Self {
			min: i64vec3(min_x, min_y, min_z),
			max: i64vec3(max_x, max_y, max_z)
		}
	}

	/// A radius of 1 will result in a 3x3x3 Bounds3.
	pub fn radius<T: Into<I64Vec3>>(center: T, radius: u64) -> Self {
		let center: I64Vec3 = center.into();
		let r = radius as i64;
		let min_x = center.x - r;
		let min_y = center.y - r;
		let min_z = center.z - r;
		let max_x = center.x + r;
		let max_y = center.y + r;
		let max_z = center.z + r;
		Self {
			min: i64vec3(min_x, min_y, min_z),
			max: i64vec3(max_x, max_y, max_z)
		}
	}

	pub fn min<R: From<I64Vec3>>(&self) -> R {
		R::from(self.min)
	}

	pub fn max<R: From<I64Vec3>>(&self) -> R {
		R::from(self.max)
	}

	pub fn size<R: From<I64Vec3>>(&self) -> R {
		let x = self.max.x - self.min.x + 1;
		let y = self.max.y - self.min.y + 1;
		let z = self.max.z - self.min.z + 1;
		R::from(i64vec3(x, y, z))
	}

	pub fn for_each<F: FnMut(I64Vec3) -> ()>(&self, mut f: F) {
		(self.min.y..=self.max.y).for_each(|y| {
			(self.min.z..=self.max.z).for_each(|z| {
				(self.min.x..=self.max.x).for_each(|x| {
					f(i64vec3(x, y, z));
				})
			})
		})
	}

}

// impl<T: Into<I64Vec2>,  It: IntoIterator<Item = T>> From<It> for Bounds2 {
// 	fn from(value: It) -> Self {
// 		let mut min = Option::<I64Vec2>::None;
// 		let mut max = Option::<I64Vec2>::None;
// 		value.into_iter().for_each(|coord| {
// 			if min.is_none() {
// 				let m: I64Vec2 = coord.borrow().into();
// 			}
// 		});
// 	}
// }