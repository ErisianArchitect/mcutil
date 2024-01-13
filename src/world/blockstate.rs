use std::fmt::Display;

use sorted_vec::SortedVec;

use crate::{nbt::{tag::*, Map}, McResult, McError};

/// Create a [BlockState].
/// 
/// Syntax:
/// ```no_run,rust
/// blockstate!(air)
/// // Becomes
/// BlockState::new("minecraft:air", BlockProperties::none())
/// 
/// blockstate!(namespace:tile[prop1="string_literal", prop2=identifier, prop3=10])
/// // Becomes
/// BlockState::new("namespace:tile", BlockProperties::from([
/// 	("prop1".to_owned(), "string_literal".to_owned()),
/// 	("prop2".to_owned(), "identifier".to_owned()),
/// 	("prop3".to_owned(), "10".to_owned())
/// ]))
/// ```
#[macro_export]
macro_rules! blockstate {
	($id:ident) => {
		// We assume 'minecraft' namespace by default.
		blockstate!(minecraft:$id)
	};
	($id:ident [ $($name:tt = $value:tt),+$(,)? ]) => {
		blockstate!(minecraft:$id[ $($name = $value),+ ])
	};
	($namespace:ident:$id:ident) => {
		$crate::world::blockstate::BlockState::new(
			format!("{}:{}", stringify!($namespace), stringify!($id)),
			$crate::world::blockstate::BlockProperties::none()
		)
	};
	($namespace:ident:$id:ident [ $($name:tt = $value:tt),+$(,)? ]) => {
		$crate::world::blockstate::BlockState::new(
			format!("{}:{}", stringify!($namespace), stringify!($id)),
			$crate::world::blockstate::BlockProperties::from([
				$(
					(
						$crate::blockstate!(@decode_token; $name),
						$crate::blockstate!(@decode_token; $value)
					),
				)+
			])
		)
	};
	(@decode_token; $value:literal) => {
		$value.to_string()
	};
	(@decode_token; $value:ident) => {
		stringify!($value).to_owned()
	};
	(@decode_token; $value:expr) => {
		($value).to_string()
	};
}

pub use crate::blockstate;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct BlockProperty {
	pub name: String,
	pub value: String,
}

impl BlockProperty {
	pub fn new<S1: AsRef<str>, S2: AsRef<str>>(name: S1, value: S2) -> Self {
		Self {
			name: name.as_ref().to_owned(),
			value: value.as_ref().to_owned(),
		}
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn value(&self) -> &str {
		&self.value
	}
}

impl PartialOrd for BlockProperty {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (&self.name, &self.value).partial_cmp(&(&other.name, &other.value))
    }
}

impl Ord for BlockProperty {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(&self.name, &self.value).cmp(&(&other.name, &other.value))
	}
}

impl<S1: AsRef<str>, S2: AsRef<str>> From<(S1, S2)> for BlockProperty {
	fn from(value: (S1, S2)) -> Self {
		BlockProperty {
			name: value.0.as_ref().to_owned(),
			value: value.1.as_ref().to_owned(),
		}
	}
}

impl Into<(String, String)> for BlockProperty {
	fn into(self) -> (String, String) {
		(self.name, self.value)
	}
}

#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct BlockProperties {
	pub properties: Option<SortedVec<BlockProperty>>
}

impl BlockProperties {
	pub fn none() -> Self {
		Self {
			properties: None
		}
	}

	pub fn is_empty(&self) -> bool {
		if let Some(properties) = &self.properties {
			properties.is_empty()
		} else {
			true
		}
	}

	pub fn properties(&self) -> Option<&[BlockProperty]> {
		if let Some(props) = &self.properties {
			Some(props.as_slice())
		} else {
			None
		}
	}
}

impl<T: Into<BlockProperty>, It: IntoIterator<Item = T>> From<It> for BlockProperties {
	fn from(value: It) -> Self {
		let properties = value.into_iter()
			.map(T::into)
			.collect::<Vec<BlockProperty>>();
		Self {
			properties: Some(properties.into())
		}
	}
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct BlockState {
	name: String,
	properties: BlockProperties,
}

impl BlockState {
	pub fn new<S: AsRef<str>, P: Into<BlockProperties>>(name: S, properties: P) -> Self {
		Self {
			name: name.as_ref().to_owned(),
			properties: properties.into(),
		}
	}

	pub fn air() -> Self {
		blockstate!(air)
		// Self::new("minecraft:air", BlockProperties::none())
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn properties(&self) -> Option<&[BlockProperty]> {
		self.properties.properties()
	}

	pub fn to_map(self) -> Map {
		let mut props = Map::new();
		if let Some(properties) = self.properties.properties {
			props.extend(properties.iter().map(|prop| {
				(prop.name.clone(), Tag::String(prop.value.clone()))
			}));
		}
		Map::from([
			("Name".to_owned(), Tag::String(self.name.clone())),
			("Properties".to_owned(), Tag::Compound(props)),
		])
	}

	pub fn try_from_map(map: &Map) -> McResult<Self> {
		let Some(Tag::String(name)) = map.get("Name") else {
			return Err(crate::McError::NbtDecodeError);
		};
		let properties = if let Some(props_some) = map.get("Properties") {
			if let Tag::Compound(properties) = props_some {
				BlockProperties::from(properties.iter().map(|(key, value)| {
					if let Tag::String(value) = value {
						Ok((key.clone(), value.clone()))
					} else {
						Err(McError::NbtDecodeError)
					}
				}).collect::<McResult<Vec<(String, String)>>>()?)
			} else {
				return Err(McError::NbtDecodeError);
			}
		} else {
			BlockProperties::none()
		};
		Ok(Self::new(name, properties))
	}
}

// Allows for creating BlockState from strings.
impl<S: AsRef<str>> From<S> for BlockState {
	fn from(value: S) -> Self {
		BlockState::new(value, BlockProperties::none())
	}
}

impl EncodeNbt for BlockState {
	fn encode_nbt(self) -> Tag {
		let map = self.to_map();
		Tag::Compound(map)
	}
}

impl Display for BlockState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", &self.name)?;
		if !self.properties.is_empty() {
			write!(f, "{}", &self.properties)?;
		}
		Ok(())
	}
}

impl Display for BlockProperties {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(props) = &self.properties {
			write!(f, "[")?;
			let last = props.len() - 1;
			props.iter()
				.enumerate()
				.try_for_each(|(index, prop)| {
					write!(f, "{}={}", &prop.name, &prop.value)?;
					if index < last {
						write!(f, ", ")?;
					}
					Ok(())
				})?;
			write!(f, "]")?;
			Ok(())
		} else {
			write!(f, "[]")?;
			Ok(())
		}
	}
}