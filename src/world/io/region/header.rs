use crate::{
    ioext::*,
    McResult,
};

use super::sector::*;
use super::timestamp::*;
use super::coord::*;

use std::{
    fmt::Debug,
    io::{
        Read, Write,
        SeekFrom,
    }, 
    ops::{
        Index, IndexMut,
    },
};

/// You really don't need to worry about this.
/// This trait defines the offset in a file where
/// a table can be found for a specific type.
/// So if you have a Timestamp type, you can define
/// the offset of that type to make a RegionTable
/// with that type.
/// This trait is meant to be defined for [RegionSector] and [Timestamp]
pub trait RegionTableItem {
    /// The offset in the file that this type's table begins.
    const OFFSET: u64;
}

impl RegionTableItem for RegionSector {
    // Determines the offset of the table for the RegionSector type.
    const OFFSET: u64 = 0;
}

impl RegionTableItem for Timestamp {
    // Determines the offset of the table for the Timestamp type.
    const OFFSET: u64 = 4096;
}

/// A table of 1024 elements that contain information related to
/// a Minecraft chunk within a Region file.
#[derive(Debug, Clone)]
pub struct RegionTable<T: RegionTableItem>(Box<[T; 1024]>);

/// A table of 1024 [RegionSector] elements for each potential chunk in
/// a 32x32 chunk region file.
pub type SectorTable = RegionTable<RegionSector>;

/// A table of 1024 [Timestamp] elements for each potential chunk in a
/// 32x32 chunk region file.
pub type TimestampTable = RegionTable<Timestamp>;

/// The header at the beginning of every region file.
/// It contains 1024 [RegionSector] elements and 1024 [Timestamp] elements.
#[derive(Debug, Clone, Default)]
pub struct RegionHeader {
    /// The sector table, containing information about where chunks exist
    /// in the file.
    pub sectors: SectorTable,
    /// The timestamp table, which tells the last modification time for the chunk.
    pub timestamps: TimestampTable,
}

impl<T: RegionTableItem> RegionTable<T> {
    pub const OFFSET: u64 = T::OFFSET;

    /// Get the offset in the file where this table begins.
    pub fn offset() -> u64 {
        Self::OFFSET
    }

    /// Returns a [SeekFrom] value that will seek to the
    /// beginning of the table.
    pub const fn seeker() -> SeekFrom {
        SeekFrom::Start(Self::OFFSET)
    }

    /// Returns an iterator of the elements in the table.
    pub fn iter(&self) -> std::slice::Iter<T> {
        self.0.iter()
    }

    /// Returns a mutable iterator of the elements in the table.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
        self.0.iter_mut()
    }

    /// Return the inner `Box<[T; 1024]>` value.
    pub fn take_box(self) -> Box<[T; 1024]> {
        self.0
    }

    /// Return the inner array for this table.
    pub fn take_array(self) -> [T; 1024] {
        *self.0
    }
}

impl<T: RegionTableItem> IntoIterator for RegionTable<T> {
    type Item = T;
    type IntoIter = std::array::IntoIter<T, 1024>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T: Default + Copy + RegionTableItem> Default for RegionTable<T> {
    fn default() -> Self {
        Self(Box::new([T::default(); 1024]))
    }
}

impl<C: Into<RegionCoord>,T: RegionTableItem> Index<C> for RegionTable<T> {
    type Output = T;

    fn index(&self, index: C) -> &Self::Output {
        let coord: RegionCoord = index.into();
        &self.0[coord.index()]
    }
}

impl<C: Into<RegionCoord>,T: RegionTableItem> IndexMut<C> for RegionTable<T> {
    fn index_mut(&mut self, index: C) -> &mut Self::Output {
        let coord: RegionCoord = index.into();
        &mut self.0[coord.index()]
    }
}

impl<T: Readable + Debug + RegionTableItem> Readable for RegionTable<T> {
    fn read_from<R: Read>(reader: &mut R) -> McResult<Self> {
        let table: Box<[T; 1024]> = (0..1024).map(|_| {
            T::read_from(reader)
        }).collect::<McResult<Box<[T]>>>()?
        .try_into().unwrap();
        Ok(Self(table))
    }
}

impl<T: Writable + /* Debug + */ RegionTableItem + Sized> Writable for RegionTable<T> {
    fn write_to<W: Write>(&self, writer: &mut W) -> McResult<usize> {
        let mut write_size: usize = 0;
        for i in 0..1024 {
            write_size += self.0[i].write_to(writer)?;
        }
        Ok(write_size)
    }
}

impl<T: RegionTableItem> From<[T; 1024]> for RegionTable<T> {
    fn from(value: [T; 1024]) -> Self {
        Self(Box::new(value))
    }
}

impl<T: RegionTableItem> From<RegionTable<T>> for Box<[T; 1024]> {
    fn from(value: RegionTable<T>) -> Self {
        value.0
    }
}

impl Readable for RegionHeader {
    fn read_from<R: Read>(reader: &mut R) -> McResult<Self> {
        Ok(Self {
            sectors: SectorTable::read_from(reader)?,
            timestamps: TimestampTable::read_from(reader)?,
        })
    }
}

impl Writable for RegionHeader {
    fn write_to<W: Write>(&self, writer: &mut W) -> McResult<usize> {
        Ok(
            self.sectors.write_to(writer)? + self.timestamps.write_to(writer)?
        )
    }
}