#![allow(unused)]

use std::borrow::Borrow;
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

	pub fn len(&self) -> usize {
		self.states.len()
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

	/// Registers the air [BlockState].
	pub fn register_air(mut self) -> Self {
		self.register(BlockState::air());
		self
	}

	/// Registers a [BlockState] with the registry and returns the ID.
	/// The returned ID can be used to acquire a [BlockState].
	pub fn register<T: Borrow<BlockState>>(&mut self, state: T) -> u32 {
		self.ids.get(state.borrow())
			.map(|&id| id)
			.unwrap_or_else(|| {
				let state = state.borrow().clone();
				let id = self.states.len() as u32;
				self.ids.insert(state.clone(), id);
				self.states.push(state);
				id
			})
	}

	/// Finds the ID of a [BlockState] that has already been registered.
	pub fn find<T: Borrow<BlockState>>(&self, state: T) -> Option<u32> {
		if let Some(&id) = self.ids.get(state.borrow()) {
			Some(id)
		} else {
			None
		}
	}

	/// Gets a [BlockState] from the registry by ID.
	pub fn get(&self, id: u32) -> Option<&BlockState> {
		if (id as usize) < self.states.len() {
			Some(&self.states[id as usize])
		} else {
			None
		}
	}
}