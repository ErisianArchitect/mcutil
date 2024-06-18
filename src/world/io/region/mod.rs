pub mod header;
pub mod sector;
pub use sector::RegionSector;
pub mod timestamp;
pub use timestamp::Timestamp;
pub mod coord;
pub use coord::RegionCoord;
pub mod info;
pub mod compressionscheme;
pub use compressionscheme::CompressionScheme;
pub mod managedsector;
pub use managedsector::ManagedSector;
pub mod sectormanager;
pub use sectormanager::*;
pub mod regionfile;
pub use regionfile::RegionFile;
pub mod prelude;

/*	╭──────────────────────────────────────────────────────────────────────────────╮
    │ How do Region Files work?                                                    │
    ╰──────────────────────────────────────────────────────────────────────────────╯
    Region files have an 8KiB header that contains two tables, each table with 1024
    32-bit elements.

    The first table is the Sector Offset table. Sector offsets are 2 values, the
    actual offset, and the size. Both of these values are packed into 4 bytes. The
    offset is 3 bytes big-endian and the size is 1 byte. They are laid out in 
    memory like so: |offset(3)|size(1)|
    This layout means that when these 4 bytes are turned into a single 32-bit
    unsigned integer, the individual values can be access like so:
        For the offset:	value_u32 >> 8
        For the size:	value_u32 & 0xFF
    This is the first 4KiB.

    Directly fter the offset table is the timestamp table, which also contains 1024
    32-bit elements. The timestamps are Unix timestamps in (I believe UTC).

    These 1024 elements in these 2 tables represent data associated with some chunk
    that may be written to the file. There are 32x32 potential slots for chunks.
    If a chunk is not present, the offset value will be 0, or the length within the
    sector is 0 (more on that later.)

    Both values within the sector offset must be multiplied by 4096 in order to get
    the actual value. So to get the stream offset that you must seek to in order to
    find this sector, simply multiply the offset value by 4096. To get the size
    within the file that the data occupies, multiply the size by 4096.

    If the sector offset's values are not 0, there may be a chunk present in the
    file. If you go to the file offset that the sector offset points to, you will
    find a 32-bit unsigned (big-endian) integer representing the size in bytes of
    the data following that unsigned integer. If this value is zero, that means
    there is no data stored, but there is still a sector being occupied. I don't
    know if that is something that happens in region files, I have yet to do that
    research.

    TODO: Research whether or not Minecraft ever saves a sector offset as
        : occupied while the length at that offset is zero.

    Following the length is a single byte representing the compression scheme used
    to save that chunk. The possible values are 1 for GZip, 2 for ZLib, and 3 for 
    uncompressed. After the compression scheme are (length - 1) bytes of data that
    represent a chunk within a Minecraft world, which is in NBT format. This chunk
    is a named tag.

    After the chunk is some pad bytes (typically zeroes, but I don't think that it
    is a requirement that the pad bytes are zeroes).

    The region file's size MUST be a multiple of 4096. I'm pretty sure Minecraft
    will reject it if it's not.
*/

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
    4096 - (size & 4095)
}