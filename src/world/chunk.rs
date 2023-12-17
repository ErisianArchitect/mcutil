#![allow(unused)]

use std::collections::HashMap;
use std::default;

use super::blockstate::*;

use crate::McError;
use crate::McResult;
use crate::math::bit::BitLength;
use crate::nbt::*;
use crate::nbt::io::*;
use crate::nbt::tag::*;
use crate::nbt::tagtype::*;
use super::blockregistry::BlockRegistry;
use super::world::*;

/// This macro is used to remove an entry from a Map (usually HashMap or IndexMap)
/// the item that is removed from the map is then decoded from the NBT
/// into the requested type.
/// ```rust,no_run
/// let map: Map;
/// let value: Byte = map_decoder!(map; "some tag" -> Byte);
/// // In case the value might not exist.
/// let option: Option<Byte> = map_decoder!(map; "some tag" -> Option<Byte>);
/// ```
macro_rules! map_decoder {
	($map:expr; $name:literal) => {
		$map.remove($name).ok_or(McError::NbtDecodeError)?
	};
	($map:expr; $name:literal -> Option<$type:ty>) => {
		if let Some(tag) = $map.remove($name) {
			Some(<$type>::decode_nbt(tag)?)
		} else {
			None
		}
	};
	($map:expr; $name:literal -> $type:ty) => {
		<$type>::decode_nbt($map.remove($name).ok_or(McError::NotFoundInCompound($name.to_owned()))?)?
	};
}

pub struct BlockStates {
	palette: Vec<BlockState>,
	data: Vec<u32>,
}

pub struct LightData {
	data: Box<[i8; 2048]>
}

pub struct Biomes {
	palette: Vec<String>
}

pub struct TileTick {
	block_id: String,
	precedence: i32,
	ticks: i32,
	x: i32,
	y: i32,
	z: i32,
}

pub struct Heightmaps {
	motion_blocking: Vec<i64>,
	motion_blocking_no_leaves: Vec<i64>,
	ocean_floor: Vec<i64>,
	ocean_floor_wg: Vec<i64>,
	world_surface: Vec<i64>,
	world_surface_wg: Vec<i64>,
}

impl DecodeNbt for Heightmaps {
	type Error = McError;

	fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
		if let Tag::Compound(mut map) = nbt {
			Ok(Heightmaps {
				motion_blocking: map_decoder!(map; "MOTION_BLOCKING" -> Vec<i64>),
				motion_blocking_no_leaves: map_decoder!(map; "MOTION_BLOCKING_NO_LEAVES" -> Vec<i64>),
				ocean_floor: map_decoder!(map; "OCEAN_FLOOR" -> Vec<i64>),
				ocean_floor_wg: map_decoder!(map; "OCEAN_FLOOR_WG" -> Vec<i64>),
				world_surface: map_decoder!(map; "WORLD_SURFACE" -> Vec<i64>),
				world_surface_wg: map_decoder!(map; "WORLD_SURFACE_WG" -> Vec<i64>),
			})
		} else {
			Err(McError::NbtDecodeError)
		}
	}
}

pub struct Chunk {
	/// DataVersion
	pub data_version: i32,
	/// xPos
	pub x: i32,
	/// yPos
	pub y: i32,
	/// zPos
	pub z: i32,
	/// LastUpdate
	pub last_update: i64,
	/// Status
	pub status: String,
	/// sections
	pub sections: ChunkSections,
	/// block_entities
	pub block_entities: Vec<BlockEntity>,
	/// CarvingMasks
	pub carving_masks: CarvingMasks,
	/// HeightMaps
	pub heightmaps: Heightmaps,
	/// fluid_ticks
	pub fluid_ticks: ListTag,
	/// block_ticks
	pub block_ticks: ListTag,
	/// InhabitedTime
	pub inhabited_time: i64,
	/// PostProcessing
	pub post_processing: ListTag,
	/// structures
	pub structures: Map,
	/// Lights
	pub lights: Option<ListTag>,
	/// Entities
	pub entities: Option<ListTag>,
}

