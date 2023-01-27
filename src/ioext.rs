use std::io::{
	Write, Read,
	Seek, SeekFrom,
};

/// For types that can be written to a writer.
pub trait Writable {
	fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize,crate::McError>;
}

/// For types that can be read from a reader.
pub trait Readable: Sized {
	fn read_from<R: Read>(reader: &mut R) -> Result<Self,crate::McError>;
}

/// For types that represent a seekable file offset.
pub trait Seekable: Sized {
	fn seek_to<S: Seek>(&self, seeker: &mut S) -> Result<u64,crate::McError> {
		Ok(seeker.seek(self.seeker())?)
	}

	fn seeker(&self) -> SeekFrom;
}

pub trait WriteExt: Write + Sized {
	fn write_value<T: Writable>(&mut self, value: T) -> Result<usize,crate::McError>;
}

pub trait ReadExt: Read + Sized {
	fn read_value<T: Readable>(&mut self) -> Result<T,crate::McError>;
}

impl<W: Write + Sized> WriteExt for W {
    fn write_value<T: Writable>(&mut self, value: T) -> Result<usize,crate::McError> {
		value.write_to(self)
	}
}

impl<R: Read + Sized> ReadExt for R {
    fn read_value<T: Readable>(&mut self) -> Result<T,crate::McError> {
		T::read_from(self)
	}
}

pub trait SeekExt: Seek + Sized {
	fn seek_to<S: Seekable>(&mut self, seek_offset: &S) -> Result<u64,crate::McError>;

	fn seek_return(&mut self) -> Result<SeekFrom,crate::McError>;
}

impl<T: Seek + Sized> SeekExt for T {
    fn seek_to<S: Seekable>(&mut self, seek_offset: &S) -> Result<u64,crate::McError> {
		seek_offset.seek_to(self)
	}

    fn seek_return(&mut self) -> Result<SeekFrom,crate::McError> {
		Ok(SeekFrom::Start(self.stream_position()?))
	}
}