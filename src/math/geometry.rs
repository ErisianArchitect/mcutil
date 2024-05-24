/*
This module is for geometry related stuff.
*/

/// This is a trait for getting the first bit in an integer value.
/// This is used octree indices.
pub trait FirstBit {
    fn first_bit(self) -> usize;
}

/// Transforms a value to a usize because that's not built in for some reason.
pub trait ToUsize {
    fn to_usize(self) -> usize;
}

impl ToUsize for usize {
    fn to_usize(self) -> usize {
        self
    }
}

impl FirstBit for usize {
    fn first_bit(self) -> usize {
        self & 1
    }
}

macro_rules! __numeric_impls {
    ($($type:ty),+$(,)?) => {
        $(
            impl FirstBit for $type {
                fn first_bit(self) -> usize {
                    (self & 1) as usize
                }
            }

            impl ToUsize for $type {
                fn to_usize(self) -> usize {
                    self as usize
                }
            }

        )+
    };
}

__numeric_impls![
    u8,i8,
    u16,i16,
    u32,i32,
    u64,i64,
    u128,i128,
    isize,
];

pub fn octree_node_index<T: FirstBit>(x: T, y: T, z: T) -> usize {
    y.first_bit() << 2 | z.first_bit() << 1 | x.first_bit()
}

/// In a 16x16x16 chunk section, there are 4096 blocks.
/// This function returns an index in a flattened array
/// where a block would be stored. This is merely a bit manipulation.
/// Here is how the bits would be laid out (from right to left):
/// ```text
/// ╭────────────╮
/// │       <-- 0│
/// │yyyyzzzzxxxx│
/// │321032103210│
/// ╰────────────╯
/// ```
#[inline(always)]
pub fn index_16_cube<T>(x: T, y: T, z: T) -> usize
where
T: ToUsize {
    let x = x.to_usize() & 0xf;
    let y = y.to_usize() & 0xf;
    let z = z.to_usize() & 0xf;

    (y << 8) | (z << 4) | x
}

#[inline(always)]
pub fn index_32_square<T>(x: T, y: T) -> usize
where
T: ToUsize {
    let x = x.to_usize() & 0x1f;
    let y = y.to_usize() & 0x1f;
    (y << 5) | x
}


// pub fn interleave_xyz<T>(x: T, y: T, z: T) -> usize
// where
// T: ToUsize {
// 	let (x,y,z) = (
// 		x.to_usize(),
// 		y.to_usize(),
// 		x.to_usize()
// 	);
// 	let result = 0usize;
// 	let (ix, iz, iy) = (0, 1, 2);
// }