pub struct ChunkSection {
	pub y: i8,
	pub blocks: Option<Box<[u32]>>,
	pub biomes: Option<Map>,
	pub skylight: Option<Vec<i8>>,
	pub blocklight: Option<Vec<i8>>,
}

pub struct ChunkSections {
	sections: Vec<ChunkSection>,
}

pub struct CarvingMasks {
	air: Vec<i8>,
	liquid: Vec<i8>,
}

impl DecodeNbt for CarvingMasks {
	type Error = McError;

	fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
		if let Tag::Compound(mut map) = nbt {
			let air = map_decoder!(map; "AIR" -> Vec<i8>);
			let liquid = map_decoder!(map; "LIQUID" -> Vec<i8>);
			Ok(CarvingMasks {
				air,
				liquid,
			})
		} else {
			Err(McError::NbtDecodeError)
		}
	}
}

pub struct BlockEntity {
	id: String,
	keep_packed: i8,
	x: i32,
	y: i32,
	z: i32,
	data: Map,
}

impl DecodeNbt for Vec<BlockEntity> {
	type Error = McError;
	fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
		if let Tag::List(ListTag::Compound(entities)) = nbt {
			entities.into_iter().map(|mut entity| {
				Ok(BlockEntity {
					id: map_decoder!(entity; "id" -> String),
					keep_packed: map_decoder!(entity; "keepPacked" -> i8),
					x: map_decoder!(entity; "x" -> i32),
					y: map_decoder!(entity; "y" -> i32),
					z: map_decoder!(entity; "z" -> i32),
					data: entity,
				})
			}).collect::<Result<Vec<BlockEntity>, McError>>()
		} else {
			Err(McError::NbtDecodeError)
		}
	}
}

/*
def extract_index(full_index, palette_size, block_states):
	bitsize = max((palette_size - 1).bit_length(), 4)
	#vpl = values per long
	vpl = 64 // bitsize
	mask = 2**bitsize-1
	state_index = full_index // vpl
	value_offset = (full_index % vpl) * bitsize
	slot = int(block_states[state_index])
	return (slot & (mask << value_offset)) >> value_offset

	states_tag = section_tag['BlockStates']
	palette = section_tag['Palette']

	
if palette is not None and states_tag is not None:
	states = list()
	blocks = numpy.ndarray(shape=(4096,),dtype=numpy.object_)
	for v in palette.data:
		name = v.Name.value
		props = {}
		if 'Properties' in v:
			props = { k : val.value for k, val in v.Properties.data.items() }
		states.append(blockregistry.register(name, props))
	
	for i in range(4096):
		ind = extract_index(i, len(palette.data), states_tag.data)
		blocks[i] = states[ind].unique_key
*/

pub fn decode_palette(palette: ListTag) -> Result<Vec<BlockState>, McError> {
	if let ListTag::Compound(mut states) = palette {
		states.into_iter().map(|mut state| {
			let name = map_decoder!(state; "Name" -> String);
			// The "Properties" tag may not exist.
			let properties = if state.contains_key("Properties") {
				let props = map_decoder!(state; "Properties" -> Map);
				BlockProperties::from(props.into_iter().map(|(name, value)| {
					if let Tag::String(value) = value {
						Ok((name, value))
					} else {
						Err(McError::NbtDecodeError)
					}
				}).collect::<Result<Vec<(String, String)>, McError>>()?)
			} else {
				BlockProperties::none()
			};
			Ok(BlockState::new(name, properties))
		}).collect::<Result<Vec<BlockState>, McError>>()
	} else {
		Err(McError::NbtDecodeError)
	}
}

pub fn extract_palette_index(index: usize, palette_size: usize, states: &[i64]) -> usize {
	let bitsize = palette_size.bit_length().max(4);
	let vpl = (64 / bitsize) as u64;
	let mask = 2u64.pow(bitsize) - 1;
	let state_index = index as u64 / vpl;
	let value_offset = ((index as u64).rem_euclid(vpl)) as u32 * bitsize;
	let slot = states[state_index as usize] as u64;
	((slot & (mask << value_offset)) >> value_offset) as usize
}

