#![doc = "
The format module is for formatting NBT Tags into SNBT, which is a modified
version of JSON.
"]

use chumsky::primitive::todo;

use crate::nbt::{tagtype::*, tag::ListTag, MapType};
// TODO: This module is incomplete, so don't use it unless you wanna lose a finger.
use std::{fmt::{Write, Display, Debug, Pointer}};

pub fn write_byte<W: Write>(writer: &mut W, value: Byte) -> std::fmt::Result {
	write!(writer, "{value}B")
}

pub fn write_short<W: Write>(writer: &mut W, value: Short) -> std::fmt::Result {
	write!(writer, "{value}S")
}

pub fn write_int<W: Write>(writer: &mut W, value: Int) -> std::fmt::Result {
	write!(writer, "{value}")
}

pub fn write_long<W: Write>(writer: &mut W, value: Long) -> std::fmt::Result {
	write!(writer, "{value}L")
}

pub fn write_float<W: Write>(writer: &mut W, value: Float) -> std::fmt::Result {
	write!(writer, "{value}F")
}

pub fn write_double<W: Write>(writer: &mut W, value: Double) -> std::fmt::Result {
	write!(writer, "{value}D")
}

macro_rules! array_writer {
	([$prefix:ident;]: $writer:ident, $array:ident, $sameline:ident, $indentation:ident) => {
		{
			if $array.len() > 0 {
				// If there is only one item, we will write it on the same line.
				if $sameline {
					write!($writer, "[{}; ", stringify!($prefix))?;
					write!($writer, "{}", $array[0])?;
					// $(write!($writer, "{}", stringify!($suffix))?;)?
					// write_byte(array[0], writer)?;
					$array[1..].iter().try_for_each(|value| {
						write!($writer, ", ")?;
						write!($writer, "{}", *value)?;
						// $(write!($writer, "{}", stringify!($suffix))?;)?
						Ok(())
					})?;
					write!($writer, "]")?;
				} else {
					write!($writer, "[{};\n", stringify!($prefix))?;
					let indent = $indentation.indent();
					{
						write!($writer, "{indent}")?;
						write!($writer, "{}", $array[0])?;
						// $(write!($writer, "{}", stringify!($suffix))?;)?
						$array[1..].iter().try_for_each(|value| {
							write!($writer, ",\n")?;
							write!($writer, "{indent}")?;
							write!($writer, "{}", *value)?;
							// $(write!($writer, "{}", stringify!($suffix))?;)?
							Ok(())
						})?;
						write!($writer, "\n{}]", $indentation)?;
					}
				}
			} else {
				write!($writer, "[{};]", stringify!($prefix))?;
			}
			Ok(())
		}
	};
}

pub fn write_bytearray<W: Write>(writer: &mut W, array: &[Byte], sameline: bool, indentation: Indentation) -> std::fmt::Result {
	array_writer!([B;]: writer, array, sameline, indentation)
}

pub fn write_intarray<W: Write>(writer: &mut W, array: &[Int], sameline: bool, indentation: Indentation) -> std::fmt::Result {
	array_writer!([I;]: writer, array, sameline, indentation)
}

pub fn write_longarray<W: Write>(writer: &mut W, array: &[Long], sameline: bool, indentation: Indentation) -> std::fmt::Result {
	array_writer!([L;]: writer, array, sameline, indentation)
}

pub fn write_string<W: Write>(writer: &mut W, value: &str) -> std::fmt::Result {
	write!(writer, "\"")?;
	write_escaped_string(writer, value)?;
	write!(writer, "\"")
}

