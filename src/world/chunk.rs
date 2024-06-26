// #![allow(unused)]
use std::collections::HashMap;
// use std::default;
use std::ops::Not;

use super::block::HeightmapFlag;
use super::blockstate::*;

use crate::McError;
use crate::McResult;
use crate::math::bit::BitLength;
use crate::nbt::*;
// use crate::nbt::io::*;
use crate::nbt::tag::*;
use crate::nbt::tagtype::*;
use super::blockregistry::BlockRegistry;
// use super::world::*;

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
        $map.remove($name).ok_or(McError::NotFoundInCompound($name.to_owned()))?
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

macro_rules! map_encoder {
    ($map:expr; $name:literal = $value:expr) => {
        ($map).insert($name.to_owned(), $value.encode_nbt());
    };
    ($map:expr; $($name:literal = $value:expr;)+) => {
        $(
            map_encoder!($map; $name = $value);
        )+
    };
}

// pub struct BlockStates {
// 	palette: Vec<BlockState>,
// 	data: Vec<u32>,
// }

// pub struct LightData {
// 	data: Box<[i8; 2048]>
// }

// pub struct Biomes {
// 	palette: Vec<String>
// }

// pub struct TileTick {
// 	block_id: String,
// 	precedence: i32,
// 	ticks: i32,
// 	x: i32,
// 	y: i32,
// 	z: i32,
// }
/*
Chunks that are written to a region file are formatted a certain way.
They are isolated from the world that they come from, and as such they
contain data from that world that may not necessarily be contained in a chunk.
That is to say, it's likely that some data in a region-chunk can be transformed
into a higher level structure within the world structure. The heighmaps are a
good example. It may perhaps be more optimal in-engine for the world to
consider the Chunk struct to be mere data and not suitable for representation
in engine, so the engine may choose a different representation.
*/
#[derive(Clone)]
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
    /// CarvingMasks
    pub carving_masks: Option<CarvingMasks>,
    /// Lights
    pub lights: Option<ListTag>,
    /// Entities
    pub entities: Option<ListTag>,
    /// All other unknown tags.
    pub other: Map,
}

impl Chunk {

    #[inline(always)]
    fn section_index_and_local_coord(&self, coord: (i64, i64, i64)) -> (usize, (i64, i64, i64)) {
        let lowy = self.sections.sections[0].y;
        let section_index = chunk_section_index(coord.1, lowy as i64);
        let local = chunk_local_coord(coord);
        (section_index, local)
    }

    pub fn blocklight(&self, coord: (i64, i64, i64)) -> u8 {
        let (section_index, (x, y, z)) = self.section_index_and_local_coord(coord);
        self.sections.sections[section_index].blocklight(x, y, z)
    }

    pub fn skylight(&self, coord: (i64, i64, i64)) -> u8 {
        let (section_index, (x, y, z)) = self.section_index_and_local_coord(coord);
        self.sections.sections[section_index].skylight(x, y, z)
    }

    pub fn set_blocklight(&mut self, coord: (i64, i64, i64), level: u8) -> u8 {
        let (section_index, (x, y, z)) = self.section_index_and_local_coord(coord);
        self.sections.sections[section_index].set_blocklight(x, y, z, level)
    }

    pub fn set_skylight(&mut self, coord: (i64, i64, i64), level: u8) -> u8 {
        let (section_index, (x, y, z)) = self.section_index_and_local_coord(coord);
        self.sections.sections[section_index].set_skylight(x, y, z, level)
    }

    pub fn get_id(&self, coord: (i64, i64, i64)) -> Option<u32> {
        let (section_index, (x, y, z)) = self.section_index_and_local_coord(coord);
        self.sections.sections[section_index].get_id(x, y, z)
    }

    pub fn set_id(&mut self, coord: (i64, i64, i64), id: u32) -> Option<u32> {
        let (section_index, (x, y, z)) = self.section_index_and_local_coord(coord);
        self.sections.sections[section_index].set_id(x, y, z, id)
    }

    pub fn to_nbt(&self, block_registry: &BlockRegistry) -> Tag {
        Tag::Compound(encode_chunk(block_registry, self))
    }

