
use std::io::{Read, Write};
use chrono::{NaiveDateTime, DateTime, Utc, TimeZone};
use crate::{
	McResult,
	for_each_int_type,
	ioext::*,
};

/// A 32-bit Unix timestamp.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub struct Timestamp(u32);

impl Timestamp {
	pub fn to_datetime(&self) -> Option<DateTime<Utc>> {
		DateTime::<Utc>::try_from(*self).ok()
	}

	/// Get a [Timestamp] for the current time (in Utc).
	pub fn utc_now() -> Timestamp {
		Timestamp(
			Utc::now().timestamp() as u32
		)
	}
}

macro_rules! __timestamp_impls {
	($type:ty) => {
		impl From<$type> for Timestamp {
			fn from(value: $type) -> Self {
				Self(value as u32)
			}
		}

		impl From<Timestamp> for $type {
			fn from(value: Timestamp) -> Self {
				value.0 as $type
			}
		}
	};
}

for_each_int_type!(__timestamp_impls);

impl<T: Into<Timestamp> + Copy> From<&T> for Timestamp {
    fn from(value: &T) -> Self {
        T::into(*value)
    }
}

impl Readable for Timestamp {
	fn read_from<R: Read>(reader: &mut R) -> McResult<Self> {
		Ok(Self(reader.read_value()?))
	}
}

impl Writable for Timestamp {
	fn write_to<W: Write>(&self, writer: &mut W) -> McResult<usize> {
		writer.write_value(self.0)
	}
}

impl From<DateTime<Utc>> for Timestamp {
	fn from(value: DateTime<Utc>) -> Self {
		Timestamp(value.timestamp() as u32)
	}
}

impl TryFrom<Timestamp> for DateTime<Utc> {
	type Error = ();

	fn try_from(value: Timestamp) -> Result<Self, Self::Error> {
		let naive = NaiveDateTime::from_timestamp_opt(value.0 as i64, 0);
		if let Some(naive) = naive {
			Ok(Utc.from_utc_datetime(&naive))
		} else {
			Err(())
		}
	}
}