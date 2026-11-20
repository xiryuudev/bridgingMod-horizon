// The game, as `runtime`'s search sees it.
//
// `runtime` is the same game-free code as the 1.0.1 `.hnmod`; this module is
// the only place that knows about the SDK. It answers the two traits the
// search asks for — `World` and `SlabWorld` — from handle reads, so the
// algorithm above it does not change between containers.

use mod_api::prelude::*;
use mod_api::{BlockState, ClientHost, Hand, HostBlockStates, LevelRef, PlayerRef};

use crate::runtime::{Config, Pos, SlabWorld, World};

/// Where the search reads the world from, and for which hand.
pub struct GameWorld<'a> {
    host: &'a dyn ClientHost,
    level: LevelRef,
    player: PlayerRef,
    /// 0 for the main hand, 1 for the off hand.
    hand: usize,
    crouching: bool,
}

/// What a frame needs from the game before the search runs.
#[derive(Clone, Copy, Debug)]
pub struct FrameInputs {
    pub level: LevelRef,
    pub player: PlayerRef,
    pub eye: [f64; 3],
    pub look: [f64; 3],
    pub reach: f64,
    pub crouching: bool,
    pub on_ground: bool,
    pub feet: [f64; 3],
    /// A block or entity is already under the crosshair.
    pub has_target: bool,
}

impl<'a> GameWorld<'a> {
    pub fn new(host: &'a dyn ClientHost, inputs: &FrameInputs, hand: usize) -> Self {
        Self {
            host,
            level: inputs.level,
            player: inputs.player,
            hand,
            crouching: inputs.crouching,
        }
    }

    fn state(&self, pos: Pos) -> Option<BlockState> {
        self.level
            .block_state(self.host, [pos.x, pos.y, pos.z])
    }

    /// The block's name (`minecraft:oak_slab`), for the slab, trapdoor and
    /// torch checks.
    fn name_of(&self, state: BlockState) -> String {
        state.block().id(self.host)
    }

    /// The block the held item places, by name, or none.
    fn held_name(&self) -> Option<String> {
        let hand = if self.hand == 0 { Hand::Main } else { Hand::Off };
        let stack = self.host.living(self.player.entity())?.hand_item(self.host, hand)?;
        let block = self.host.item_block(stack.item())?;
        Some(block.id(self.host))
    }
}

impl World for GameWorld<'_> {
    fn is_air(&self, pos: Pos) -> bool {
        match self.state(pos) {
            Some(state) => {
                let name = self.name_of(state);
                name == "minecraft:air" || name == "minecraft:cave_air" || name == "minecraft:void_air"
            }
            None => true,
        }
    }

    fn is_replaceable(&self, pos: Pos) -> bool {
        // A block is replaceable when it is air or a plant/snow/fluid the mod
        // may overwrite; the SDK exposes the block's own flag through the
        // metadata read on the state's block.
        self.state(pos)
            .is_some_and(|state| self.host.block_states().replaceable(state).unwrap_or(false))
    }

    fn is_liquid(&self, pos: Pos) -> bool {
        self.level
            .fluid_state(self.host, [pos.x, pos.y, pos.z])
            .is_some_and(|fluid| !fluid.is_empty())
    }

    fn player_overlaps_block(&self, pos: Pos) -> bool {
        let Some(bounds) = self.player.entity().bounding_box(self.host) else {
            return false;
        };
        let (min, max) = (bounds[0], bounds[1]);
        let block_min = [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)];
        let block_max = [block_min[0] + 1.0, block_min[1] + 1.0, block_min[2] + 1.0];
        min[0] < block_max[0]
            && max[0] > block_min[0]
            && min[1] < block_max[1]
            && max[1] > block_min[1]
            && min[2] < block_max[2]
            && max[2] > block_min[2]
    }

    fn holding_placeable(&self) -> bool {
        self.held_name().is_some()
    }

    fn crouching(&self) -> bool {
        self.crouching
    }
}

impl SlabWorld for GameWorld<'_> {
    fn slab_half(&self, pos: Pos) -> Option<bool> {
        let state = self.state(pos)?;
        if !self.name_of(state).ends_with("_slab") {
            return None;
        }
        match self
            .level
            .block_property(self.host, [pos.x, pos.y, pos.z], "type")
            .as_deref()
        {
            Some("top") => Some(true),
            Some("double") => None,
            _ => Some(false),
        }
    }

    fn holding_slab(&self) -> bool {
        self.held_name().is_some_and(|n| n.ends_with("_slab"))
    }

    fn holding_trapdoor(&self) -> bool {
        self.held_name().is_some_and(|n| n.ends_with("_trapdoor"))
    }
}

/// The mod's settings, read from the page under Options > Mods.
///
/// The page stores the same keys the 1.0.1 `.hnmod` wrote, so a player who
/// moves over keeps their choices.
pub fn config(host: &dyn ClientHost, mod_id: &str) -> Config {
    crate::runtime::config(|key| host.setting(mod_id, key))
}

/// The hand whose item would place: the main hand's, or the off hand's when
/// the main hand holds no block (vanilla tries them in that order).
pub fn placing_hand(host: &dyn ClientHost, player: PlayerRef) -> usize {
    let held = |hand| {
        host.living(player.entity())
            .and_then(|living| living.hand_item(host, hand))
            .and_then(|stack| host.item_block(stack.item()))
            .is_some()
    };
    if held(Hand::Main) { 0 } else { 1 }
}
