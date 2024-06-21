// TODO: Remove this when you no longer want to silence the warnings.

use std::{
    fs::File, io::{
        BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Take, Write
    }, path::{
        Path,
        PathBuf,
    }
};

use flate2::{
    write::ZlibEncoder,
    read::{
        GzDecoder,
        ZlibDecoder,
    },
    Compression,
};

use crate::{
    McResult, McError,
    ioext::*,
};

use super::{
    prelude::*,
    {required_sectors, pad_size},
};

pub trait RegionManager {
    type Sector;
    //	write_data
    //	write_timestamped
    //	delete_data
    //	read_data
    //	optimize
    fn read<'a, C: Into<RegionCoord>, F: FnMut(MultiDecoder<'a>) -> McResult<()>>(self, coord: C, read: F) -> McResult<()>;
    fn read_data<C: Into<RegionCoord>, T: Readable>(self, coord: C) -> McResult<T>;
    fn write_data<C: Into<RegionCoord>, T: Writable>(self, coord: C, value: &T) -> McResult<Self::Sector>;
    fn write<C: Into<RegionCoord>, F: Fn()>(self, coord: C, write: F) -> McResult<Self::Sector>;
    fn write_timestamped<C: Into<RegionCoord>, T: Writable, Ts: Into<Timestamp>>(self, coord: C, value: &T, timestamp: Ts) -> McResult<Self::Sector>;
    fn delete_data<C: Into<RegionCoord>>(self, coord: C) -> McResult<Self::Sector>;
}

/// A construct for working with RegionFiles.
/// Allows for reading and writing data from a RegionFile.
pub struct RegionFile {
    header: RegionHeader,
    sector_manager: SectorManager,
    /// This file handle is for both reading and writing.
    file_handle: File,
    path: PathBuf,
    /// Because the write size of a value sometimes can't quite be known until
    /// after it has been written, it will be helpful to have a buffer to write
    /// to before writing to the file. This will allow us to know exactly how
    /// many 4KiB blocks are needed to write this data so that a sector can be
    /// allocated.
    write_buf: Cursor<Vec<u8>>,
    pub compression: Compression,
}

pub enum MultiDecoder<'a> {
    GZip(GzDecoder<Take<BufReader<&'a mut File>>>),
    ZLib(ZlibDecoder<Take<BufReader<&'a mut File>>>),
    Uncompressed(Take<BufReader<&'a mut File>>),
}

impl<'a> Read for MultiDecoder<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            MultiDecoder::GZip(reader) => reader.read(buf),
            MultiDecoder::ZLib(reader) => reader.read(buf),
            MultiDecoder::Uncompressed(reader) => reader.read(buf),
        }
    }
}

impl RegionFile {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn sectors(&self) -> &SectorTable {
        &self.header.sectors
    }

    pub fn timestamps(&self) -> &TimestampTable {
        &self.header.timestamps
    }

    pub fn header(&self) -> &RegionHeader {
        &self.header
    }

    pub fn get_sector<C: Into<RegionCoord>>(&self, coord: C) -> RegionSector {
        let coord: RegionCoord = coord.into();
        self.header.sectors[coord.index()]
    }
    
    pub fn get_timestamp<C: Into<RegionCoord>>(&self, coord: C) -> Timestamp {
        let coord: RegionCoord = coord.into();
        self.header.timestamps[coord.index()]
    }

    // I made RegionFile.compression public, so this isn't likely needed, but it may be useful.
    pub fn compression(&self) -> Compression {
        self.compression
    }

    // I made RegionFile.compression public, so this isn't likely needed, but it may be useful.
    pub fn set_compression(&mut self, compression: Compression) {
        self.compression = compression;
    }

    /// Attempts to open a Minecraft region file at the given path, returning an error if it is not found.
    pub fn open<P: AsRef<Path>>(path: P) -> McResult<Self> {
        let path = path.as_ref();
        let mut file_handle = File::options()
            // Need to be able to read and write.
            .read(true).write(true)
            .open(path)?;
        // Seek to the end to figure out the size of the file.
        file_handle.seek(SeekFrom::End(0))?;
        let file_size = file_handle.stream_position()?;
        if file_size < 8192 {
            // The size was too small to hold the header, which means it isn't
            // a valid region file.
            return Err(McError::InvalidRegionFile);
        }
        file_handle.seek(SeekFrom::Start(0))?;
        let header = {				
            let mut temp_reader = BufReader::new((&mut file_handle).take(4096*2));
            RegionHeader::read_from(&mut temp_reader)?
        };
        let sector_manager = SectorManager::from(header.sectors.iter());
        Ok(Self {
            file_handle,
            header,
            compression: Compression::best(),
            sector_manager,
            write_buf: Cursor::new(Vec::with_capacity(4096*2)),
            path: path.to_owned(),
        })
    }

