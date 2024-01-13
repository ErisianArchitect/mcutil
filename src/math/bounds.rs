#![allow(unused)]
use std::borrow::Borrow;

use super::coord::*;
use glam::{i64::I64Vec2, i64vec2};
pub struct Bounds2 {
	pub min: I64Vec2,
	pub max: I64Vec2,
}

impl Bounds2 {
	fn new<T: Into<I64Vec2>>(a: T, b: T) -> Self {
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

	fn radius<T: Into<I64Vec2>>(center: T, radius: u64) -> Self {
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