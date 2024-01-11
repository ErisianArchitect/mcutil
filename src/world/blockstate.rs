use sorted_vec::SortedVec;

use crate::{nbt::{tag::*, Map}, McResult, McError};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct BlockProperty {
	pub name: String,
	pub value: String,
}

impl BlockProperty {
	pub fn new(name: String, value: String) -> Self {
		Self {
			name,
			value,
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
		Self::new("minecraft:air", BlockProperties::none())
	}

	pub fn name(&self) -> &str {
		return &self.name
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