    /// Attempts to create a new Minecraft region file at the given path, returning an error if it already exists.
    pub fn create<P: AsRef<Path>>(path: P) -> McResult<Self> {
        let path = path.as_ref();
        // Create region file with empty header.
        let mut file_handle = File::options()
            // Need to be able to read and write.
            .read(true).write(true)
            // The file doesn't exist, so we need to create it.
            .create_new(true)
            .open(path)?;
        // Write an empty header since this is a new file.
        file_handle.write_zeroes(4096*2)?;
        Ok(Self {
            file_handle,
            compression: Compression::best(),
            write_buf: Cursor::new(Vec::with_capacity(4096*2)),
            header: RegionHeader::default(),
            sector_manager: SectorManager::new(),
            path: path.to_owned(),
        })
    }

    /// Creates a new [RegionFile] object, opening or creating a Minecraft region file at the given path.
    pub fn open_or_create<P: AsRef<Path>>(path: P) -> McResult<Self> {
        let path = path.as_ref();
        if path.is_file() {
            Self::open(path)
        } else {
            Self::create(path)
        }
    }

    pub fn write_with_utcnow<C: Into<RegionCoord>, F: FnMut(&mut ZlibEncoder<&mut Cursor<Vec<u8>>>) -> McResult<()>>(&mut self, coord: C, mut write: F) -> McResult<RegionSector> {
        self.write_timestamped(coord, Timestamp::utc_now(), |writer| {
            write(writer)
        })
    }

    /// Writes data to the region file with the `utc_now` timestamp
    ///  and returns the [RegionSector] where it was written.
    pub fn write_data_with_utcnow<C: Into<RegionCoord>, T: Writable>(&mut self, coord: C, value: &T) -> McResult<RegionSector> {
        self.write_data_timestamped(coord, value, Timestamp::utc_now())
    }