    #[allow(unused)]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        // I'm not entirely sure how I am supposed to structure the
        // fields of the chunk for an empty chunk. I guess I should
        // do some more work researching the structure of chunks.
        // Also, for simplicity, this library will target some
        // specific version of minecraft. Let's say, 1.19.
        // I don't think it would be advisable to attempt to target
        // a wide range of data versions.
        todo!()
    }

    pub fn get_heightmap(&self, heightmap: HeightmapFlag, x: i64, z: i64) -> i64 {
        match heightmap {
            HeightmapFlag::MotionBlocking => self.heightmaps.motion_blocking.get((x, z)),
            HeightmapFlag::MotionBlockingNoLeaves => self.heightmaps.motion_blocking_no_leaves.get((x, z)),
            HeightmapFlag::OceanFloor => self.heightmaps.ocean_floor.get((x, z)),
            HeightmapFlag::WorldSurface => self.heightmaps.world_surface.get((x, z)),
        }
    }

    pub fn set_heightmap(&mut self, heightmap: HeightmapFlag, x: i64, z: i64, height: u16) {
        match heightmap {
            HeightmapFlag::MotionBlocking => self.heightmaps.motion_blocking.set((x, z), height),
            HeightmapFlag::MotionBlockingNoLeaves => self.heightmaps.motion_blocking_no_leaves.set((x, z), height),
            HeightmapFlag::OceanFloor => self.heightmaps.ocean_floor.set((x, z), height),
            HeightmapFlag::WorldSurface => self.heightmaps.world_surface.set((x, z), height),
        }
    }
}

impl EncodeNbt for Vec<BlockEntity> {
    fn encode_nbt(self) -> Tag {
        let entities = self.into_iter().map(|entity| {
            let mut map = Map::new();
            map_encoder!(map;
                "id" = entity.id;
                "keepPacked" = entity.keep_packed;
                "x" = entity.x;
                "y" = entity.y;
                "z" = entity.z;
            );
            map.extend(entity.data);
            map
        }).collect::<Vec<Map>>();
        Tag::List(ListTag::Compound(entities))
    }
}

impl DecodeNbt for Vec<BlockEntity> {
    fn decode_nbt(nbt: Tag) -> McResult<Self> {
        let Tag::List(list) = nbt else {
            return Err(McError::NbtDecodeError);
        };
        let ListTag::Compound(entities) = list else {
            return Ok(Vec::new());
        };
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
    }
}

#[derive(Clone)]
pub struct Lighting {
    levels: Vec<u8>,
}

impl From<Vec<u8>> for Lighting {
    fn from(value: Vec<u8>) -> Self {
        Self {
            levels: value
        }
    }
}

impl From<Vec<i8>> for Lighting {
    fn from(value: Vec<i8>) -> Self {
        Self {
            levels: value.into_iter().map(|v| v as u8).collect()
        }
    }
}

impl Into<Vec<i8>> for Lighting {
    fn into(self) -> Vec<i8> {
        self.levels.into_iter().map(|v| v as i8).collect()
    }
}

impl Into<Vec<u8>> for Lighting {
    fn into(self) -> Vec<u8> {
        self.levels
    }
}

impl Lighting {
    pub fn get(&self, x: i64, y: i64, z: i64) -> u8 {
        let index = chunk_yzx_index(x, y, z);
        let half_index = index.div_euclid(2);
        let bit_offset = index.rem_euclid(2) * 4;
        let level = self.levels[half_index];
        (level & (0xf << bit_offset)) >> bit_offset
    }

    pub fn set(&mut self, x: i64, y: i64, z: i64, level: u8) -> u8 {
        if level > 15 {
            panic!("level must be less than 16.")
        }
        let index = chunk_yzx_index(x, y, z);
        let half_index = index.div_euclid(2);
        let bit_offset = index.rem_euclid(2) * 4;
        let mask = 0xf << bit_offset;
        let inv_mask = mask.not();
        let old_level = self.levels[half_index];
        let new_level = (old_level & inv_mask) | (level << bit_offset);
        self.levels[half_index] = new_level;
        (old_level & mask) >> bit_offset
    }
}

impl DecodeNbt for Lighting {
    fn decode_nbt(nbt: Tag) -> McResult<Self> {
        if let Tag::ByteArray(light_data) = nbt {
            Ok(Lighting::from(light_data))
        } else {
            Err(McError::NbtDecodeError)
        }
    }
}

impl EncodeNbt for Lighting {
    fn encode_nbt(self) -> Tag {
        Tag::ByteArray(self.into())
    }
}

