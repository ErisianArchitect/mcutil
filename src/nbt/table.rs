/// Macro used for accessing the tag info table which contains information
/// about all the tag types, their tag name, their type, and the last column is for implementations.
/// Implementations are applied to the type of the tag.
/// So for example: Tag::Byte is i8, so we would implement Primitive for i8.
/// Implementations:
///            Primitive: Marker that tells the compiler that this is a known-sized type and should be treated as such.
///              NonByte: Marker that tells the compiler that this is a type that is not 8-bits wide.
///     NonBytePrimitive: Marker that combines the Primitive and NonByte marker.
/// Usage:
/// ```no_run
/// macro_rules! read_table {
///     //             $id: The tag ID that is written to file to mark a type. These are distinct integer values.
///     //                  Typically the ID will be a single 8-bit value, but as the extensions become more advanced, this may change.
///     //          $title: This is the title of this NBT type. This is different from the type name, and is used as the names of 
///     //                  variants in the TagID and Tag enums.
///     //           $type: The type that the tag holds and serializes/deserializes.
///     //           $impl: These are trait implementations that are applied to the types. These traits do not do anything besides act as markers.
///     ($($id:literal $title:ident $type:path [$($impl:path),*])+) => {
///         // generation code here.
///     }
/// }
///
/// tag_info_table!(read_table);
/// ```
#[macro_export]
macro_rules! tag_info_table {
	($macro:ident) => {
		$macro! {
//ID	Title		Type						[Implementation							]
0001	Byte		i8							[$crate::nbt::family::Primitive			]
0002	Short		i16							[$crate::nbt::family::NonBytePrimitive	]
0003	Int			i32							[$crate::nbt::family::NonBytePrimitive	]
0004	Long		i64							[$crate::nbt::family::NonBytePrimitive	]
0005	Float		f32							[$crate::nbt::family::NonBytePrimitive	]
0006	Double		f64							[$crate::nbt::family::NonBytePrimitive	]
0007	ByteArray	std::vec::Vec::<i8>			[$crate::nbt::family::NonByte			]
0008	String		std::string::String			[$crate::nbt::family::NonByte			]
0009	List		$crate::nbt::tag::ListTag	[$crate::nbt::family::NonByte			]
0010	Compound	$crate::nbt::Map			[$crate::nbt::family::NonByte			]
0011	IntArray	std::vec::Vec::<i32>		[$crate::nbt::family::NonByte			]
0012	LongArray	std::vec::Vec::<i64>		[$crate::nbt::family::NonByte			]
		}
	};
}

// Python: ['Byte', 'Short', 'Int', 'Long', 'Float', 'Double', 'ByteArray', 'String', 'List', 'Compound', 'IntArray', 'LongArray']
/*
Byte,Short,Int,Long,Float,Double,ByteArray,String,List,Compound,IntArray,LongArray
*/

pub use tag_info_table;