    pub fn read<'a, C: Into<RegionCoord>, R, F: FnMut(MultiDecoder<'a>) -> McResult<R>>(&'a mut self, coord: C, mut read: F) -> McResult<R> {
        let coord: RegionCoord = coord.into();
        let sector = self.header.sectors[coord.index()];
        if sector.is_empty() {
            return Err(McError::RegionDataNotFound);
        }
        let mut reader = BufReader::new(&mut self.file_handle);
        reader.seek(SeekFrom::Start(sector.offset()))?;
        let length: u32 = reader.read_value()?;
        if length == 0 {
            return Err(McError::RegionDataNotFound);
        }
        let scheme: CompressionScheme = reader.read_value()?;
        match scheme {
            CompressionScheme::GZip => {
                // Subtract 1 from length because the compression scheme is included in the length.
                let decoder = GzDecoder::new(reader.take((length - 1) as u64));
                let multi = MultiDecoder::GZip(decoder);
                read(multi)
            },
            CompressionScheme::ZLib => {
                let decoder = ZlibDecoder::new(reader.take((length - 1) as u64));
                let multi = MultiDecoder::ZLib(decoder);
                read(multi)
            },
            CompressionScheme::Uncompressed => {
                let multi = MultiDecoder::Uncompressed(reader.take((length - 1) as u64));
                read(multi)
            },
        }
    }

    pub fn read_data<C: Into<RegionCoord>, T: Readable>(&mut self, coord: C) -> McResult<T> {
        self.read(coord, |mut decoder| {
            T::read_from(&mut decoder)
        })
    }

    pub fn write<'a, C: Into<RegionCoord>, F: FnMut(&mut ZlibEncoder<&mut Cursor<Vec<u8>>>) -> McResult<()>>(&'a mut self, coord: C, mut write: F) -> McResult<RegionSector> {
        let coord: RegionCoord = coord.into();
        // Clear the write_buf to prepare it for writing.
        self.write_buf.get_mut().clear();
        // Gotta write 5 bytes to the buffer so that there's room for the length and the compression scheme.
        // To kill two birds with one stone, I'll write all 2s so that I don't have to go back and write the
        // compression scheme after writing the length.
        self.write_buf.write_all(&[2u8; 5])?;
        // Now we'll write the data to the compressor.
        let mut encoder = ZlibEncoder::new(&mut self.write_buf, self.compression);
        // value.write_to(&mut encoder)?;
        write(&mut encoder)?;
        encoder.finish()?;
        // Get the length of the written data by getting the length of the buffer and subtracting 5 (for
        // the bytes that were pre-written in a previous step)
        let length = self.write_buf.get_ref().len() - 5;
        // Get sectors required to accomodate the buffer.
        // + 5 because you need to add the (length_bytes + CompressionScheme)
        let required_sectors = required_sectors((length + 5) as u32);
        // If there is an overflow, return an error because there's no way to write it to the file.
        if required_sectors > 255 {
            return Err(McError::RegionDataTooLarge);
        }
        // Write pad zeroes
        // + 5 because you need to add the (length_bytes + CompressionScheme)
        let pad_bytes = pad_size((length + 5) as u64);
        self.write_buf.write_zeroes(pad_bytes)?;
        // Seek back to the beginning to write the length.
        self.write_buf.set_position(0);
        // Add 1 to the length because the specification requires that the compression scheme is included in the length for some reason.
        self.write_buf.write_value((length + 1) as u32)?;
        // Allocation
        let old_sector = self.header.sectors[coord.index()];
        let new_sector = self.sector_manager.reallocate_err(old_sector, required_sectors as u8)?;
        self.header.sectors[coord.index()] = new_sector;
        // Writing to file
        let mut writer = BufWriter::new(&mut self.file_handle);
        writer.seek(SeekFrom::Start(new_sector.offset()))?;
        writer.write_all(self.write_buf.get_ref().as_slice())?;
        writer.seek(coord.sector_table_offset())?;
        writer.write_value(new_sector)?;
        writer.flush()?;
        Ok(new_sector)
    }

    pub fn write_data<C: Into<RegionCoord>, T: Writable>(&mut self, coord: C, value: &T) -> McResult<RegionSector> {
        self.write(coord, |mut encoder| {
            value.write_to(&mut encoder)?;
            Ok(())
        })
    }

    pub fn write_timestamped<'a, C: Into<RegionCoord>, Ts: Into<Timestamp>, F: FnMut(&mut ZlibEncoder<&mut Cursor<Vec<u8>>>) -> McResult<()>>(&mut self, coord: C, timestamp: Ts, write: F) -> McResult<RegionSector> {
        let coord: RegionCoord = coord.into();
        // let allocation = self.write_data(coord, value)?;
        let allocation = self.write(coord, write)?;
        let timestamp: Timestamp = timestamp.into();
        self.header.timestamps[coord.index()] = timestamp;
        // Write the timestamp to the file.
        let mut writer = BufWriter::new(&mut self.file_handle);
        writer.seek(coord.timestamp_table_offset())?;
        writer.write_value(timestamp)?;
        // I'm pretty sure that flush() doesn't do anything, but I'll put it here just in case.
        writer.flush()?;
        Ok(allocation)
    }

    pub fn write_data_timestamped<C: Into<RegionCoord>, T: Writable, Ts: Into<Timestamp>>(&mut self, coord: C, value: &T, timestamp: Ts) -> McResult<RegionSector> {
        self.write_timestamped(coord, timestamp, |writer| {
            value.write_to(writer)?;
            Ok(())
        })
    }

    pub fn delete_data<C: Into<RegionCoord>>(&mut self, coord: C) -> McResult<RegionSector> {
        let coord: RegionCoord = coord.into();
        let sector = self.header.sectors[coord.index()];
        if sector.is_empty() {
            return Ok(sector);
        }
        self.sector_manager.deallocate(sector);
        self.header.sectors[coord.index()] = RegionSector::default();
        self.header.timestamps[coord.index()] = Timestamp::default();
        // Clear the sector from the sector table
        let mut writer = BufWriter::new(&mut self.file_handle);
        writer.seek(coord.sector_table_offset())?;
        writer.write_zeroes(4)?;
        // Clear the timestamp from the timestamp table.
        writer.seek(coord.timestamp_table_offset())?;
        writer.write_zeroes(4)?;
        writer.flush()?;
        Ok(sector)
    }

    ///	Removes all unused sectors from the region file, rearranging it so that it is optimized.
    ///	This is a costly operation, so it should only be performed when a region file reaches a certain threshhold 
    ///	of complexity.
    pub fn optimize(&mut self) -> McResult<()> {
        //	There is likely an algorithm that can be invented to optimize the file, and as a consequence
        //	there should be an algorithm that can measure the complexity for solving with the first algorithm.
        //	Therefore it should be possible to pass a sector table into the complexity measuring algorithm to measure the cost
        //	of optimization.
        //		optimization_cost(sector_table)
        
        // I had an idea for how I might be able to write the optimization algorithm.
        // What I can do is I can get information about the sectors:
        // I would need the gaps, then the upper sectors that need to be moved around to fill in the gaps.
        

        todo!()
    }
}