#[derive(Clone)]
pub struct ChunkSection {
    pub y: i8,
    pub blocks: Option<Box<[u32]>>,
    pub biomes: Option<Map>,
    pub skylight: Option<Lighting>,
    pub blocklight: Option<Lighting>,
}

impl ChunkSection {

    pub fn skylight(&self, x: i64, y: i64, z: i64) -> u8 {
        if let Some(light) = &self.skylight {
            light.get(x, y, z)
        } else {
            0
        }
    }

    pub fn set_skylight(&mut self, x: i64, y: i64, z: i64, level: u8) -> u8 {
        if let Some(light) = &mut self.skylight {
            light.set(x, y, z, level)
        } else {
            0
        }
    }

    pub fn blocklight(&self, x: i64, y: i64, z: i64) -> u8 {
        if let Some(light) = &self.blocklight {
            light.get(x, y, z)
        } else {
            0
        }
    }

    pub fn set_blocklight(&mut self, x: i64, y: i64, z: i64, level: u8) -> u8 {
        if let Some(light) = &mut self.blocklight {
            light.set(x, y, z, level)
        } else {
            0
        }
    }

    pub fn get_id(&self, local_x: i64, local_y: i64, local_z: i64) -> Option<u32> {
        if let Some(blocks) = &self.blocks {
            let index = chunk_yzx_index(local_x, local_y, local_z);
            Some(blocks[index])
        } else {
            None
        }
    }

    pub fn set_id(&mut self, local_x: i64, local_y: i64, local_z: i64, id: u32) -> Option<u32> {
        if self.blocks.is_none() && id != 0 {
            self.blocks = Some(Box::new([0u32; 4096]));
        }
        let Some(blocks) = &mut self.blocks else {
            return None;
        };
        let index = chunk_yzx_index(local_x, local_y, local_z);
        let result = blocks[index];
        blocks[index] = id;
        Some(result)
    }
}

#[derive(Clone)]
pub struct ChunkSections {
    pub sections: Vec<ChunkSection>,
}

#[derive(Clone)]
pub struct BlockEntity {
    pub id: String,
    pub keep_packed: i8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub data: Map,
}

#[derive(Clone)]
pub struct Heightmap {
    pub map: Vec<i64>
}

impl Heightmap {
    pub fn get(&self, coord: (i64, i64)) -> i64 {
        let index = (coord.1 * 16 + coord.0) as usize;
        let sub_index = index / 7;
        let mask_offset = (index % 7) * 9;
        let mask = 511 << mask_offset;
        (self.map[sub_index] & mask) >> mask_offset
    }

    /// Height must be less than 512. Will panic otherwise.
    pub fn set(&mut self, coord: (i64, i64), height: u16) {
        if height > 511 {
            panic!("Height should not be greater than 511.");
        }
        let index = (coord.1 * 16 + coord.0) as usize;
        let sub_index = index / 7;
        let mask_offset = (index % 7) * 9;
        let mask = 511i64 << mask_offset;
        let value = (height & 511) as i64;
        let invert_mask = mask.not();
        self.map[sub_index] = (self.map[sub_index] & invert_mask) | (value << mask_offset)
    }
}

impl DecodeNbt for Heightmap {
    fn decode_nbt(nbt: Tag) -> McResult<Self> {
        if let Tag::LongArray(data) = nbt {
            Ok(Heightmap::from(data))
        } else {
            Err(McError::NbtDecodeError)
        }
    }
}

impl EncodeNbt for Heightmap {
    fn encode_nbt(self) -> Tag {
        Tag::LongArray(self.map)
    }
}

impl From<Vec<i64>> for Heightmap {
    fn from(value: Vec<i64>) -> Self {
        Self { map: value }
    }
}

impl Into<Vec<i64>> for Heightmap {
    fn into(self) -> Vec<i64> {
        self.map
    }
}

#[derive(Clone)]
pub struct Heightmaps {
    pub motion_blocking: Heightmap,
    pub motion_blocking_no_leaves: Heightmap,
    pub ocean_floor: Heightmap,
    pub ocean_floor_wg: Option<Heightmap>,
    pub world_surface: Heightmap,
    pub world_surface_wg: Option<Heightmap>,
}