pub fn write_list<W: Write>(writer: &mut W, value: &ListTag, sameline: bool, indentation: Indentation, sort_keys: bool) -> std::fmt::Result {
	macro_rules! write_func {
		($writer:ident, $list:ident, $sameline:ident, $indentation: ident: $func:ident($($arg:expr),*)$([$ref:tt])?) => {
			{
				write!($writer, "[")?;
				if !$sameline {
					write!($writer, "\n")?;
				}
				let last_index = $list.len() - 1;
				$list.iter().enumerate().try_for_each(|(index, value)| {
					// write_byte($writer, *value)?;
					if !$sameline {
						write!($writer, "{}", $indentation)?;
					}
					$func(writer, $($ref)?value, $($arg),*)?;
					if index != last_index {
						write!($writer, ",")?;
						if $sameline {
							write!($writer, " ")?;
						}
					}
					if !$sameline {
						write!($writer, "\n")?;
					}
					Ok(())
				})?;
				if !$sameline {
					write!($writer, "{}", $indentation.outdent())?;
				}
				write!($writer, "]")
			}	
		}
	}
	let indent = indentation.indent();
	match value {
		ListTag::Empty => write!(writer, "[]"),
		ListTag::Byte(list) => write_func!(writer, list, sameline, indent: write_byte()[*]),
		ListTag::Short(list) => write_func!(writer, list, sameline, indent: write_short()[*]),
		ListTag::Int(list) => write_func!(writer, list, sameline, indent: write_int()[*]),
		ListTag::Long(list) => write_func!(writer, list, sameline, indent: write_long()[*]),
		ListTag::Float(list) => write_func!(writer, list, sameline, indent: write_float()[*]),
		ListTag::Double(list) => write_func!(writer, list, sameline, indent: write_double()[*]),
		ListTag::ByteArray(list) => write_func!(writer, list, sameline, indent: write_bytearray(sameline, indent)),
		ListTag::String(list) => write_func!(writer, list, sameline, indent: write_string()),
		ListTag::List(list) => write_func!(writer, list, sameline, indent: write_list(sameline, indent, sort_keys)),
		ListTag::Compound(list) => write_func!(writer, list, sameline, indent: write_compound(sameline, indent, sort_keys)),
		ListTag::IntArray(list) => write_func!(writer, list, sameline, indent: write_intarray(sameline, indent)),
		ListTag::LongArray(list) => write_func!(writer, list, sameline, indent: write_longarray(sameline, indent)),
	}
}

pub fn write_identifier<W: Write>(writer: &mut W, ident: &str) -> std::fmt::Result {
	if is_identifier(ident) {
		write!(writer, "{ident}")
	} else {
		write_string(writer, ident)
	}
}

pub fn write_compound<W: Write>(writer: &mut W, value: &crate::nbt::Map, sameline: bool, indentation: Indentation, sort_keys: bool) -> std::fmt::Result {
	use crate::nbt::tag::*;
	if value.is_empty() {
		write!(writer, "{{}}")
	} else {
		write!(writer, "{{")?;
		if sameline {
			write!(writer, " ")?;
		} else {
			write!(writer, "\n")?;
		}
		let last_index = value.len() - 1;
		let indent = indentation.indent();
		let tags: Vec<(&String, &Tag)> = {
			let mut tags: Vec<(&String, &Tag)> = value.iter().collect();
			if sort_keys {
				tags.sort_by_key(|(key, _)| {
					key.to_owned()
				});
			}
			tags
		};
		tags.iter().enumerate().try_for_each(|(index, (key, tag))| {
			if !sameline {
				write!(writer, "{}", indent)?;
			}
			write_identifier(writer, key)?;
			write!(writer, " : ")?;
			match tag {
				Tag::Byte(value) => write_byte(writer, *value),
				Tag::Short(value) => write_short(writer, *value),
				Tag::Int(value) => write_int(writer, *value),
				Tag::Long(value) => write_long(writer, *value),
				Tag::Float(value) => write_float(writer, *value),
				Tag::Double(value) => write_double(writer, *value),
				Tag::ByteArray(array) => write_bytearray(writer, array, sameline, indent),
				Tag::String(value) => write_string(writer, value),
				Tag::List(value) => write_list(writer, value, sameline, indent, sort_keys),
				Tag::Compound(value) => write_compound(writer, value, sameline, indent, sort_keys),
				Tag::IntArray(array) => write_intarray(writer, array, sameline, indent),
				Tag::LongArray(array) => write_longarray(writer, array, sameline, indent),
			}?;
			if index != last_index {
				write!(writer, ",")?;
				if sameline {
					write!(writer, " ")?;
				} else {
					write!(writer, "\n")?;
				}
			}
			Ok(())
		})?;
		if sameline {
			write!(writer, " ")?;
		} else {
			write!(writer, "\n{}", indentation)?;
		}
		write!(writer, "}}")
	}
}

