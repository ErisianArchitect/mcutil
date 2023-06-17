//! Module for bit level manipulation.
#![allow(unused)]

use std::ops::{
	Range,
	RangeBounds,
};

use crate::for_each_int_type;

pub trait BitSize {
	const BITSIZE: usize;
}

pub trait ShiftIndex: Copy {
	/// A `u32` value that represents an index that a `1` bit can be shifted to.
	fn shift_index(self) -> u32;
}

macro_rules! __shiftindex_impls {
	($type:ty) => {
		impl ShiftIndex for $type {
			fn shift_index(self) -> u32 {
				self as u32
			}
		}
	};
}

for_each_int_type!(__shiftindex_impls);

macro_rules! __bitsize_impls {
	($type:ty) => {
		impl BitSize for $type {
			const BITSIZE: usize = std::mem::size_of::<$type>() * 8;
		}
	};
}

for_each_int_type!(__bitsize_impls);

pub trait SetBit {
	fn set_bit<I: ShiftIndex>(self, index: I, on: bool) -> Self;
}

pub trait GetBit {
	fn get_bit<I: ShiftIndex>(self, index: I) -> bool;
	fn get_bitmask(self, mask: Range<usize>) -> Self;
}

pub trait InvertBit {
	fn invert_bit<I: ShiftIndex>(self, index: I) -> Self;
}

impl<T: GetBit + SetBit + Copy> InvertBit for T {
	fn invert_bit<I: ShiftIndex>(self, index: I) -> Self {
		let bit = self.get_bit(index);
		self.set_bit(index, !bit)
	}
}

macro_rules! __get_set_impl {
	($type:ty) => {

		impl SetBit for $type {
			fn set_bit<I: ShiftIndex>(self, index: I, on: bool) -> Self {
				if let (mask, false) = (1 as $type).overflowing_shl(index.shift_index()) {
					if on {
						self | mask
					} else {
						self & !mask
					}
				} else {
					self
				}
			}
		}

		impl GetBit for $type {
			fn get_bit<I: ShiftIndex>(self, index: I) -> bool {
				if let (mask, false) = (1 as $type).overflowing_shl(index.shift_index()) {
					(self & mask) != 0
				} else {
					false
				}
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

/// To allow polymorphism for iterators of different integer types or references to integer types.
pub trait MoveBitsIteratorItem {
	fn translate(self) -> usize;
}

pub trait MoveBits: Sized {
	fn move_bits<T: MoveBitsIteratorItem, It: IntoIterator<Item = T>>(self, new_indices: It) -> Self;
	/// Much like move_bits, but takes indices in reverse order. This is useful if you want to have the
	/// indices laid out more naturally from right to left.
	fn move_bits_rev<T: MoveBitsIteratorItem, It: IntoIterator<Item = T>>(self, new_indices: It) -> Self
	where It::IntoIter: DoubleEndedIterator {
		self.move_bits(new_indices.into_iter().rev())
	}
}

macro_rules! __movebits_impls {
	($type:ty) => {
		impl MoveBitsIteratorItem for $type {
			fn translate(self) -> usize {
				self as usize
			}
		}

		impl MoveBitsIteratorItem for &$type {
			fn translate(self) -> usize {
				*self as usize
			}
		}
	};
}

for_each_int_type!(__movebits_impls);

impl<T: BitSize + GetBit + SetBit + Copy> MoveBits for T {
	fn move_bits<I: MoveBitsIteratorItem, It: IntoIterator<Item = I>>(self, new_indices: It) -> Self {
		new_indices.into_iter()
			.map(I::translate)
			.enumerate()
			.take(Self::BITSIZE)
			.fold(self, |value, (index, swap_index)| {
				let on = value.get_bit(swap_index);
				value.set_bit(index, on)
			})
	}
}

#[test]
fn move_bits_test() {
	use super::*;// 76543210
	let result = (0b10110001u32).move_bits([6, 3, 4, 7, 1, 0, 2, 5]);
	println!("Result: {result}");
}

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