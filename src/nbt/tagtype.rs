use crate::nbt::Map;
use crate::nbt::tag::{
	Tag,
	ListTag,
	NbtType,
};

pub type Byte = i8;
pub type Short = i16;
pub type Int = i32;
pub type Long = i64;
pub type Float = f32;
pub type Double = f64;
pub type ByteArray = Vec<i8>;
pub type String = std::string::String; // Lol (for solidarity and isomorphism)
pub type List<T> = Vec<T>;
pub type Compound = Map;
pub type IntArray = Vec<i32>;
pub type LongArray = Vec<i64>;