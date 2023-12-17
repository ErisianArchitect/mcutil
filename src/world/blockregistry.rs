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
	states: HashMap<u32, BlockState>,
	counter: AtomicU32,
}

impl BlockRegistry {
	pub fn new() -> Self {
		let air = BlockState::new("minecraft:air", BlockProperties::none());
		Self {
			ids: HashMap::from([(air.clone(), 0)]),
			states: HashMap::from([(0, air)]),
			counter: AtomicU32::new(1),
		}
	}

	/// Registers a [BlockState] with the registry and returns the ID.
	/// The returned ID can be used to acquire a [BlockState].
	pub fn register(&mut self, state: &BlockState) -> u32 {
		self.ids.get(state)
			.map(|id| *id)
			.unwrap_or_else(|| {
				let id = self.counter.fetch_add(1, Ordering::SeqCst);
				self.ids.insert(state.clone(), id);
				self.states.insert(id, state.clone());
				id
			})
	}

	/// Gets a [BlockState] from the registry by ID.
	pub fn get(&self, id: u32) -> Option<BlockState> {
		self.states.get(&id).map(|state| state.clone())
	}
}