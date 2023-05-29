pub mod header;
pub mod sector;
pub mod timestamp;
pub mod coord;
pub mod info;
pub mod reader;
pub mod writer;
pub mod compressionscheme;
pub mod managedsector;
pub mod sectormanager;
pub mod regionfile;

/// Tests if a value is a multiple of 4096.
pub const fn is_multiple_of_4096(n: u64) -> bool {
	(n & 4095) == 0
}

/// Counts the number of 4KiB sectors required to accomodate `size` bytes.
pub const fn required_sectors(size: u32) -> u32 {
	// Yay for branchless programming!
	let sub = size.overflowing_shr(12).0;
	// use some casting magic to turn a boolean into an integer.
	// true => 1 | false => 0
	let overflow = !is_multiple_of_4096(size as u64) as u32;
	sub + overflow
}

/// Returns the 4KiB pad size for the given size.
/// The pad size is the number of bytes required
/// to add to the size in order to make it a
/// multiple of 4096.
pub const fn pad_size(size: u64) -> u64 {
	// Some bit-level hacking makes this really easy.
	(4096 - (size & 4095)) & 4095
}