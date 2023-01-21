use std::process::Child;

use chumsky::primitive::todo;

/*
This is to create a reference representation type of nbt::Tag and nbt::ListTag.
This is meant to allow for accessing values directly.
*/
use crate::nbt::tag::*;
use crate::nbt::tagpath;
use crate::nbt::tagtype::*;

use super::tagpath::TagPathPart;

// So the idea is that a TagPath can be used to access elements/values within
// a Tag.
// So we want to create two types: TagRef, and TagRefMut
// The type that I plan on creating should be able to access values
// of each tag type, but should also be able to access values
// within a List tag and Compound tag.
// The TagRef should not hold information about whether it is sourced
// from a tag or if it is sourced from an array/list.
// So there would be no fundamental difference between accessing a Byte
// tag inside a compound and accessing an individual byte within a list
// or array.

// pub trait NbtNode {
// 	fn get_child<'a>(&'a self, at: &TagPathPart) -> Option<ValueRef<'a>>;
// 	fn get_child_mut<'a>(&'a mut self, at: &TagPathPart) -> Option<ValueRefMut<'a>> { None }
// 	fn find_child<'a>(&'a self, path: &[TagPathPart]) -> Option<ValueRef<'a>> { 
// 		if path.is_empty() {
// 			return None;
// 		}
// 		let mut walker: Option<ValueRef<'a>> = self.get_child(&path[0]);
// 		let mut path_remaining = &path[1..];
// 		while !path_remaining.is_empty() && walker.is_some() {
// 			walker = walker.and_then(|valref| {
// 				valref.get_child(&path_remaining[0])
// 			});
// 			path_remaining = &path_remaining[1..];
// 		}
// 		walker
// 	}

// 	fn find_child_mut<'a>(&'a mut self, path: &[TagPathPart]) -> Option<ValueRefMut<'a>> {
// 		if path.is_empty() {
// 			return None;
// 		}
// 		let mut walker: Option<ValueRefMut<'a>> = self.get_child_mut(&path[0]);
// 		let mut path_remaining = &path[1..];
// 		while !path_remaining.is_empty() && walker.is_some() {
// 			walker = walker.and_then(|valref| {
// 				valref.get_child_mut(&path_remaining[0])
// 			});
// 			path_remaining = &path_remaining[1..];
// 		}
// 		walker
// 	}
// }

macro_rules! get_child_in_array {
	($reftype:ident::$variant:ident($([$mut:ident])? $node:ident[$index:expr])) => {
		{
			let index = {
				if $index >= 0 {
					$index
				} else {
					$node.len() as i64 - $index.abs()
				}
			};
			if index >= 0 && index < $node.len() as i64 {
				Some($reftype::$variant(&$($mut)? $node[index as usize]))
			} else {
				None
			}
		}
	};

}

macro_rules! get_child_dry {
	(@map_get;mut;$map:ident $key:ident) => {
		$map.get_mut($key).unwrap()
	};
	(@map_get;;$map:ident $key:ident) => {
		$map.get($key).unwrap()
	};
	($self_val:ident:$self_type:ident $at_val:ident => $(&$mut:tt)? $enum_type:ident) => {
		match $at_val {
			&TagPathPart::AtIndex(index) => {
				match $self_val {
					$self_type::List(node) => match node {
						ListTag::Empty => None,
						ListTag::Byte(node) => get_child_in_array!($enum_type::Byte($([$mut])? node[index])),
						ListTag::Short(node) => get_child_in_array!($enum_type::Short($([$mut])? node[index])),
						ListTag::Int(node) => get_child_in_array!($enum_type::Int($([$mut])? node[index])),
						ListTag::Long(node) => get_child_in_array!($enum_type::Long($([$mut])? node[index])),
						ListTag::Float(node) => get_child_in_array!($enum_type::Float($([$mut])? node[index])),
						ListTag::Double(node) => get_child_in_array!($enum_type::Double($([$mut])? node[index])),
						ListTag::ByteArray(node) => get_child_in_array!($enum_type::ByteArray($([$mut])? node[index])),
						ListTag::String(node) => get_child_in_array!($enum_type::String($([$mut])? node[index])),
						ListTag::List(node) => get_child_in_array!($enum_type::List($([$mut])? node[index])),
						ListTag::Compound(node) => get_child_in_array!($enum_type::Compound($([$mut])? node[index])),
						ListTag::IntArray(node) => get_child_in_array!($enum_type::IntArray($([$mut])? node[index])),
						ListTag::LongArray(node) => get_child_in_array!($enum_type::LongArray($([$mut])? node[index])),
					},
					$self_type::ByteArray(node) => get_child_in_array!($enum_type::Byte($([$mut])? node[index])),
					$self_type::IntArray(node) => get_child_in_array!($enum_type::Int($([$mut])? node[index])),
					$self_type::LongArray(node) =>  get_child_in_array!($enum_type::Long($([$mut])? node[index])),
					_ => None,
				}
			},
			TagPathPart::AtKey(key) => {
				match $self_val {
					$self_type::Compound(map) if map.contains_key(key) => {
						Some(match get_child_dry!(@map_get;$($mut)?;map key) {
							Tag::Byte(child) => $enum_type::Byte(child),
							Tag::Short(child) => $enum_type::Short(child),
							Tag::Int(child) => $enum_type::Int(child),
							Tag::Long(child) => $enum_type::Long(child),
							Tag::Float(child) => $enum_type::Float(child),
							Tag::Double(child) => $enum_type::Double(child),
							Tag::ByteArray(child) => $enum_type::ByteArray(child),
							Tag::String(child) => $enum_type::String(child),
							Tag::List(child) => $enum_type::List(child),
							Tag::Compound(child) => $enum_type::Compound(child),
							Tag::IntArray(child) => $enum_type::IntArray(child),
							Tag::LongArray(child) => $enum_type::LongArray(child),
						})
					},
					_ => None,
				}
			},
		}
	};
}