impl EncodeNbt for Heightmaps {
    fn encode_nbt(self) -> Tag {
        let mut map = Map::new();
        map_encoder!(map;
            "MOTION_BLOCKING" = self.motion_blocking;
            "MOTION_BLOCKING_NO_LEAVES" = self.motion_blocking_no_leaves;
            "OCEAN_FLOOR" = self.ocean_floor;
            // "OCEAN_FLOOR_WG" = self.ocean_floor_wg;
            "WORLD_SURFACE" = self.world_surface;
            // "WORLD_SURFACE_WG" = self.world_surface_wg;
        );
        if let Some(ofwg) = self.ocean_floor_wg {
            map_encoder!(map; "OCEAN_FLOOR_WG" = ofwg);
        }
        if let Some(wswg) = self.world_surface_wg {
            map_encoder!(map; "WORLD_SURFACE_WG" = wswg);
        }
        Tag::Compound(map)
    }
}

impl DecodeNbt for Heightmaps {
    fn decode_nbt(nbt: Tag) -> McResult<Self> {
        let Tag::Compound(mut map) = nbt else {
            return Err(McError::NbtDecodeError);
        };
        Ok(Heightmaps {
            motion_blocking: map_decoder!(map; "MOTION_BLOCKING" -> Heightmap),
            motion_blocking_no_leaves: map_decoder!(map; "MOTION_BLOCKING_NO_LEAVES" -> Heightmap),
            ocean_floor: map_decoder!(map; "OCEAN_FLOOR" -> Heightmap),
            ocean_floor_wg: map_decoder!(map; "OCEAN_FLOOR_WG" -> Option<Heightmap>),
            world_surface: map_decoder!(map; "WORLD_SURFACE" -> Heightmap),
            world_surface_wg: map_decoder!(map; "WORLD_SURFACE_WG" -> Option<Heightmap>),
        })
    }
}

#[derive(Clone)]
pub struct CarvingMasks {
    pub air: Vec<i8>,
    pub liquid: Vec<i8>,
}

impl EncodeNbt for CarvingMasks {
    fn encode_nbt(self) -> Tag {
        let mut map = Map::new();
        map_encoder!(map;
            "AIR" = self.air;
            "LIQUID" = self.liquid;
        );
        Tag::Compound(map)
    }
}

impl DecodeNbt for CarvingMasks {
    fn decode_nbt(nbt: Tag) -> McResult<Self> {
        let Tag::Compound(mut map) = nbt else {
            return Err(McError::NbtDecodeError);
        };
        let air = map_decoder!(map; "AIR" -> Vec<i8>);
        let liquid = map_decoder!(map; "LIQUID" -> Vec<i8>);
        Ok(CarvingMasks {
            air,
            liquid,
        })
    }
}

#[inline(always)]
fn chunk_local_coord(coord: (i64, i64, i64)) -> (i64, i64, i64) {
    (
        coord.0 & 0xf,
        coord.1 & 0xf,
        coord.2 & 0xf,
    )
}

#[inline(always)]
const fn chunk_section_index(coord_y: i64, chunk_y: i64) -> usize {
    let section_index = coord_y.div_euclid(16);
    let adj_index = section_index - chunk_y;
    adj_index as usize
}

#[inline(always)]
fn chunk_yzx_index(x: i64, y: i64, z: i64) -> usize {
    let local_x = x & 0xf;
    let local_y = y & 0xf;
    let local_z = z & 0xf;
    ((local_y<<8) | (local_z<<4) | local_x) as usize
}

pub fn extract_palette_index(index: usize, palette_size: usize, states: &[i64]) -> usize {
    // Subtract 1 because it's the bit length of the largest possible index
    // If the palette size is 16, the bit length to represent
    // 16 is 5, but the bit length to represent the largest index (15)
    // is only 4.
    let bitsize = (palette_size - 1).bit_length().max(4);
    // vpl: values-per-long
    let vpl = (64 / bitsize) as u64;
    let mask = 2u64.pow(bitsize) - 1;
    let state_index = index as u64 / vpl;
    let value_offset = ((index as u64).rem_euclid(vpl)) as u32 * bitsize;
    let slot = states[state_index as usize] as u64;
    ((slot & (mask << value_offset)) >> value_offset) as usize
}

