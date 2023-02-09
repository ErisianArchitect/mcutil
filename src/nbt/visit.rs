//! A module for visiting each value in an NBT tree.

use crate::{
	nbt::{
		tag::*,
	},
};

macro_rules! make_functions {
	($($name:ident($type:ty));+$(;)?) => {
		$(
			fn $name(&mut self, value: &mut $type, param: T) -> R;
		)+
	};
}

pub trait NbtVisitor<T,R> {

	/// A key-value pair in a Compound tag.
	fn visit_named_tag(&mut self, key: &str, value: &mut Tag, param: T) -> R;

	make_functions!{
		visit_root(NamedTag);
		visit_tag(Tag);
		visit_byte(i8);
		visit_short(i16);
		visit_int(i32);
		visit_long(i64);
		visit_float(f32);
		visit_double(f64);
		visit_bytearray(Vec<i8>);
		visit_string(String);
		visit_list(ListTag);
		visit_compound(crate::nbt::Map);
		visit_intarray(Vec<i32>);
		visit_longarray(Vec<i64>);
	}

}