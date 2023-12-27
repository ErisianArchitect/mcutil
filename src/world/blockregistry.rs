#![allow(unused)]

use std::sync::atomic::{
	AtomicU32,
	Ordering,
};

use std::collections::HashMap;

use super::blockstate::*;

/*
BlockRegistry handles all blocks that are used in a world.
Each block will have a unique ID assigned to it when it is added to
the registry. Blocks are never removed from the registry. They remain
in the registry for as long as the registry exists.
*/
pub struct BlockRegistry {
	ids: HashMap<BlockState, u32>,
	states: Vec<BlockState>,
}

impl BlockRegistry {
	pub fn new() -> Self {
		Self {
			ids: HashMap::new(),
			states: Vec::new(),
		}
	}

	/// Creates a block registry with "minecraft:air" registered in
	/// the first slot (index/id 0).
	pub fn with_air() -> Self {
		let air = BlockState::air();
		Self {
			ids: HashMap::from([(air.clone(), 0)]),
			states: Vec::from([air])
		}
	}

	pub fn register_air(mut self) -> Self {
		self.register(BlockState::air());
		self
	}

	/// Registers a [BlockState] with the registry and returns the ID.
	/// The returned ID can be used to acquire a [BlockState].
	pub fn register(&mut self, state: BlockState) -> u32 {
		self.ids.get(&state)
			.map(|id| *id)
			.unwrap_or_else(|| {
				let id = self.states.len() as u32;
				self.ids.insert(state.clone(), id);
				self.states.push(state.clone());
				id
			})
	}

	/// Gets a [BlockState] from the registry by ID.
	pub fn get(&self, id: u32) -> Option<BlockState> {
		if (id as usize) < self.states.len() {
			Some(self.states[id as usize].clone())
		} else {
			None
		}
	}
}