fn inject_palette_index(full_index: usize, palette_size: usize, states: &mut [i64], value: u32) {
    let bitsize = (palette_size - 1).bit_length().max(4);
    // vpl: values-per-long
    let vpl = (64 / bitsize) as u64;
    let mask = 2u64.pow(bitsize) - 1;
    let state_index = full_index as u64 / vpl;
    let value_offset = ((full_index as u64).rem_euclid(vpl) as u32) * bitsize;
    let state = states[state_index as usize] as u64;
    let new_value = (state & (mask << value_offset).not()) | ((value as u64) << value_offset);
    states[state_index as usize] = new_value as i64;
}

pub fn decode_palette(palette: ListTag) -> Result<Vec<BlockState>, McError> {
    let ListTag::Compound(states) = palette else {
        return Err(McError::NbtDecodeError);
    };
    states.into_iter().map(|state| {
        BlockState::try_from_map(&state)
    }).collect::<Result<Vec<BlockState>, McError>>()
}

pub fn decode_section(block_registry: &mut BlockRegistry, mut section: Map) -> Result<ChunkSection, McError> {
    let y = map_decoder!(section; "Y" -> Byte);
    // The following three may or may not exist.
    let biomes = map_decoder!(section; "biomes" -> Option<Map>);
    let blocklight = map_decoder!(section; "BlockLight" -> Option<Lighting>);
    let skylight = map_decoder!(section; "SkyLight" -> Option<Lighting>);

    let block_states = map_decoder!(section; "block_states" -> Option<Map>);

    let blocks = if let Some(mut block_states) = block_states {
        // Now I need to transform the block_data and palette into registry IDs.
        // The ending block_data should contain 4096 u32 values representing their IDs in
        // the registry. So I need to register each BlockState in the palette with the
        // registry, retrieving the ID. I think the appropriate way to do this would be
        // to do an iterator map to the block_registry IDs.
        let palette = decode_palette(map_decoder!(block_states; "palette" -> ListTag))?;
        // Register blocks.
        let palette = palette.iter().map(|state| {
            block_registry.register(state)
        }).collect::<Vec<u32>>();
        map_decoder!(block_states; "data" -> Option<LongArray>).map(|blocks| {
            (0..4096).into_iter().map(|full_index| {
                let index = extract_palette_index(full_index, palette.len(), &blocks);
                palette[index]
            }).collect::<Box<[u32]>>()
        })
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
}

pub fn decode_chunk(block_registry: &mut BlockRegistry, nbt: Tag) -> McResult<Chunk> {
    let Tag::Compound(mut map) = nbt else {
        return Err(McError::NbtDecodeError);
    };
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
        heightmaps: map_decoder!(map; "Heightmaps" -> Heightmaps),
        fluid_ticks: map_decoder!(map; "fluid_ticks" -> ListTag),
        block_ticks: map_decoder!(map; "block_ticks" -> ListTag),
        post_processing: map_decoder!(map; "PostProcessing" -> ListTag),
        structures: map_decoder!(map; "structures" -> Map),
        inhabited_time: map_decoder!(map; "InhabitedTime" -> i64),
        status: map_decoder!(map; "Status" -> String),
        carving_masks: map_decoder!(map; "CarvingMasks" -> Option<CarvingMasks>),
        lights: map_decoder!(map; "Lights" -> Option<ListTag>),
        entities: map_decoder!(map; "Entities" -> Option<ListTag>),
        other: map,
    })
}