pub fn write_tag<W: Write>(writer: &mut W, tag: &crate::nbt::tag::Tag, sameline: bool, indentation: Indentation, sort_keys: bool) -> std::fmt::Result {
	use crate::nbt::tag::Tag;
	match tag {
		Tag::Byte(value) => write_byte(writer, *value),
		Tag::Short(value) => write_short(writer, *value),
		Tag::Int(value) => write_int(writer, *value),
		Tag::Long(value) => write_long(writer, *value),
		Tag::Float(value) => write_float(writer, *value),
		Tag::Double(value) => write_double(writer, *value),
		Tag::ByteArray(array) => write_bytearray(writer, array, sameline, indentation),
		Tag::String(value) => write_string(writer, value),
		Tag::List(list) => write_list(writer, list, sameline, indentation, sort_keys),
		Tag::Compound(compound) => write_compound(writer, compound, sameline, indentation, sort_keys),
		Tag::IntArray(array) => write_intarray(writer, array, sameline, indentation),
		Tag::LongArray(array) => write_longarray(writer, array, sameline, indentation),
	}
}

// TODO: This seems like useful code, so I might as well keep it around. Oh, and I'm pretty sure there's a better way to do it.
/// Measures the length of the resulting string if `n` were converted to a string.
// const fn num_width(n: i64) -> usize {
// 	const MAX_ACCUM: i64 = 1000000000000000000;
// 	match n {
// 		i64::MIN => 20,
// 		i64::MAX => 19,
// 		_ => {
// 			let mut size = if n < 0 { 2 } else { 1 };
// 			let n = n.abs();
// 			let mut accum = 10;
// 			// max: 1000000000000000000
// 			while accum <= n && accum < MAX_ACCUM {
// 				size += 1;
// 				accum *= 10;
// 			}
// 			size
// 		}
// 	}
// }

pub(crate) fn is_identifier(value: &str) -> bool {
	if value.len() == 0 {
		return false;
	}
	value.chars().try_for_each(|c| {
		if c.is_ascii_alphanumeric() || "+-_.".contains(c) {
			Ok(())
		} else {
			Err(())
		}
	}).is_ok()
}

pub(crate) fn write_escaped_string<S: AsRef<str>, W: Write>(writer: &mut W, unescaped: S) -> std::fmt::Result {
	// Macros make the whole world better!
	macro_rules! match_char {
		($buffer:expr, $input:expr; $($tok:tt => $escaped:expr),+) => {
			match $input {
				$(
					$tok => write!($buffer, "{}", $escaped),
				)+
			}
		};
	}
	unescaped.as_ref().chars().try_for_each(|ch| {
		match_char!{writer, ch;
			// TODO: Find out what escape sequences are supported by Minecraft.
			'\\' => "\\\\",
			'/' => "\\/",
			'"' => "\\\"",
			'\'' => "\\'",
			'\x08' => "\\b",
			'\x0C' => "\\f",
			'\n' => "\\n",
			'\r' => "\\r",
			'\t' => "\\t",
			'\0' => "\\0",
			other => other
		}
	})
}

/// Space count constrained to powers of two with an upper-bound of 32 and a lower bound of 1.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(usize)]
pub enum SpaceCount {
	One = 1,
	Two = 2,
	Four = 4,
	Eight = 8,
	/// Do you really need 16 spaces?
	Sixteen = 16,
	/// Come on! 32? You do not need this many spaces! But fine.
	/// Have it your way. Here are your 32 spaces!
	ThirtyTwo = 32,
	Exact(usize) = 0,
}

impl SpaceCount {
	pub fn len(&self) -> usize {
		match self {
			SpaceCount::One => 1,
			SpaceCount::Two => 2,
			SpaceCount::Four => 4,
			SpaceCount::Eight => 8,
			SpaceCount::Sixteen => 16,
			SpaceCount::ThirtyTwo => 32,
			SpaceCount::Exact(count) => *count,
		}
	}
}