pub fn decode_section(block_registry: &mut BlockRegistry, mut section: Map) -> Result<ChunkSection, McError> {
	let y = map_decoder!(section; "Y" -> Byte);
	// The following three may or may not exist.
	let biomes = map_decoder!(section; "biomes" -> Option<Map>);
	let blocklight = map_decoder!(section; "BlockLight" -> Option<ByteArray>);
	let skylight = map_decoder!(section; "SkyLight" -> Option<ByteArray>);

	let mut block_states = map_decoder!(section; "block_states" -> Map);

	// Now I need to transform the block_data and palette into registry IDs.
	// The ending block_data should contain 4096 u32 values representing their IDs in
	// the registry. So I need to register each BlockState in the palette with the
	// registry, retrieving the ID. I think the appropriate way to do this would be
	// to do an iterator map to the block_registry IDs.
	let palette = decode_palette(map_decoder!(block_states; "palette" -> ListTag))?;
	// Register blocks.
	let palette = palette.into_iter()
		.map(|state| {
			block_registry.register(&state)
		}).collect::<Vec<u32>>();
	let blocks = if block_states.contains_key("data") {
		// Extract indices from packed values.
		let data = map_decoder!(block_states; "data" -> LongArray);
		let data = (0..4096).into_iter().map(|full_index| {
			let index = extract_palette_index(full_index, palette.len(), data.as_slice());
			palette[index]
		}).collect::<Vec<u32>>();
		Some(data.into_boxed_slice())
	} else {
		None
	};
	Ok(ChunkSection {
		y,
		biomes,
		blocklight,
		skylight,
		blocks,
	})
	
	// todo!()
}

pub fn decode_chunk(block_registry: &mut BlockRegistry, nbt: Tag) -> McResult<Chunk> {
	if let Tag::Compound(mut map) = nbt {
		
		if let Tag::List(ListTag::Compound(mut sections)) = map.remove("sections").ok_or(McError::NotFoundInCompound("sections".to_owned()))? {
			let sections = sections.into_iter()
				.map(|mut section| decode_section(block_registry, section))
				.collect::<Result<Vec<ChunkSection>, McError>>()?;
		} else {
			return Err(McError::NbtDecodeError)
		}
		let sections = if let ListTag::Compound(sections) = map_decoder!(map; "sections" -> ListTag) {
			sections.into_iter()
				.map(|section| decode_section(block_registry, section))
				.collect::<McResult<Vec<ChunkSection>>>()?
		} else {
			return Err(McError::NbtDecodeError);
		};
		let sections = ChunkSections {
			sections,
		};
		Ok(Chunk {
			sections,
			data_version: map_decoder!(map; "DataVersion" -> i32),
			x: map_decoder!(map; "xPos" -> i32),
			y: map_decoder!(map; "yPos" -> i32),
			z: map_decoder!(map; "zPos" -> i32),
			last_update: map_decoder!(map; "LastUpdate" -> i64),
			block_entities: map_decoder!(map; "block_entities" -> Vec<BlockEntity>),
			carving_masks: map_decoder!(map; "CarvingMasks" -> CarvingMasks),
			heightmaps: map_decoder!(map; "Heightmaps" -> Heightmaps),
			fluid_ticks: map_decoder!(map; "fluid_ticks" -> ListTag),
			block_ticks: map_decoder!(map; "block_ticks" -> ListTag),
			post_processing: map_decoder!(map; "PostProcessing" -> ListTag),
			structures: map_decoder!(map; "structures" -> Map),
			inhabited_time: map_decoder!(map; "InhabitedTime" -> i64),
			status: map_decoder!(map; "Status" -> String),
			lights: map_decoder!(map; "Lights" -> Option<ListTag>),
			entities: map_decoder!(map; "Entities" -> Option<ListTag>),
		})
	} else {
		Err(McError::NbtDecodeError)
	}
}

/*
TODO: 	Make it so that chunks can be loaded directly from memory.
		This would involve more complicated programming, but it would
		give faster load times. I also need to make it so that there
		is a World block registry to register blocks to.
*/