fn encode_block_states(block_registry: &BlockRegistry, blocks: &Option<Box<[u32]>>) -> Map {
    if let Some(blocks) = blocks {
        // Collect unique block-ids
        // local_registry holds the mapping from old ids to new ids.
        // This procedure maps out the block-states used into a palette and remaps
        // the block ids to the new palette.
        let mut local_registry = HashMap::<u32, u32>::new();
        let mut palette = Vec::<BlockState>::new();
        let local_ids = blocks.iter().map(|block_id| {
            if let Some(local_id) = local_registry.get(block_id) {
                *local_id
            } else {
                if let Some(state) = block_registry.get(*block_id) {
                    // The id is the index of the item, so to get the proper id
                    // we get the length of the palette prior to adding the new block state.
                    let id = palette.len() as u32;
                    local_registry.insert(*block_id, id);
                    palette.push(state.clone());
                    id
                } else {
                    0
                }
            }
        }).collect::<Vec<u32>>();
        // Pack 4096 block ids into array of i64.
        // The buffer size for the long_array is calculated based on
        // palette size.
        // `palette.len() - 1`: The `- 1` is because The bitsize is the bit_length of
        //	the maximum index, which is the same as the length of the palette minus 1.
        let bitsize = (palette.len() - 1).bit_length().max(4);
        // vpl: values-per-long
        let vpl = (64 / bitsize) as u64;
        // (4096u64.rem_euclid(vpl) != 0 as u64)
        // The buffer needs to be able to hold 4096 (16*16*16) elements.
        // To find the packed buffer size, you simply divide 4096 by vpl, and if
        // there is a remainder, add one.
        let buffer_size = 4096/vpl + ((4096u64.rem_euclid(vpl) != 0) as u64);
        let mut packed = vec![0i64; buffer_size as usize];
        local_ids.into_iter().enumerate().for_each(|(i, id)| {
            inject_palette_index(i, palette.len(), &mut packed, id);
        });
        // Build palette
        let palette = palette.into_iter().map(|state| {
            state.to_nbt()
        }).collect::<Vec<Map>>();
        let palette = Tag::List(ListTag::Compound(palette));
        let data = Tag::LongArray(packed);
        Map::from([
            ("palette".to_owned(), palette),
            ("data".to_owned(), data),
        ])
    } else {
        let palette = Map::from([
            ("Name".to_owned(), Tag::string("minecraft:air"))
        ]);
        let palette = ListTag::Compound(vec![palette]);
        Map::from([
            ("palette".to_owned(), Tag::List(palette)),
        ])
    }
}

fn encode_section(block_registry: &BlockRegistry, section: &ChunkSection) -> Map {
    let mut map = Map::new();
    map_encoder!(map; "Y" = section.y);
    if let Some(biomes) = &section.biomes {
        let biomes = biomes.clone();
        map_encoder!(map; "biomes" = biomes);
    }
    if let Some(blocklight) = &section.blocklight {
        let blocklight = blocklight.clone();
        map_encoder!(map; "BlockLight" = blocklight);
    }
    if let Some(skylight) = &section.skylight {
        let skylight = skylight.clone();
        map_encoder!(map; "SkyLight" = skylight);
    }
    let block_states = encode_block_states(block_registry, &section.blocks);
    map_encoder!(map; "block_states" = block_states);
    map
}

pub fn encode_chunk(block_registry: &BlockRegistry, chunk: &Chunk) -> Map {
    let mut map = Map::new();
    let data_version = chunk.data_version;
    let x = chunk.x;
    let y = chunk.y;
    let z = chunk.z;
    let last_update = chunk.last_update;
    let inhabited_time = chunk.inhabited_time;
    let status = chunk.status.clone(); 
    let block_entities = chunk.block_entities.clone();
    let heightmaps = chunk.heightmaps.clone();
    let fluid_ticks = chunk.fluid_ticks.clone();
    let block_ticks = chunk.block_ticks.clone();
    let post_processing = chunk.post_processing.clone();
    let structures = chunk.structures.clone();
    map_encoder!(map;
        "DataVersion" = data_version;
        "xPos" = x;
        "yPos" = y;
        "zPos" = z;
        "LastUpdate" = last_update;
        "InhabitedTime" = inhabited_time;
        "Status" = status;
        "block_entities" = block_entities;
        "Heightmaps" = heightmaps;
        "fluid_ticks" = fluid_ticks;
        "block_ticks" = block_ticks;
        "PostProcessing" = post_processing;
        "structures" = structures;
    );
    if let Some(carvingmasks) = &chunk.carving_masks {
        let carvingmasks = carvingmasks.clone();
        map_encoder!(map; "CarvingMasks" = carvingmasks);
    }
    if let Some(lights) = &chunk.lights {
        let lights = lights.clone();
        map_encoder!(map; "Lights" = lights);
    }
    if let Some(entities) = &chunk.entities {
        let entities = entities.clone();
        map_encoder!(map; "Entities" = entities);
    }
    let sections = ListTag::Compound(chunk.sections.sections.iter().map(|section| {
        encode_section(block_registry, section)
    }).collect::<Vec<Map>>());
    map_encoder!(map; "sections" = sections);
    if !chunk.other.is_empty() {
        map.extend(chunk.other.clone());
    }
    map
}

/*
TODO: 	Make it so that chunks can be loaded directly from memory.
        This would involve more complicated programming, but it would
        give faster load times. I also need to make it so that there
        is a World block registry to register blocks to.
*/