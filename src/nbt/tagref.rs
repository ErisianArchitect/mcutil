use std::fmt::Display;
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


/// Allows immutable access to a value within an NBT tag hieracrchy.
/// This includes values within `ByteArray`/`IntArray`/`LongArray`/`ListTag`.
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

/// Allows mutable access to a value within an NBT tag hieracrchy.
/// This includes values within `ByteArray`/`IntArray`/`LongArray`/`ListTag`.
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

// "dry" as in "don't repeat yourself"
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
	pub fn get_child(self, at: &TagPathPart) -> Option<ValueRef<'a>> {
		get_child_dry!(self:ValueRef at => ValueRef)
	}

	pub fn find_child(self, path: &[TagPathPart]) -> Option<ValueRef<'a>> {
		if path.is_empty() {
			return None;
		}
		let mut walker: Option<ValueRef<'a>> = self.get_child(&path[0]);
		let mut path_remaining = &path[1..];
		for part in path_remaining {
			let Some(next) = walker else { break };
			walker = next.get_child(part);
		}
		walker
	}
}

fn _set_child_at_index(node: ValueRefMut<'_>, index: i64, value: Tag) -> Result<(), ()> {
	macro_rules! set_child {
		($array:ident[$index:ident] = $variant:ident($value:ident)) => {
			{
				let Tag::$variant(value) = $value else { return Err(()) };
				// If index is negative, we want to index from the end.
				// That means subtracting index from array.len() and hoping
				// that it doesn't go into the negatives again.
				let index = if $index < 0 {
					$array.len() as i64 - $index.abs()
				} else {
					$index
				};
				if index < 0 || index >= ($array.len() as i64) {
					return Err(());
				}
				$array[index as usize] = value;
				Ok(())
			}
		};
	}
	match node {
		ValueRefMut::ByteArray(array) if value.id() == TagID::Byte => set_child!(array[index] = Byte(value)),
		ValueRefMut::IntArray(array) if value.id() == TagID::Int => set_child!(array[index] = Int(value)),
		ValueRefMut::LongArray(array) if value.id() == TagID::Long => set_child!(array[index] = Long(value)),
		ValueRefMut::List(list) => match list {
			ListTag::Byte(array) if value.id() == TagID::Byte => set_child!(array[index] = Byte(value)),
			ListTag::Short(array) if value.id() == TagID::Short => set_child!(array[index] = Short(value)),
			ListTag::Int(array) if value.id() == TagID::Int => set_child!(array[index] = Int(value)),
			ListTag::Long(array) if value.id() == TagID::Long => set_child!(array[index] = Long(value)),
			ListTag::Float(array) if value.id() == TagID::Float => set_child!(array[index] = Float(value)),
			ListTag::Double(array) if value.id() == TagID::Double => set_child!(array[index] = Double(value)),
			ListTag::ByteArray(array) if value.id() == TagID::ByteArray => set_child!(array[index] = ByteArray(value)),
			ListTag::String(array) if value.id() == TagID::String => set_child!(array[index] = String(value)),
			ListTag::List(array) if value.id() == TagID::List => set_child!(array[index] = List(value)),
			ListTag::Compound(array) if value.id() == TagID::Compound => set_child!(array[index] = Compound(value)),
			ListTag::IntArray(array) if value.id() == TagID::IntArray => set_child!(array[index] = IntArray(value)),
			ListTag::LongArray(array) if value.id() == TagID::LongArray => set_child!(array[index] = LongArray(value)),
			_ => Err(()),
		},
		_ => Err(()),
	}
}

impl<'a> ValueRefMut<'a> {
	pub fn get_child(self, at: &TagPathPart) -> Option<ValueRef<'a>> {
		// get_child_dry!(self:ValueRefMut at => ValueRef)
		ValueRef::from(self).get_child(at)
	}

	pub fn get_child_mut(self, at: &TagPathPart) -> Option<ValueRefMut<'a>> {
		get_child_dry!(self:ValueRefMut at => &mut ValueRefMut)
	}

	pub fn find_child(self, path: &[TagPathPart]) -> Option<ValueRef<'a>> {
		let valref = ValueRef::from(self);
		valref.find_child(path)
	}

	pub fn find_child_mut(self, path: &[TagPathPart]) -> Option<ValueRefMut<'a>> {
		if path.is_empty() {
			return None;
		}
		let mut walker: Option<ValueRefMut<'a>> = self.get_child_mut(&path[0]);
		let mut path_remaining = &path[1..];
		for part in path_remaining {
			let Some(next) = walker else { break };
			walker = next.get_child_mut(part);
		}
		walker
	}

	pub fn set_child<T: Into<Tag>>(self, path: &[TagPathPart], value: T) -> Result<(),()> {
		/*
		First, take all path parts from path except final part.
		Then find mutable node at that path.
		Then attempt to inject value at final path part in node.
		*/
		if path.is_empty() {
			return Err(())
		}
		let Some((last, first)) = path.split_last() else { return Err(()) };
		let Some(node) = self.find_child_mut(first) else { return Err(()) };
		let value: Tag = value.into();
		match last {
			&TagPathPart::AtIndex(index) => {
				_set_child_at_index(node, index, value)
			},
			TagPathPart::AtKey(key) => {
				match node {
					ValueRefMut::Compound(map) => {
						map.insert(key.to_owned(), value);
						Ok(())
					},
					_ => Err(()),
				}
			},
		}
	}

}