impl Default for SpaceCount {
	/// Returns [SpaceCount::Four].
	fn default() -> Self {
		Self::Four
	}
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Indent {
	/// Only a single tab.
	Tabs,
	Spaces(SpaceCount),
	Text(&'static str),
}

impl Default for Indent {
	/// Returns [Indent::Spaces]\([SpaceCount::Four]\)
	fn default() -> Self {
		Self::Tabs
	}
}

impl Indent {

	/// Returns the length of the indent string.
	pub fn len(&self) -> usize {
		match self {
			Indent::Tabs => 1,
			Indent::Spaces(count) => count.len(),
			Indent::Text(text) => text.len(),
		}
	}

	pub fn indentation(self) -> Indentation {
		Indentation::new(self)
	}

	pub const fn space() -> Self {
		Self::Spaces(SpaceCount::One)
	}

	pub const fn two_spaces() -> Self {
		Self::Spaces(SpaceCount::Two)
	}

	pub const fn four_spaces() -> Self {
		Self::Spaces(SpaceCount::Four)
	}

	pub const fn eight_spaces() -> Self {
		Self::Spaces(SpaceCount::Eight)
	}
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Indentation {
	indent: Indent,
	// u32 just in case someone is really determined to write out some deeply nested structures for whatever reason.
	level: u32,
}

impl Indentation {

	/// Returns the length of the indentation string.
	pub fn len(&self) -> usize {
		self.indent.len() * self.level as usize
	}

	pub const fn new(indent: Indent) -> Self {
		Self {
			indent,
			level: 0,
		}
	}

	pub const fn level(mut self, level: u32) -> Self {
		self.level = level;
		self
	}

	pub const fn space() -> Self {
		Self::new(Indent::space())
	}

	pub const fn two_spaces() -> Self {
		Self::new(Indent::two_spaces())
	}

	pub const fn four_spaces() -> Self {
		Self::new(Indent::four_spaces())
	}

	pub const fn eight_spaces() -> Self {
		Self::new(Indent::eight_spaces())
	}

	pub const fn spaces(count: SpaceCount) -> Self {
		Self::new(Indent::Spaces(count))
	}

	pub const fn tabs() -> Self {
		Self::new(Indent::Tabs)
	}

	pub fn indent(self) -> Self {
		Self {
			indent: self.indent,
			level: self.level + 1,
		}
	}

	pub fn outdent(self) -> Self {
		if self.level > 0 {
			Self {
				indent: self.indent,
				level: self.level - 1,
			}
		} else {
			self
		}
	}
}

impl Display for Indentation {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		(0..self.level)
			.try_for_each(|_| {
				write!(f, "{}", self.indent)
			})
	}
}

impl Display for SpaceCount {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use SpaceCount::*;
		match self {
			One => write!(f, " "),
			Two => write!(f, "  "),
			Four => write!(f, "    "),
			Eight => write!(f, "        "),
			Sixteen => write!(f, "                "),
			ThirtyTwo => write!(f, "                                "),
			&Exact(mut count) => {
				let spaces32 = "                                ";
				while count >= 32 {
					write!(f, "{}", spaces32)?;
					count -= 32;
				}
				write!(f, "{}", &spaces32[0..count])
			},
		}
	}
}

impl Display for Indent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Indent::Tabs => write!(f, "\t"),
			Indent::Spaces(spaces) => write!(f, "{spaces}"),
			Indent::Text(text) => write!(f, "{text}"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn format_test() {
		let mut file = std::fs::File::create("./ignore/test_output.txt").expect("Failed to open file.");
		use crate::nbt::snbt;
		use crate::nbt::tag::*;
	
		let snbt = r#"
		{
			byte1 : 0b,
			byte2 : -10b,
			byte3 : 127b,
			short : 69s,
			int : 420,
			long : 69420L,
			float : 3f,
			float2 : 3.14f,
			double : 4d,
			double2 : 4.5d,
			double3 : 5.1,
			bytearray : [B; true, false, 5b],
			intarray : [I; 3, 5, 1],
			longarray : [L; 3l, 4l, 5l],
			lists : [
				[4b, 3b, 2b],
				[1s, -2s, 5s],
				[420, 69],
				["Hello", 'world']
			],
			compound : {
				"test" : "The quick brown fox jumps over the lazy dog.",
				nested : {
					nested : {
						nested : {
							nested : {
								leaf : "This is a secret."
							}
						}
					}
				}
			}
		}
		"#;
		if let Ok(Tag::Compound(compound)) = Tag::parse(snbt) {
			let mut text = String::new();
			write_compound(&mut text, &compound, false, Indentation::tabs(), true);
			use std::io::Write as WriteIO;
			write!(file, "{}", text);
		}
	}
}