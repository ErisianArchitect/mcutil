use std::io::{
	Read, Write,
	BufWriter,
	Seek, SeekFrom,
};

use crate::{
	ioext::*,
	McResult, McError,
};

use super::{
	compressionscheme::*,
	header::*,
	coord::*,
	sector::*,
	timestamp::*,
	is_multiple_of_4096,
	required_sectors,
	pad_size,
};

use flate2::{
	Compression,
	write::ZlibEncoder,
};

/// An abstraction for writing Region files.
/// You open a region file, pass the writer over to this
/// struct, then you write whatever offsets/timestamps/chunks
/// that you need to write. When you're done writing, you can
/// call `.finish()` to take the writer back.
pub struct RegionWriter<W: Write + Seek> {
	/// The writer that this [RegionWriter] is bound to.
	writer: W,
}

impl<W: Write + Seek> Write for RegionWriter<W> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.writer.write(buf)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		self.writer.flush()
	}
}

impl<W: Write + Seek> Seek for RegionWriter<W> {
	fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
		self.writer.seek(pos)
	}
}

impl<W: Write + Seek> RegionWriter<W> {
	pub fn new(writer: W) -> Self {
		Self {
			writer,
		}
	}

	pub fn with_capacity(capacity: usize, inner: W) -> RegionWriter<BufWriter<W>> {
		RegionWriter::<BufWriter<W>>{
			writer: BufWriter::with_capacity(capacity, inner)
		}
	}

	/// Returns the 4KiB offset of the sector that the writer is writing to.
	/// This is NOT the stream position.
	pub fn sector_offset(&mut self) -> McResult<u32> {
		Ok((self.writer.stream_position()? as u32).overflowing_shr(12).0)
	}

	/// This function writes an 8KiB zeroed header to the writer.
	/// In order to reduce system calls and whatever, this function
	/// assumes that you are already at the start of the file.
	/// This is a function that you would call while building a new
	/// region file.
	pub fn write_empty_header(&mut self) -> McResult<u64> {
		Ok(self.writer.write_zeroes(4096*2)?)
	}

	/// Seeks to the beginning of the stream and writes a header.
	pub fn write_header(&mut self, header: &RegionHeader) -> McResult<()> {
		let ret = self.writer.seek_return()?;
		self.seek(SeekFrom::Start(0))?;
		header.write_to(&mut self.writer)?;
		self.writer.seek(ret)?;
		Ok(())
	}

	/// Seeks to the table and writes it to the file.
	pub fn write_sector_table(&mut self, table: &SectorTable) -> McResult<()> {
		let ret = self.writer.seek_return()?;
		self.seek(SectorTable::seeker())?;
		table.write_to(&mut self.writer)?;
		self.writer.seek(ret)?;
		Ok(())
	}

	/// Seeks to the table and writes it to the file.
	pub fn write_timestamp_table(&mut self, table: &TimestampTable) -> McResult<()> {
		let ret = self.writer.seek_return()?;
		self.seek(TimestampTable::seeker())?;
		table.write_to(&mut self.writer)?;
		self.writer.seek(ret)?;
		Ok(())
	}

	/// Write an offset to the offset table of the Region file.
	pub fn write_offset_at_coord<C: Into<RegionCoord>,O: Into<RegionSector>>(&mut self, coord: C, offset: O) -> McResult<usize> {
		let coord: RegionCoord = coord.into();
		let oldpos = self.writer.seek_return()?;
		self.writer.seek(coord.sector_table_offset())?;
		let offset: RegionSector = offset.into();
		let result = self.writer.write_value(offset);
		// Return to the original seek position.
		self.writer.seek(oldpos)?;
		result
	}

	/// Write a [Timestamp] to the [Timestamp] table of the Region file.
	pub fn write_timestamp_at_coord<C: Into<RegionCoord>, O: Into<Timestamp>>(&mut self, coord: C, timestamp: O) -> McResult<usize> {
		let coord: RegionCoord = coord.into();
		let oldpos = self.writer.seek_return()?;
		self.writer.seek(coord.timestamp_table_offset())?;
		let timestamp: Timestamp = timestamp.into();
		let result = self.writer.write_value(timestamp);
		// Return to the original seek position.
		self.writer.seek(oldpos)?;
		result
	}

	/// Write data to Region File, then write the sector that data
	/// was written to into the sector table.
	/// `compression_level` must be a value from 0 to 9, where 0 means
	/// "no compression" and 9 means "take as along as you like" (best compression)
	pub fn write_data_at_coord<T: Writable,C: Into<RegionCoord>>(
		&mut self,
		compression: Compression,
		coord: C,
		data: &T,
	) -> McResult<RegionSector> {
		let sector = self.write_data_to_sector(compression, data)?;
		self.write_offset_at_coord(coord, sector)?;
		Ok(sector)
	}