impl Tag {
	pub fn get_child(&self, at: &TagPathPart) -> Option<ValueRef<'_>> {
		ValueRef::from(self).get_child(at)
	}

	pub fn get_child_mut(&mut self, at: &TagPathPart) -> Option<ValueRefMut<'_>> {
		ValueRefMut::from(self).get_child_mut(at)
	}

	pub fn find_child(&self, path: &[TagPathPart]) -> Option<ValueRef<'_>> {
		ValueRef::from(self).find_child(path)
	}

	pub fn find_child_mut(&mut self, path: &[TagPathPart]) -> Option<ValueRefMut<'_>> {
		ValueRefMut::from(self).find_child_mut(path)
	}

	pub fn set_child<T: Into<Tag>>(&mut self, path: &[TagPathPart], value: T) -> Result<(),()> {
		ValueRefMut::from(self).set_child(path, value)
	}

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

impl<'a> From<ValueRefMut<'a>> for ValueRef<'a> {
	fn from(value: ValueRefMut<'a>) -> Self {
		match value {
			ValueRefMut::Byte(value) => ValueRef::Byte(value),
			ValueRefMut::Short(value) => ValueRef::Short(value),
			ValueRefMut::Int(value) => ValueRef::Int(value),
			ValueRefMut::Long(value) => ValueRef::Long(value),
			ValueRefMut::Float(value) => ValueRef::Float(value),
			ValueRefMut::Double(value) => ValueRef::Double(value),
			ValueRefMut::ByteArray(value) => ValueRef::ByteArray(value),
			ValueRefMut::String(value) => ValueRef::String(value),
			ValueRefMut::List(value) => ValueRef::List(value),
			ValueRefMut::Compound(value) => ValueRef::Compound(value),
			ValueRefMut::IntArray(value) => ValueRef::IntArray(value),
			ValueRefMut::LongArray(value) => ValueRef::LongArray(value),
		}
	}
}

impl<'a> From<ValueRef<'a>> for Tag {
	fn from(value: ValueRef<'a>) -> Self {
		match value {
			ValueRef::Byte(val) => Tag::Byte(*val),
			ValueRef::Short(val) => Tag::Short(*val),
			ValueRef::Int(val) => Tag::Int(*val),
			ValueRef::Long(val) => Tag::Long(*val),
			ValueRef::Float(val) => Tag::Float(*val),
			ValueRef::Double(val) => Tag::Double(*val),
			ValueRef::ByteArray(val) => Tag::ByteArray(val.clone()),
			ValueRef::String(val) => Tag::String(val.clone()),
			ValueRef::List(val) => Tag::List(val.clone()),
			ValueRef::Compound(val) => Tag::Compound(val.clone()),
			ValueRef::IntArray(val) => Tag::IntArray(val.clone()),
			ValueRef::LongArray(val) => Tag::LongArray(val.clone()),
		}
	}
}

impl<'a> From<ValueRefMut<'a>> for Tag {
    fn from(value: ValueRefMut<'a>) -> Self {
        match value {
			ValueRefMut::Byte(val) => Tag::Byte(*val),
			ValueRefMut::Short(val) => Tag::Short(*val),
			ValueRefMut::Int(val) => Tag::Int(*val),
			ValueRefMut::Long(val) => Tag::Long(*val),
			ValueRefMut::Float(val) => Tag::Float(*val),
			ValueRefMut::Double(val) => Tag::Double(*val),
			ValueRefMut::ByteArray(val) => Tag::ByteArray(val.clone()),
			ValueRefMut::String(val) => Tag::String(val.clone()),
			ValueRefMut::List(val) => Tag::List(val.clone()),
			ValueRefMut::Compound(val) => Tag::Compound(val.clone()),
			ValueRefMut::IntArray(val) => Tag::IntArray(val.clone()),
			ValueRefMut::LongArray(val) => Tag::LongArray(val.clone()),
        }
    }
}