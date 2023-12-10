use crate::for_each_int_type;
use crate::nbt::Map;
use crate::nbt::tag::{
	Tag,
	TagID,
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

pub trait TypeId {
	fn tag_id() -> TagID;
}

macro_rules! typeid_impls {
	($($types:ty => $id:expr;)+) => {
		$(
			impl TypeId for $types {
				fn tag_id() -> TagID {
					$id
				}
			}
		)+
	};
}

typeid_impls!(
	Byte => TagID::Byte;
	Short => TagID::Short;
	Int => TagID::Int;
	Long => TagID::Long;
	Float => TagID::Float;
	Double => TagID::Double;
	ByteArray => TagID::ByteArray;
	String => TagID::String;
	ListTag => TagID::List;
	Compound => TagID::Compound;
	IntArray => TagID::IntArray;
	LongArray => TagID::LongArray;
);