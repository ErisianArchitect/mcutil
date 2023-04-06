// https://wiki.vg/NBT
// https://minecraft.fandom.com/wiki/NBT_format

use crate::{
	nbt::{
		Map,
		// McError,
		tag::{
			Tag,
			TagID,
			ListTag,
			NamedTag,
		},
		family::*,
		tag_info_table,
	},
	ioext::*,
	McError,
};
use std::io::{ Read, Write };

/// Trait that gives the serialization size in bytes of various values.
/// This size may include a 2 or 4 byte length, or a single byte end marker in addition to the payload.
pub trait NbtSize {
	/// Returns the serialization size of this data.
	fn nbt_size(&self) -> usize;
}

/// Trait applied to all readers for NBT extensions.
pub trait ReadNbt: Read {
	/// Read NBT (anything that implements NbtRead).
	fn read_nbt<T: NbtRead>(&mut self) -> Result<T, McError>;
}

// std::io::Read extension method read_nbt implementation.
impl<Reader: Read> ReadNbt for Reader {
	/// Read NBT (anything that implements NbtRead).
	fn read_nbt<T: NbtRead>(&mut self) -> Result<T, McError> {
		T::nbt_read(self)
	}
}

/// Trait applied to all writers for NBT extensions.
pub trait WriteNbt: Write {
	/// Write NBT (anything that implements NbtWrite).
	fn write_nbt<T: NbtWrite>(&mut self, value: &T) -> Result<usize, McError>;
}

// std::io::Write extension method write_nbt implementation.
impl<Writer: Write> WriteNbt for Writer {
	/// Write NBT (anything that implements NbtWrite).
	fn write_nbt<T: NbtWrite>(&mut self, value: &T) -> Result<usize, McError> {
		value.nbt_write(self)
	}
}

/// A trait for reading values from readers.
/// Minecraft's NBT format demands that values are read in Big-Endian byteorder, so
/// that means that it is pertinent to implement custom readers for those types.
/// By applying [NbtRead] to all the types that can be represented with NBT, we
/// are able to deserialize NBT data with greater ease.
/// Although this trait is public, it is not intended for public API usage.
pub trait NbtRead: Sized {
	/// Attempt to read a value from a reader.
	fn nbt_read<R: Read>(reader: &mut R) -> Result<Self, McError>;
}

impl<T: NbtRead> Readable for T {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self,crate::McError> {
        use crate::nbt::io::*;
		Ok(reader.read_nbt()?)
    }
}

/// A trait for writing values to writers.
/// Minecraft's NBT format demands that values are read in Big-Endian byteorder, so
/// that means that it is pertinent to implement custom writers for those types.
/// By applying [NbtWrite] to all types that can be represented with NBT, we
/// are able to deserialize NBT data with greater ease.
/// Although this trait is public, is is not intended for public API usage.
pub trait NbtWrite {
	/// Write a value to a writer.
	fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError>;
}

impl<T: NbtWrite> Writable for T {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize,crate::McError> {
        use crate::nbt::io::*;
		Ok(writer.write_nbt(self)?)
    }
}

