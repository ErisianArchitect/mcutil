// D	BorderCenterX
// D	BorderCenterZ
// D	BorderDamagePerBlock
// D	BorderSafeZone
// D	BorderSize
// D	BorderSizeLerpTarget
// L	BorderSizeLerpTime
// D	BorderWarningBlocks
// D	BorderWarningTime
// C	CustomBossEvents
// C	DataPacks
// I	DavaVersion
// L	DayTime
// B	Difficulty
// B	DifficultyLocked
// C	DragonFight
// C	GameRules
// I	GameType
// L	LastPlayed
// S	LevelName
// C	Player
//

use std::{fs::File, io::{BufReader, BufWriter, Read, Seek, SeekFrom}, path::Path};

use crate::{
	ioext::ReadExt, nbt::{io::write_named_tag, tag::*, Map}, McError, McResult
};
use flate2::{read::GzDecoder, Compression};
use flate2::write::GzEncoder;

pub fn read_level_from_file<P: AsRef<Path>>(path: P) -> McResult<Level> {
	let mut file = File::open(path)?;
	let mut buffer: [u8; 1] = [0];
	file.read_exact(&mut buffer)?;
	if buffer[0] == 31 {
		file.seek(SeekFrom::Start(0))?;
		let reader = BufReader::new(file);
		let mut decoder = GzDecoder::new(reader);
		let root: NamedTag = decoder.read_value()?;
		Level::decode_nbt(root.take_tag())
	} else {
		todo!()
	}
}

pub fn write_level_to_file<P: AsRef<Path>>(path: P, level: &Level) -> McResult<usize> {
	let file = File::create(path)?;
	let writer = BufWriter::new(file);
	let mut encoder = GzEncoder::new(writer, Compression::best());
	let level_tag = level.encode_nbt();
	// let root = NamedTag::new(level_tag);
	// encoder.write_value(&root)
	write_named_tag(&mut encoder, &level_tag, "")
}

/*
Double     BorderCenterX
Double     BorderCenterZ       
Double     BorderDamagePerBlock
Double     BorderSafeZone      
Double     BorderSize
Double     BorderSizeLerpTarget
Long       BorderSizeLerpTime  
Double     BorderWarningBlocks 
Double     BorderWarningTime   
Compound   CustomBossEvents    
Compound   DataPacks
Int        DataVersion
Long       DayTime
Byte       Difficulty
Byte       DifficultyLocked    
Compound   DragonFight
Compound   GameRules
Int        GameType
Long       LastPlayed
String     LevelName
Compound   Player
List       ScheduledEvents
List       ServerBrands
Float      SpawnAngle
Int        SpawnX
Int        SpawnY
Int        SpawnZ
Long       Time
Compound   Version
Int        WanderingTraderSpawnChance
Int        WanderingTraderSpawnDelay
Byte       WasModded
Compound   WorldGenSettings
Byte       allowCommands
Int        clearWeatherTime
Byte       hardcore
Byte       initialized
Int        rainTime
Byte       raining
Int        thunderTime
Byte       thundering
Int        version
*/

pub struct Level {
	/// BorderCenterX
	border_center_x: f64,
	/// BorderCenterZ
	border_center_z: f64,
	/// BorderDamagePerBlock
	border_damage_per_block: f64,
	/// BorderSize
	border_size: f64,
	/// BorderSizeLerpTarget
	border_size_lerp_target: f64,
	/// BorderSizeLerpTime
	border_size_lerp_time: i64,
	/// BorderWarningBlocks
	border_warning_blocks: f64,
	/// BorderWarningTime
	border_warning_time: f64,
	/// CustomBossEvents
	custom_boss_events: Map,
	/// DataPacks
	data_packs: Map,
	/// DataVersion
	data_version: i32,
	/// DayTime
	day_time: i64,
	/// Difficulty
	difficulty: i8,
	///	DifficultyLocked
	difficulty_locked: i8,
	/// DragonFight
	dragon_fight: Map,
	/// GameRules
	game_rules: Map,
	/// GameType
	game_type: i32,
	/// LastPlayed
	last_played: i64,
	/// LevelName
	level_name: String,
	/// Player
	player: Map,
	/// ScheduledEvents
	scheduled_events: ListTag,
	/// ServerBrands
	server_brands: ListTag,
	/// SpawnAngle
	spawn_angle: f32,
	/// SpawnX
	spawn_x: i32,
	/// SpawnY
	spawn_y: i32,
	/// SpawnZ
	spawn_z: i32,
	/// Time
	time: i64,
	/// Version
	version: Map,
	/// WanderingTraderSpawnChance
	wandering_trader_spawn_chance: i32,
	/// WanderingTraderSpawnDelay
	wandering_trader_spawn_delay: i32,
	/// WasModded
	was_modded: i8,
	/// WorldGenSettings
	world_gen_settings: Map,
	/// allowCommands
	allow_commands: i8,
	/// clearWeatherTime
	clear_weather_time: i32,
	/// hardcore
	hardcore: i8,
	/// initialized
	initialized: i8,
	/// rainTime
	rain_time: i32,
	/// raining
	raining: i8,
	/// thunderTime
	thunder_time: i32,
	/// thundering
	thundering: i8,
	/// version
	version2: i32, // What absolute moron decided to have two variables named "version"?
}

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

