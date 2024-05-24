use crate::{
    ioext::*,
    McResult,
    math::bit::{GetBit, SetBit},
};
use super::{
    header::*,
    coord::*,
    sector::*,
    timestamp::*,
    is_multiple_of_4096,
};
use std::{
    path::{PathBuf, Path},
    fs::{
        Metadata,
        File,
    },
    io::{
        BufReader, Seek,
    },
};

// /// Tests if a value is a multiple of 4096.
// const fn is_multiple_of_4096(n: u64) -> bool {
// 	(n & 4095) == 0
// }

/// This is a bitmask containing 1024 bits.
/// This can be used however you want, but it was created
/// as a way to store flags for present chunks.
pub struct RegionBitmask(Box<[u32; 32]>);

/// Info about a region file.
/// This info includes:
/// - Metadata
/// - Chunk Sectors
/// - Timestamps
/// - Which chunks are present
pub struct RegionFileInfo {
    /// The path to the region file.
    pub path: PathBuf,
    /// Metadata information about the region file.
    pub metadata: Metadata,
    /// The region file's header.
    pub header: RegionHeader,
    /// The bitmask that describes which chunks are present in the file.
    pub present_bits: RegionBitmask,
}

impl RegionFileInfo {

    // TODO: Better documentation.
    /// Gathers information about a region file at the given path.
    pub fn load<P: AsRef<Path>>(path: P) -> McResult<Self> {
        let file = File::open(path.as_ref())?;
        let metadata = std::fs::metadata(path.as_ref())?;
        let mut reader = BufReader::with_capacity(4096*2, file);
        let header = RegionHeader::read_from(&mut reader)?;
        let mut bits = RegionBitmask::new();
        for i in 0..1024 {
            if !header.sectors[i].is_empty() {
                reader.seek(header.sectors[i].seeker())?;
                let length = u32::read_from(&mut reader)?;
                if length != 0 {
                    bits.set(i, true);
                }
            }
        }
        Ok(Self {
            path: PathBuf::from(path.as_ref()),
            metadata,
            header,
            present_bits: bits,
        })
    }

    /// Opens the file that this RegionFileInfo points to.
    pub fn open(&self) -> McResult<File> {
        Ok(File::open(&self.path)?)
    }

    /// The path that this RegionFileInfo points to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the file's metadata.
    pub fn metadata(&self) -> std::fs::Metadata {
        self.metadata.clone()
    }

    /// Get a RegionSector for the provided coordinate.
    pub fn get_offset<C: Into<RegionCoord>>(&self, coord: C) -> RegionSector {
        self.header.sectors[coord]
    }

    /// Get the Timestamp for the provided coordinate.
    pub fn get_timestamp<C: Into<RegionCoord>>(&self, coord: C) -> Timestamp {
        self.header.timestamps[coord]
    }

    /// Checks if the chunk exists in the region file.
    pub fn has_chunk<C: Into<RegionCoord>>(&self, coord: C) -> bool {
        self.present_bits.get(coord)
    }

    /// The time that the file was created.
    pub fn creation_time(&self) -> std::io::Result<std::time::SystemTime> {
        self.metadata.created()
    }

    /// The last modification time of this file.
    pub fn modified_time(&self) -> std::io::Result<std::time::SystemTime> {
        self.metadata.modified()
    }

    /// The last time this file was accessed. (This will probably end up being very
    /// recent since it was accessed for reading to load it.)
    pub fn accessed_time(&self) -> std::io::Result<std::time::SystemTime> {
        self.metadata.accessed()
    }

    /// Returns the size of the region file.
    pub fn size(&self) -> u64 {
        self.metadata.len()
    }

    /// Returns true if the region file has a size
    /// that is a multiple of 4KiB. Minecraft will
    /// consider the region to be corrupted
    /// otherwise.
    pub fn is_correct_size_multiple(&self) -> bool {
        is_multiple_of_4096(self.size())
    }

}

impl RegionBitmask {
    /// Creates a new bitmask with all bits set to off.
    pub fn new() -> Self {
        Self(
            Box::new([0; 32])
        )
    }
    
    /// Creates a new bitmask with all bits set to on.
    pub fn new_on() -> Self {
        Self(
            Box::new([u32::MAX; 32])
        )
    }

    pub fn get<C: Into<RegionCoord>>(&self, coord: C) -> bool {
        let coord: RegionCoord = coord.into();
        let index = coord.index();
        let sub_index = index.div_euclid(32);
        let bit_index = index.rem_euclid(32);
        self.0[sub_index].get_bit(bit_index)
    }

    pub fn set<C: Into<RegionCoord>>(&mut self, coord: C, on: bool) {
        let coord: RegionCoord = coord.into();
        let index = coord.index();
        let sub_index = index.div_euclid(32);
        let bit_index = index.rem_euclid(32);
        self.0[sub_index] = self.0[sub_index].set_bit(bit_index, on);
    }

    /// Clear all bits (Setting them to 0).
    pub fn clear(&mut self) {
        self.0.iter_mut().for_each(|value| {
            *value = 0;
        });
    }
}

impl Default for RegionBitmask {
    fn default() -> Self {
        Self::new()
    }
}

impl From<[[bool; 32]; 32]> for RegionBitmask {
    fn from(value: [[bool; 32]; 32]) -> Self {
        let mut mask = Self::new();
        for z in 0..32 {
            for x in 0..32 {
                mask.set((x, z), value[z][x]);
            }
        }
        mask
    }
}

impl From<&[[bool; 32]; 32]> for RegionBitmask {
    fn from(value: &[[bool; 32]; 32]) -> Self {
        let mut mask = Self::new();
        for z in 0..32 {
            for x in 0..32 {
                mask.set((x, z), value[z][x]);
            }
        }
        mask
    }
}
impl From<[u32; 32]> for RegionBitmask {
    fn from(value: [u32; 32]) -> Self {
        RegionBitmask(Box::new(value))
    }
}

impl From<[bool; 1024]> for RegionBitmask {
    fn from(value: [bool; 1024]) -> Self {
        let mut mask = RegionBitmask::new();
        value.into_iter()
            .enumerate()
            .for_each(|(index, on)| {
                mask.set(index, on)
            });
        mask
    }
}

impl From<&[bool; 1024]> for RegionBitmask {
    fn from(value: &[bool; 1024]) -> Self {
        let mut mask = RegionBitmask::new();
        value.iter()
            .enumerate()
            .for_each(|(index, &on)| {
                mask.set(index, on)
            });
        mask
    }
}

impl From<RegionBitmask> for [bool; 1024] {
    fn from(value: RegionBitmask) -> Self {
        let mut bits = [false; 1024];
        bits.iter_mut()
            .enumerate()
            .for_each(|(index, bit)| {
                *bit = value.get(index);
            });
        bits
    }
}

impl From<RegionBitmask> for [u32; 32] {
    fn from(value: RegionBitmask) -> Self {
        *value.0
    }
}

impl From<&RegionBitmask> for [u32; 32] {
    fn from(value: &RegionBitmask) -> Self {
        let mut bits = [0u32; 32];
        bits.iter_mut()
            .enumerate()
            .for_each(|(i, bitmask)| {
                *bitmask = value.0[i];
            });
        bits
    }
}