


use thiserror::Error;

/// The master error type.
#[derive(Debug, Error)]
pub enum McError {
	#[error("IO Error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("Chunk not found.")]
	ChunkNotFound,
	#[error("Invalid Compression value: {0}")]
	InvalidCompressionScheme(u8),
	#[error("Out of range error.")]
	OutOfRange,
	#[error("Failed to convert to UTF-8 string.")]
	FromUtf8Error(#[from] std::string::FromUtf8Error),
	#[error("Unsupported Tag ID: {0}")]
	UnsupportedTagId(u8),
	#[error("Encountered the End Tag ID marker.")]
	EndTagMarker,
	#[error("Attempted to save two chunks to the same location.")]
	DuplicateChunk,
	#[error("{0}")]
	Custom(String),
}

impl McError {
	
	pub fn range_check<T, R>(value: T, range: R) -> Result<(),McError>
	where
	T: PartialOrd + Sized,
	R: std::ops::RangeBounds<T> {
		if range.contains(&value) {
			Ok(())
		} else {
			Err(McError::OutOfRange)
		}
	}

	pub fn custom<T, S: AsRef<str>>(msg: S) -> Result<T,Self> {
		Err(McError::Custom(msg.as_ref().to_owned()))
	}
}