macro_rules! find_child_dry {
	($self:ident,$path:ident => $type_name:ident = $get_fn:ident()) => {
		{
			if $path.is_empty() {
				return None;
			}
			let mut walker: Option<$type_name<'b>> = $self.$get_fn(&$path[0]);
			let mut path_remaining = &$path[1..];
			while !path_remaining.is_empty() && walker.is_some() {
				walker = walker.and_then(|result| result.$get_fn(&path_remaining[0]));
				path_remaining = &path_remaining[1..];
			}
			walker
		}
	};
}

impl<'a> ValueRef<'a> {
	pub fn get_child<'b>(&'b self, at: &TagPathPart) -> Option<ValueRef<'a>> {
		get_child_dry!(self:ValueRef at => ValueRef)
	}
}

impl Tag {
	pub fn get_child<'a>(&'a self, at: &TagPathPart) -> Option<ValueRef<'a>> {
		get_child_dry!(self:Tag at => ValueRef)
	}

	pub fn get_child_mut<'a>(&'a mut self, at: &TagPathPart) -> Option<ValueRefMut<'a>> {
		get_child_dry!(self:Tag at => &mut ValueRefMut)
	}

	pub fn find_child<'a>(&'a self, path: &[TagPathPart]) -> Option<ValueRef<'a>> {
		if path.is_empty() {
			return None;
		}
		let mut walker: Option<ValueRef<'a>> = self.get_child(&path[0]);
		let mut path_remaining = &path[1..];
		while !path_remaining.is_empty() && walker.is_some() {
			walker = walker.and_then(|result| result.get_child(&path_remaining[0]));
			path_remaining = &path_remaining[1..];
		}
		walker
	}

	// fn find_child_mut<'a>(&'a mut self, path: &[TagPathPart]) -> Option<ValueRefMut<'a>> {
	// 	if path.is_empty() {
	// 		return None;
	// 	}
	// 	let mut walker: Option<ValueRefMut<'a>> = self.get_child_mut(&path[0]);
	// 	let mut path_remaining = &path[1..];
	// 	while !path_remaining.is_empty() && walker.is_some() {
	// 		walker = walker.and_then(|mut result| result.get_child_mut(&path_remaining[0]));
	// 		path_remaining = &path_remaining[1..];
	// 	}
	// 	walker
	// }

}

