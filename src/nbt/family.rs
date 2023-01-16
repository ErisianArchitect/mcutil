// The traits in this file are for marking types for trait bounds.
/// Marks a type as neither i8 or u8.
pub trait NonByte {}
/// Marks a type as a primitive (scalar types such as integers or floating point numbers)
pub trait Primitive {}
/// Marks a type as both NonByte and Primitive.
pub trait NonBytePrimitive {}
impl<T: NonBytePrimitive> NonByte for T {}
impl<T: NonBytePrimitive> Primitive for T {}
