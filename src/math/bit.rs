//! Module for bit level manipulation.
#![allow(unused)]

use std::ops::{
	Range,
	RangeBounds,
};

pub trait SetBit {
	fn set_bit(self, index: usize, on: bool) -> Self;
}

pub trait GetBit {
	fn get_bit(self, index: usize) -> bool;
	fn get_bitmask(self, mask: Range<usize>) -> Self;
}

macro_rules! __get_set_impl {
	($type:ty) => {

		impl SetBit for $type {
			fn set_bit(self, index: usize, on: bool) -> Self {
				if on {
					self | (1 << index)
				} else {
					self & !(1 << index)
				}
			}
		}

		impl GetBit for $type {
			fn get_bit(self, index: usize) -> bool {
				(self & (1 << index)) != 0
			}

			fn get_bitmask(self, mask: Range<usize>) -> Self {
				let mut result = 0;
				for i in mask.clone() {
					result = result.set_bit(i - mask.start, self.get_bit(i));
				}
				result
			}
		}

	};
}

crate::for_each_int_type!(__get_set_impl);

#[test]
fn num_test() {
	#![allow(unused)]
	use super::*;
	use std::ops::Range;
	let range = 1..4;
	let bits = 0b10111010;
	let mut result = 0;
	for i in 1..4 {
		result = result.set_bit(i - range.start, bits.get_bit(i));
	}

	println!("{}", result);
	println!("{}", (bits & 0b1111) >> 1);

}