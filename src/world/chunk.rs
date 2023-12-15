#![allow(unused)]

use std::collections::HashMap;
use std::default;

use super::blockstate::*;

use crate::McError;
use crate::nbt::*;
use crate::nbt::io::*;
use crate::nbt::tag::*;
use crate::nbt::tagtype::*;

macro_rules! map_decoder {
	($map:expr; $name:literal -> $type:ty) => {
		<$type>::decode_nbt($map.remove($name).ok_or(McError::NotFoundInCompound($name.to_owned()))?)?
	};
}

// pub struct BlockStateProperty {
// 	name: String,
// 	value: String,
// }

// pub enum BlockStateProperties {
// 	None,
// 	Single {
// 		name: String,
// 		value: String,
// 	},
// 	Many(HashMap<String, String>),
// }

// pub struct BlockState {
// 	name: String,
// 	properties: BlockStateProperties,
// }

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

pub struct HeightMaps {
	motion_blocking: Vec<i64>,
	motion_blocking_no_leaves: Vec<i64>,
	ocean_floor: Vec<i64>,
	ocean_floor_wg: Vec<i64>,
	world_surface: Vec<i64>,
	world_surface_wg: Vec<i64>,
}

pub struct Chunk {
	/// DataVersion
	data_version: i32,
	/// xPos
	x: i32,
	/// yPos
	y: i32,
	/// zPos
	z: i32,
	/// LastUpdate
	last_update: i64,
	/// Status
	status: String,
	/// sections
	sections: ChunkSections,
	/// block_entities
	block_entities: Vec<BlockEntity>,
	/// CarvingMasks
	carving_masks: CarvingMasks,
	/// HeightMaps
	heightmaps: HeightMaps,
	/// Lights
	lights: ListTag,
	/// Entities
	entities: ListTag,
	/// fluid_ticks
	fluid_ticks: Vec<Map>,
	/// block_ticks
	block_ticks: Vec<Map>,
	/// InhabitedTime
	inhabited_time: i64,
	/// PostProcessing
	post_processing: Vec<ListTag>,
	/// structures
	structures: Map,
}

pub struct ChunkSection {
	y: i8,
	block_states: BlockStates,
	biomes: Option<Biomes>,
	skylight: Option<LightData>,
	blocklight: Option<LightData>,
}

pub struct ChunkSections {
	sections: Vec<ChunkSection>,
}

pub struct CarvingMasks {
	air: Vec<i8>,
	liquid: Vec<i8>,
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
			Ok(entities.into_iter().map(|mut entity| {
				Ok(BlockEntity {
					id: map_decoder!(entity; "id" -> String),
					keep_packed: map_decoder!(entity; "keepPacked" -> i8),
					x: map_decoder!(entity; "x" -> i32),
					y: map_decoder!(entity; "y" -> i32),
					z: map_decoder!(entity; "z" -> i32),
					data: entity,
				})
			}).collect::<Result<Vec<BlockEntity>, McError>>()?)
		} else {
			Err(McError::NbtDecodeError)
		}
	}
}

pub fn get_coord_from_chunk_nbt(tag: &Tag) -> Option<(i32, i32)> {
	if let Tag::Compound(compound) = tag {
		if let Some(Tag::Int(x_pos)) = compound.get("xPos") {
			if let Some(Tag::Int(z_pos)) = compound.get("zPos") {
				return Some((*x_pos, *z_pos));
			}
		}
	}
	None
}

// impl DecodeNbt for Vec<ChunkSection> {
// 	type Error = McError;

// 	fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
// 		if let Tag::List(ListTag::Compound(compounds)) = nbt {
// 			let sections = compounds.into_iter().map(|section| {
// 				ChunkSection {
// 					y: todo!(),
// 					block_states: todo!(),
// 					biomes: todo!(),
// 					skylight: todo!(),
// 					blocklight: todo!(),
// 				}
// 			}).collect();
// 			Ok(sections)
// 		} else {
// 			Err(McError::NbtDecodeError)
// 		}
// 	}
// }

impl DecodeNbt for ChunkSection {
	type Error = McError;

	fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
		if let Tag::Compound(map) = nbt {
			todo!()
		} else {
			Err(McError::NbtDecodeError)
		}
	}
}

impl DecodeNbt for ChunkSections {
	type Error = McError;

	fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
		if let Tag::List(ListTag::Compound(sections)) = nbt {
			Ok(Self {
				sections: sections.into_iter().map(|section| {
					Ok(ChunkSection {
						y: todo!(),
						block_states: todo!(),
						biomes: todo!(),
						skylight: todo!(),
						blocklight: todo!(),
					})
				}).collect::<Result<Vec<ChunkSection>, McError>>()?,
			})
		} else {
			Err(McError::NbtDecodeError)
		}
	}
}

pub fn load_chunk() -> Chunk {
	todo!()
}

/*
TODO: 	Make it so that chunks can be loaded directly from memory.
		This would involve more complicated programming, but it would
		give faster load times. I also need to make it so that there
		is a World block registry to register blocks to.
*/
impl DecodeNbt for Chunk {
    type Error = McError;

    fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
		
        if let Tag::Compound(mut map) = nbt {
			Ok(Self {
				data_version: map_decoder!(map; "DataVersion" -> i32),
				x: map_decoder!(map; "xPos" -> i32),
				y: map_decoder!(map; "yPos" -> i32),
				z: map_decoder!(map; "zPos" -> i32),
				last_update: map_decoder!(map; "LastUpdate" -> i64),
				sections: map_decoder!(map; "sections" -> ChunkSections),
				block_entities: map_decoder!(map; "block_entities" -> Vec<BlockEntity>),
				carving_masks: todo!(),
				heightmaps: todo!(),
				lights: todo!(),
				entities: todo!(),
				fluid_ticks: todo!(),
				block_ticks: todo!(),
				post_processing: todo!(),
				structures: todo!(),
				inhabited_time: todo!(),
				status: todo!(),
			})
		} else {
			Err(McError::NbtDecodeError)
		}
    }
}