	/// Write a chunk to the region file starting at the current
	/// position in the file. After writing the chunk, pad bytes will 
	/// be written to ensure that the region file is a multiple of 4096
	/// bytes.
	/// This function does not write anything to the header. 
	/// Returns the RegionSector that was written to.
	pub fn write_data_to_sector<T: Writable>(
		&mut self,
		compression: Compression,
		data: &T
	) -> McResult<RegionSector> {
		// TODO: Remove the fancy box-drawing characters to make it easier for screen readers.
		/*	╭────────────────────────────────────────────────────────────────────────────────────────────────╮
			│ Instead of using an in-memory buffer to do compression, I'll write                             │
			│ directly to the writer. This should speed things up a bit, and reduce                          │
			│ resource load.                                                                                 │
			│ Steps:                                                                                         │
			│ 01.) Retrieve starting position in stream (on 4KiB boundary).                                  │
			│ 02.) Check that position is on 4KiB boundary.                                                  │
			│ 03.) Move the stream forward 4 bytes.                                                          │
			│ 04.) Write the compression scheme (2 for ZLib) .                                               │
			│ 05.) Create ZLib encoder from writer.                                                          │
			│ 06.) Write the data.                                                                           │
			│ 07.) Release the ZLib encoder.                                                                 │
			│ 08.) Get the final offset.                                                                     │
			│ 09.) Subtract starting offset from final offset then add 4 (for the length) to get the length. │
			│ 10.) Write pad zeroes.                                                                         │
			│ 11.) Store writer stream position.                                                             │
			│ 12.) Return to the offset from Step 01.).                                                      │
			│ 13.) Write length.                                                                             │
			│ 14.) Return writer to stream position in Step 11.).                                            │
			╰────────────────────────────────────────────────────────────────────────────────────────────────╯*/
		// Step 01.)
		let sector_offset = self.writer.stream_position()?;
		// Step 02.)
		if !is_multiple_of_4096(sector_offset) {
			return Err(McError::StreamSectorBoundaryError);
		}
		// Step 03.)
		self.writer.write(&[0u8; 4])?;
		// Step 04.)
		self.writer.write_value(CompressionScheme::ZLib)?;
		// Step 05.)
		let mut compressor = ZlibEncoder::new(
			&mut self.writer,
			compression
		);
		// Step 06.)
		data.write_to(&mut compressor)?;
		// Step 07.)
		compressor.finish()?;
		// Step 08.)
		let final_offset: u64 = self.writer.stream_position()?;
		// Step 09.)
		let length: u64 = (final_offset - sector_offset) - 4;
		// Step 10.)
		let padsize = pad_size(length + 4);
		self.writer.write_zeroes(padsize)?;
		// Step 11.)
		let return_position = self.writer.seek_return()?;
		// Step 12.)
		self.writer.seek(SeekFrom::Start(sector_offset))?;
		// Step 13.)
		self.writer.write_value(length as u32)?;
		// Step 14.)
		self.writer.seek(return_position)?;
		let length = length as u32;
		Ok(RegionSector::new(
			// Shifting right 12 bits is a shortcut to get the 4KiB sector offset. This is done because sector_offset comes from stream_position
			sector_offset.overflowing_shr(12).0 as u32,
			// add 4 to the length because you have to include the 4 bytes for the length value.
			required_sectors(length + 4) as u8
		))
	}

	/// Copies a chunk from a reader into this writer.
	/// This function assumes that the given reader is already positioned
	/// to the beginning of the sector that you would like to copy from.
	/// 
	/// For a refresher on region file format, each sector begins with a
	/// 32-bit unsigned big-endian length value, which represents the
	/// length in bytes that the sector data occupies. This length also
	/// includes a single byte for the compression scheme (which is 
	/// irrellevant for copying).
	/// This function will read that length, then it will copy the sector
	/// data over to the writer. If the length is zero, nothing is copied
	/// and the value returned is an empty RegionSector.
	pub fn copy_chunk_from<R: Read>(&mut self, reader: &mut R) -> McResult<RegionSector> {
		if !is_multiple_of_4096(self.stream_position()?) {
			return Err(McError::StreamSectorBoundaryError);
		}
		let sector_offset = self.sector_offset()?;
		let mut length_buffer = [0u8; 4];
		reader.read_exact(&mut length_buffer)?;
		let length = u32::from_be_bytes(length_buffer.clone());
		// The length is zero means that there isn't any data in this
		// sector, but the sector is still being used. That means it's
		// a wasted sector. This can be fixed by simply not writing
		// anything to the writer and returning an empty RegionSector
		// to tell anything upstream that nothing was written.
		if length == 0  {
			return Err(McError::ChunkNotFound);
		}
		// Copy the length to the writer. Very important step.
		self.write_all(&length_buffer)?;
		copy_bytes(reader, &mut self.writer, length as u64)?;
		// The padsize is the number of bytes required to to put
		// the writer on a 4KiB boundary. You have to add 4 because you need
		// to include the 4 bytes for the length.
		let padsize = pad_size((length + 4) as u64);
		self.writer.write_zeroes(padsize)?;
		Ok(RegionSector::new(
			sector_offset, // DO NOT SHIFT sector_offset!! Just because it's done above doesn't mean it needs to here.
			// + 4 to include the 4 bytes holding the length.
			required_sectors(length + 4) as u8
		))
	}

	/// Returns the inner writer.
	pub fn finish(self) -> W {
		self.writer
	}
}