impl Level {
	pub fn encode_nbt(&self) -> Tag {
		let mut data = Map::new();
		map_encoder!(data;
			"BorderCenterX" = self.border_center_x;
			"BorderCenterZ" = self.border_center_z;
			"BorderDamagePerBlock" = self.border_damage_per_block;
			"BorderSize" = self.border_size;
			"BorderSizeLerpTarget" = self.border_size_lerp_target;
			"BorderSizeLerpTime" = self.border_size_lerp_time;
			"BorderWarningBlocks" = self.border_warning_blocks;
			"BorderWarningTime" = self.border_warning_time;
			"CustomBossEvents" = self.custom_boss_events.clone();
			"DataPacks" = self.data_packs.clone();
			"DataVersion" = self.data_version;
			"DayTime" = self.day_time;
			"Difficulty" = self.difficulty;
			"DifficultyLocked" = self.difficulty_locked;
			"DragonFight" = self.dragon_fight.clone();
			"GameRules" = self.game_rules.clone();
			"GameType" = self.game_type;
			"LastPlayed" = self.last_played;
			"LevelName" = self.level_name.clone();
			"Player" = self.player.clone();
			"ScheduledEvents" = self.scheduled_events.clone();
			"ServerBrands" = self.server_brands.clone();
			"SpawnAngle" = self.spawn_angle;
			"SpawnX" = self.spawn_x;
			"SpawnY" = self.spawn_y;
			"SpawnZ" = self.spawn_z;
			"Time" = self.time;
			"Version" = self.version.clone();
			"WanderingTraderSpawnChance" = self.wandering_trader_spawn_chance;
			"WanderingTraderSpawnDelay" = self.wandering_trader_spawn_delay;
			"WasModded" = self.was_modded;
			"WorldGenSettings" = self.world_gen_settings.clone();
			"allowCommands" = self.allow_commands;
			"clearWeatherTime" = self.clear_weather_time;
			"hardcore" = self.hardcore;
			"initialized" = self.initialized;
			"rainTime" = self.rain_time;
			"raining" = self.raining;
			"thunderTime" = self.thunder_time;
			"thundering" = self.thundering;
			"version" = self.version2;
		);
		Tag::Compound(Map::from([("Data".to_owned(), Tag::Compound(data))]))
	}
}

impl DecodeNbt for Level {
	fn decode_nbt(nbt: Tag) -> McResult<Self> {
		if let Tag::Compound(mut map) = nbt {
			let mut data: Map = map_decoder!(map; "Data" -> Map);
			Ok(Level {
				border_center_x: map_decoder!(data; "BorderCenterX" -> f64),
				border_center_z: map_decoder!(data; "BorderCenterZ" -> f64),
				border_damage_per_block: map_decoder!(data; "BorderDamagePerBlock" -> f64),
				border_size: map_decoder!(data; "BorderSize" -> f64),
				border_size_lerp_target: map_decoder!(data; "BorderSizeLerpTarget" -> f64),
				border_size_lerp_time: map_decoder!(data; "BorderSizeLerpTime" -> i64),
				border_warning_blocks: map_decoder!(data; "BorderWarningBlocks" -> f64),
				border_warning_time: map_decoder!(data; "BorderWarningTime" -> f64),
				custom_boss_events: map_decoder!(data; "CustomBossEvents" -> Map),
				data_packs: map_decoder!(data; "DataPacks" -> Map),
				data_version: map_decoder!(data; "DataVersion" -> i32),
				day_time: map_decoder!(data; "DayTime" -> i64),
				difficulty: map_decoder!(data; "Difficulty" -> i8),
				difficulty_locked: map_decoder!(data; "DifficultyLocked" -> i8),
				dragon_fight: map_decoder!(data; "DragonFight" -> Map),
				game_rules: map_decoder!(data; "GameRules" -> Map),
				game_type: map_decoder!(data; "GameType" -> i32),
				last_played: map_decoder!(data; "LastPlayed" -> i64),
				level_name: map_decoder!(data; "LevelName" -> String),
				player: map_decoder!(data; "Player" -> Map),
				scheduled_events: map_decoder!(data; "ScheduledEvents" -> ListTag),
				server_brands: map_decoder!(data; "ServerBrands" -> ListTag),
				spawn_angle: map_decoder!(data; "SpawnAngle" -> f32),
				spawn_x: map_decoder!(data; "SpawnX" -> i32),
				spawn_y: map_decoder!(data; "SpawnY" -> i32),
				spawn_z: map_decoder!(data; "SpawnZ" -> i32),
				time: map_decoder!(data; "Time" -> i64),
				version: map_decoder!(data; "Version" -> Map),
				wandering_trader_spawn_chance: map_decoder!(data; "WanderingTraderSpawnChance" -> i32),
				wandering_trader_spawn_delay: map_decoder!(data; "WanderingTraderSpawnDelay" -> i32),
				was_modded: map_decoder!(data; "WasModded" -> i8),
				world_gen_settings: map_decoder!(data; "WorldGenSettings" -> Map),
				allow_commands: map_decoder!(data; "allowCommands" -> i8),
				clear_weather_time: map_decoder!(data; "clearWeatherTime" -> i32),
				hardcore: map_decoder!(data; "hardcore" -> i8),
				initialized: map_decoder!(data; "initialized" -> i8),
				rain_time: map_decoder!(data; "rainTime" -> i32),
				raining: map_decoder!(data; "raining" -> i8),
				thunder_time: map_decoder!(data; "thunderTime" -> i32),
				thundering: map_decoder!(data; "thundering" -> i8),
				version2: map_decoder!(data; "version" -> i32),
			})
		} else {
			return Err(McError::NbtDecodeError);
		}
	}
}