impl<'a> ValueRefMut<'a> {
    fn get_child(&'a self, at: &TagPathPart) -> Option<ValueRef<'a>> {
        get_child_dry!(self:ValueRefMut at => ValueRef)
    }

    fn get_child_mut(&'a mut self, at: &TagPathPart) -> Option<ValueRefMut<'a>> {
        get_child_dry!(self:ValueRefMut at => &mut ValueRefMut)
    }

	fn find_child(&'a self, path: &[TagPathPart]) -> Option<ValueRef<'a>> {
		if path.is_empty() {
			return None;
		}
		let mut walker: Option<ValueRef<'a>> = self.get_child(&path[0]);
		let mut path_remaining = &path[1..];
		while !path_remaining.is_empty() && walker.is_some() {
			walker = walker.and_then(|result| result.get_child(&path_remaining[0]));
			path_remaining = &path_remaining[1..];
		}
		walker
	}

}

#[derive(Clone, Copy)]
#[repr(isize)]
pub enum ValueRef<'a> {
	Byte(&'a Byte) = 1,
	Short(&'a Short) = 2,
	Int(&'a Int) = 3,
	Long(&'a Long) = 4,
	Float(&'a Float) = 5,
	Double(&'a Double) = 6,
	ByteArray(&'a ByteArray) = 7,
	String(&'a String) = 8,
	List(&'a ListTag) = 9,
	Compound(&'a Compound) = 10,
	IntArray(&'a IntArray) = 11,
	LongArray(&'a LongArray) = 12,
}

#[repr(isize)]
pub enum ValueRefMut<'a> {
	Byte(&'a mut Byte) = 1,
	Short(&'a mut Short) = 2,
	Int(&'a mut Int) = 3,
	Long(&'a mut Long) = 4,
	Float(&'a mut Float) = 5,
	Double(&'a mut Double) = 6,
	ByteArray(&'a mut ByteArray) = 7,
	String(&'a mut String) = 8,
	List(&'a mut ListTag) = 9,
	Compound(&'a mut Compound) = 10,
	IntArray(&'a mut IntArray) = 11,
	LongArray(&'a mut LongArray) = 12,
}

impl<'a> From<&'a mut Tag> for ValueRefMut<'a> {
	fn from(value: &'a mut Tag) -> Self {
		match value {
			Tag::Byte(val) => ValueRefMut::Byte(val),
			Tag::Short(val) => ValueRefMut::Short(val),
			Tag::Int(val) => ValueRefMut::Int(val),
			Tag::Long(val) => ValueRefMut::Long(val),
			Tag::Float(val) => ValueRefMut::Float(val),
			Tag::Double(val) => ValueRefMut::Double(val),
			Tag::ByteArray(val) => ValueRefMut::ByteArray(val),
			Tag::String(val) => ValueRefMut::String(val),
			Tag::List(val) => ValueRefMut::List(val),
			Tag::Compound(val) => ValueRefMut::Compound(val),
			Tag::IntArray(val) => ValueRefMut::IntArray(val),
			Tag::LongArray(val) => ValueRefMut::LongArray(val),
		}
	}
}

impl<'a> From<&'a Tag> for ValueRef<'a> {
	fn from(value: &'a Tag) -> Self {
		match value {
			Tag::Byte(val) => ValueRef::Byte(val),
			Tag::Short(val) => ValueRef::Short(val),
			Tag::Int(val) => ValueRef::Int(val),
			Tag::Long(val) => ValueRef::Long(val),
			Tag::Float(val) => ValueRef::Float(val),
			Tag::Double(val) => ValueRef::Double(val),
			Tag::ByteArray(val) => ValueRef::ByteArray(val),
			Tag::String(val) => ValueRef::String(val),
			Tag::List(val) => ValueRef::List(val),
			Tag::Compound(val) => ValueRef::Compound(val),
			Tag::IntArray(val) => ValueRef::IntArray(val),
			Tag::LongArray(val) => ValueRef::LongArray(val),
		}
	}
}

impl<'a> From<ValueRef<'a>> for Tag {
	fn from(value: ValueRef<'a>) -> Self {
		match value {
			ValueRef::Byte(val) => Tag::Byte(val.clone()),
			ValueRef::Short(val) => Tag::Short(val.clone()),
			ValueRef::Int(val) => Tag::Int(val.clone()),
			ValueRef::Long(val) => Tag::Long(val.clone()),
			ValueRef::Float(val) => Tag::Float(val.clone()),
			ValueRef::Double(val) => Tag::Double(val.clone()),
			ValueRef::ByteArray(val) => Tag::ByteArray(val.clone()),
			ValueRef::String(val) => Tag::String(val.clone()),
			ValueRef::List(val) => Tag::List(val.clone()),
			ValueRef::Compound(val) => Tag::Compound(val.clone()),
			ValueRef::IntArray(val) => Tag::IntArray(val.clone()),
			ValueRef::LongArray(val) => Tag::LongArray(val.clone()),
		}
	}
}