macro_rules! tag_io {
	($($id:literal $title:ident $type:path [$($impl:path)?])+) => {
		#[doc = "
		This function is the bread and butter of serialization of NBT data.<br>
		This function will write the [Tag]'s ID, the provided [Tag] Name, and then the tag itself.
		This is necessary for writing Compound (HashMap) tags.
		This is also how the root tag of an NBT file is written.
		"]
		pub fn write_named_tag<W: Write, S: AsRef<str>>(writer: &mut W, tag: &Tag, name: S) -> Result<usize, McError> {
			let id = tag.id();
			id.nbt_write(writer)?;
			let key_size = name.as_ref().nbt_write(writer)?;
			match tag {
				$(
					Tag::$title(data) => {
						let tag_size = data.nbt_write(writer)?;
						Ok(key_size + tag_size + /* ID */ 1 )
					}
				)+
			}
		}

		#[doc = "
		Like [write_named_tag], this function is crucial to deserialization of NBT data.
		This function will first read a byte representing the [Tag] ID.
		It will then verify that the [Tag] ID is valid (can't be 0, and must match one of the Tag IDs).
		After verifying that the [Tag] ID is valid, it will read the name of the tag.
		After reading the name, it will read the tag itself, using the [Tag] ID that was read to
		determine which [Tag] type to read. Typically this will be a Compound tag (ID: 10), or a List tag (ID: 9).
		There is no restriction on what type this tag can be, though.
		"]
		pub fn read_named_tag<R: Read>(reader: &mut R) -> Result<(String, Tag), McError> {
			let id = TagID::nbt_read(reader)?;
			let name = String::nbt_read(reader)?;
			let tag = match id {
				$(
					TagID::$title => {
						Tag::$title(<$type>::nbt_read(reader)?)
					}
				)+
			};
			Ok((name, tag))
		}

		impl NbtSize for Tag {
			#[doc = "Get the number of bytes that this data will serialize to."]
			fn nbt_size(&self) -> usize {
				match self {
					$(
						Tag::$title(data) => data.nbt_size(),
					)+
				}
			}
		}

		impl NbtSize for ListTag {
			#[doc = "Get the number of bytes that this data will serialize to."]
			fn nbt_size(&self) -> usize {
				match self {
					$(
						ListTag::$title(list) => list.iter().map(|item| item.nbt_size()).sum::<usize>() + 5 /* 5 = 4 bytes for length and 1 byte for id */,
					)+
					ListTag::Empty => 5 /* 5 = 4 bytes for length and 1 byte for id */,
				}
			}
		}

		impl NbtRead for ListTag {
			#[doc = "Attempt to read a [ListTag] from a reader."]
			fn nbt_read<R: Read>(reader: &mut R) -> Result<Self, McError> {
				let id = TagID::nbt_read(reader);
				match id {
					$(
						Ok(TagID::$title) => {
							let length = u32::nbt_read(reader)?;
							Ok(ListTag::$title(
								read_array(reader, length as usize)?
							))
						},
					)+
					Err($crate::McError::EndTagMarker) => {
						u32::nbt_read(reader)?;
						Ok(ListTag::Empty)
					},
					Err(err) => {
						Err(err)
					},
				}
			}
		}

		impl NbtWrite for ListTag {
			#[doc = "Attmept to write a [ListTag] to a writer."]
			fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize,McError> {
				match self {
					$(
						ListTag::$title(list) => {
							TagID::$title.nbt_write(writer)?;
							list.nbt_write(writer).map(|size| size + 1)
						}
					)+
					ListTag::Empty => {
						0u8.nbt_write(writer)?;
						0u32.nbt_write(writer)?;
						Ok(5)
					},
				}
			}
		}

		impl NbtRead for Map {
			#[doc = "Attempt to read a [Map] from a reader."]
			fn nbt_read<R: Read>(reader: &mut R) -> Result<Self, McError> {
				// Reading goes like this:
				// Read TagID
				// if TagID is not End or Unsupported,
				//     Read string for name
				//     Read tag
				//     read next id
				//     repeat until id is End or Unsupported
				let mut map = Map::new();
				let mut id = TagID::nbt_read(reader);
				while !matches!(id, Err($crate::McError::EndTagMarker)) {
					let name = String::nbt_read(reader)?;
					let tag = match id {
						$(
							Ok(TagID::$title) => Tag::$title(<$type>::nbt_read(reader)?),
						)+
						Err(err) => return Err(err),
					};
					map.insert(name, tag);
					id = TagID::nbt_read(reader);
				}
				Ok(map)
			}
		}

		impl NbtWrite for Tag {
			#[doc = "Attempt to write a [Tag]"]
			fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError> {
				match self {
					$(
						Tag::$title(tag) => tag.nbt_write(writer),
					)+
				}
			}
		}
	};
}

/// Blanket implementations for reading and writing primitives (scalar types).
macro_rules! primitive_io {
	($($primitive:ident)+) => {
		$(
			impl NbtRead for $primitive {
				#[doc ="Attempts to read primitive from reader. This will read in Big-Endian byte-order."]
				fn nbt_read<R: Read>(reader: &mut R) -> Result<Self, McError> {
					let mut buf = [0u8; std::mem::size_of::<$primitive>()];
					reader.read_exact(&mut buf)?;
					Ok(Self::from_be_bytes(buf))
				}
			}

			impl NbtWrite for $primitive {
				#[doc = "Attempts to write primitive to writer. This will write in Big-Endian byte-order."]
				fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError> {
					Ok(writer.write(self.to_be_bytes().as_slice())?)
				}
			}
		)+
	};
}

/// These are the primitive types that will be read and written in Big-Endian order.
primitive_io![
	i8 u8
	i16 u16
	i32 u32 f32
	i64 u64 f64
	i128 u128
];

tag_info_table!(tag_io);

/// Reads an exact number of bytes from a reader, returning them as a [Vec].
fn read_bytes<R: Read>(reader: &mut R, length: usize) -> Result<Vec<u8>, McError> {
	let mut buf: Vec<u8> = vec![0u8; length];
	reader.read_exact(&mut buf)?;
	Ok(buf)
}

/// Writes a byte slice to a writer, returning the number of bytes that were written.
fn write_bytes<W: Write>(writer: &mut W, data: &[u8]) -> Result<usize, McError> {
	Ok(writer.write_all(data).map(|_| data.len())?)
}

/// Reads a certain number of elements from a reader.
fn read_array<R, T>(reader: &mut R, length: usize) -> Result<Vec<T>, McError>
where
	R: Read,
	T: NbtRead,
{
	(0..length).map(|_| T::nbt_read(reader)).collect()
}

/// Writes elements to a writer, returning the total number of bytes written.
fn write_array<W, T>(writer: &mut W, data: &[T]) -> Result<usize, McError>
where
	W: Write,
	T: NbtWrite,
{
	data.iter().map(|item| item.nbt_write(writer)).sum()
}

impl<T: Primitive + Sized> NbtSize for T {
	/// Get the number of bytes that this data will serialize to.
	fn nbt_size(&self) -> usize {
		std::mem::size_of::<T>()
	}
}

impl<T: Primitive + Sized> NbtSize for Vec<T> {
	/// Get the number of bytes that this data will serialize to.
	fn nbt_size(&self) -> usize {
		std::mem::size_of::<T>() * self.len() + 4usize
	}
}

impl NbtSize for String {
	/// Get the number of bytes that this data will serialize to.
	fn nbt_size(&self) -> usize {
		/*2 bytes for the length*/ 2usize + self.len()
	}
}

impl NbtSize for Vec<String> {
	/// Returns the size that this would be written as NBT.
	/// It will add 4 to the sum size of the elements, marking
	/// the number of bytes reserved for the length, which is
	/// a requirement to write this to memory.
	fn nbt_size(&self) -> usize {
		self.iter()
			.map(|value| value.nbt_size())
			.sum::<usize>()
			+ 4 // +4 for u32 size
	}
}

impl NbtSize for Map {
	/// Get the serialization size in bytes.
	/// This will determine the total serialization size of this data when written to a writer.
	fn nbt_size(&self) -> usize {
		self.iter()
			.map(|(name, tag)| name.nbt_size() + tag.nbt_size() + 1)
			.sum::<usize>()
			+ 1 // The +1 represents the TagID::End that marks the end of the map.
	}
}

impl NbtSize for Vec<Map> {
	/// Get the serialization size in bytes.
	/// The length of the [Vec] is part of serialization, which adds 4 bytes to the total size.
	fn nbt_size(&self) -> usize {
		self.iter()
			.map(|value| value.nbt_size())
			.sum::<usize>()
			+ 4 // +4 for u32 size
	}
}

impl NbtSize for Vec<ListTag> {
	/// Get the serialization size in bytes.
	/// The length of the [ListTag] is part of serialization, which adds 4 bytes to the total size.
	fn nbt_size(&self) -> usize {
		self.iter()
			.map(|value| value.nbt_size())
			.sum::<usize>()
			+ 4 // +4 for u32 size
	}
}

// For reading Named Tag straight into a Tuple.
impl<S: From<String>, T: From<Tag>> NbtRead for (S, T) {
	/// For reading a named tag straight into a Tuple.
	fn nbt_read<R: Read>(reader: &mut R) -> Result<Self, McError> {
		let (name, tag) = read_named_tag(reader)?;
		Ok((S::from(name), T::from(tag)))
	}
}

impl<T: NbtRead + NonByte> NbtRead for Vec<T> {
	/// Read a [Vec] from a reader.
	fn nbt_read<R: Read>(reader: &mut R) -> Result<Self, McError> {
		let length = u32::nbt_read(reader)?;
		read_array(reader, length as usize)
	}
}

impl NbtRead for Vec<i8> {
	/// Read a bytearray from a reader.
	fn nbt_read<R: Read>(reader: &mut R) -> Result<Self, McError> {
		let length = u32::nbt_read(reader)?;
		let bytes = read_bytes(reader, length as usize)?;
		// Use compiler magic to convert Vec<u8> to Vec<i8>
		Ok(
			bytes.into_iter()
				.map(|x| x as i8)
				.collect()
		)
	}
}

impl NbtRead for String {
	/// Read a String from a reader.
	fn nbt_read<R: Read>(reader: &mut R) -> Result<Self, McError> {
		// 🦆 <-- Frank
		// Frank: How does this function work, eh?
		// Me: Well, you see, to read a string in NBT format, we first
		//     need to read a 16-bit unsigned big endian integer, that
		//     signifies our length. We then read that number of bytes
		//     and interpret those bytes as a utf-8 string.
		let length: u16 = u16::nbt_read(reader)?;
		let strbytes = read_bytes(reader, length as usize)?;
		Ok(String::from_utf8(strbytes)?)
	}
}

impl NbtRead for TagID {
	/// Read a TagID from a reader. If `0` is encountered, this will return `Err(McError::End)`.
	fn nbt_read<R: Read>(reader: &mut R) -> Result<Self, McError> {
		TagID::try_from(u8::nbt_read(reader)?)
	}
}

impl<S: AsRef<str>> NbtWrite for (S, Tag) {
	/// Write a Tuple as a named tag to a writer.
	fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError> {
		write_named_tag(writer, &self.1, self.0.as_ref())
	}
}

impl NbtWrite for TagID {
	/// Write a TagID to a writer.
	fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError> {
		(self.value() as u8).nbt_write(writer)
	}
}

impl NbtRead for NamedTag {
	#[doc = "Attempt to read a [NamedTag] from a reader. This is a wrapper around `read_named_tag(reader)"]
	fn nbt_read<R: Read>(reader: &mut R) -> Result<NamedTag, McError> {
		Ok(read_named_tag(reader)?.into())
	}
}

impl NbtWrite for &str {
	/// Write a string to a writer.
	fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError> {
		let length: u16 = self.len() as u16;
		length.nbt_write(writer)?;
		Ok(writer.write_all(self.as_bytes()).map(|_| self.len() + 2)?)
	}
}

impl NbtWrite for String {
	/// Write a string to a writer.
	fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError> {
		self.as_str().nbt_write(writer)
	}
}

// This is a special implementation for writing Vectors of types that
// are not u8 or i8.
impl<T: NbtWrite + NonByte> NbtWrite for Vec<T> {
	/// Write a [Vec] to a writer.
	/// This will also write the size of the [Vec] as a Big-Endian 32-bit integer.
	fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError> {
		(self.len() as u32).nbt_write(writer)?;
		write_array(writer, self.as_slice()).map(|size| size + 4) // The `+ 4` is to add the size of the u32 length
	}
}

// This is a special implementation for writing Vec<i8>.
// Profiling showed that this was an improvement, so it's what I'm going with.
impl NbtWrite for Vec<i8> {
	/// Write a bytearray to a writer.
	fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError> {
		(self.len() as u32).nbt_write(writer)?;
		let u8slice: &[u8] = bytemuck::cast_slice(self.as_slice());
		Ok(write_bytes(writer, u8slice)? + 4) // The `+ 4` is to add the size of the u32 length
	}
}

impl NbtWrite for Map {
	/// Write a [Map] to a writer.
	fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError> {
		// Writing goes like this:
		// for each key/value pair, write:
		//     TagID of value
		//     name string
		//     Payload
		// After iteration, write TagID::End (0u8)
		let write_size = self.iter().try_fold(0usize, |size, (key, tag)| {
			write_named_tag(writer, tag, key)
				.map(|written| written + size)
		})?;
		0u8.nbt_write(writer).map(|size| write_size + size)
	}
}

impl NbtWrite for NamedTag {
	#[doc = "Attempt to write a [NamedTag] to a writer. This is a wrapper around `write_named_tag(writer, self.tag(), self.name())`"]
	fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError> {
		write_named_tag(writer, &self.tag, &self.name)
	}
}


impl NbtSize for NamedTag {
	/// Get the serialization size in bytes.
	fn nbt_size(&self) -> usize {
		self.name.nbt_size() + self.tag.nbt_size() + 1 // The `+ 1` is to add the size of the 0x00 byte for the end tag.
	}
}

impl<T> NbtWrite for &T
where T: NbtWrite {
    fn nbt_write<W: Write>(&self, writer: &mut W) -> Result<usize, McError> {
        self.write_to(writer)
    }
}

impl<T> NbtRead for &T
where T: NbtRead {
    fn nbt_read<R: Read>(reader: &mut R) -> Result<Self, McError> {
        Self::read_from(reader)
    }
}

#[cfg(test)]
mod tests {
	use crate::nbt::*;
	use crate::nbt::io::*;
	use crate::nbt::tag::*;

	fn test_tag() -> Tag {
		let byte = Tag::Byte(i8::MAX);
		let short = Tag::Short(i16::MAX);
		let int = Tag::Int(69420);
		let long = Tag::Long(i64::MAX);
		let float = Tag::Float(3.14_f32);
		let double = Tag::Double(3.14159265358979_f64);
		let bytearray = Tag::ByteArray(vec![1,2,3,4]);
		let string = Tag::String(String::from("The quick brown fox jumps over the lazy dog🎈🎄"));
		let list = Tag::List(ListTag::from(vec![1i32,2,3,4]));
		let intarray = Tag::IntArray(vec![1,1,2,3,5,8,13,21,34,55,89,144]);
		let longarray = Tag::LongArray(vec![1,3,3,7, 1337, 13,37, 1,3,37,1,337, 133,7, 1,33,7,13,3,7]);
		let mut compound = Map::from([
			("Byte".to_owned(), byte),
			("Short".to_owned(), short),
			("Int".to_owned(), int),
			("Long".to_owned(), long),
			("Float".to_owned(), float),
			("Double".to_owned(), double),
			("ByteArray".to_owned(), bytearray),
			("String".to_owned(), string),
			("List".to_owned(), list),
			("Empty List".to_owned(), Tag::List(ListTag::Empty)),
			("IntArray".to_owned(), intarray),
			("LongArray".to_owned(), longarray),
		]);
		let mapclone = compound.clone();
		compound.insert("Compound".to_owned(), Tag::Compound(mapclone));
		Tag::Compound(compound)
	}
}