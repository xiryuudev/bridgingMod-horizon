// BridgingMod for Horizon: reach-around (bridge assist) placement.
//
// A port of squeeglii's BridgingMod (MIT) from Fabric/NeoForge to a Horizon
// native mixin bundle, rewritten in Rust. When the crosshair hits nothing and
// the player holds a placeable item, this searches the blocks along the view
// line for one that has a solid neighbour to build off of, in the direction
// the player is looking, and hands the game a synthetic block hit so the use
// places there. A small indicator beside the crosshair shows the direction,
// and a debug box shows the exact position (F3).
//
// This module is the game-free half: the search, the hit it makes and the
// settings. The helpers (`frame.rs`, `use.rs`, ...) only adapt the game to it.
// It is compiled into the game as `crate::game::bridgingmod`.

use std::cell::RefCell;

/// The mod's id: its settings folder and its key's line.
pub const ID: &str = "bridgingmod";
pub const NAME: &str = "BridgingMod";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str =
    "Reach-around placement: place blocks in reach-around gaps, with a direction indicator and F3 box.";
/// The toggle key's name, and its default (upstream's comma).
pub const KEY: &str = "toggle";
pub const DEFAULT_KEY: &str = "key.keyboard.comma";
/// Said in the action bar when the toggle key turns it on or off.
pub const ON: &str = "Bridging assist: ON";
pub const OFF: &str = "Bridging assist: OFF";

/// `Path.NEAR_ZERO`.
pub const NEAR_ZERO: f64 = 0.01;
/// `Path.CUBE_EXTENT`: the snap boxes are 60% of a block.
const CUBE_EXTENT: [f64; 3] = [0.5, 0.5, 0.5];
/// `GameSupport.TRAPDOOR_HEIGHT`.
const TRAPDOOR_HEIGHT: f64 = 3.0 / 16.0;
/// `GameSupport.SLAB_HEIGHT`.
const SLAB_HEIGHT: f64 = 8.0 / 16.0;
/// `PathTraversalHandler.DIRECTION_SIMILARITY_THRESHOLD`.
const DIRECTION_SIMILARITY_THRESHOLD: f64 = 0.1;

/// A block position (`net.minecraft.core.BlockPos`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    pub fn offset(self, d: Dir) -> Self {
        let v = d.unit();
        Self::new(self.x + v[0], self.y + v[1], self.z + v[2])
    }
    pub fn containing(v: [f64; 3]) -> Self {
        Self::new(
            v[0].floor() as i32,
            v[1].floor() as i32,
            v[2].floor() as i32,
        )
    }
    pub fn center(self) -> [f64; 3] {
        [
            f64::from(self.x) + 0.5,
            f64::from(self.y) + 0.5,
            f64::from(self.z) + 0.5,
        ]
    }
    pub fn bottom_center(self) -> [f64; 3] {
        [
            f64::from(self.x) + 0.5,
            f64::from(self.y),
            f64::from(self.z) + 0.5,
        ]
    }
    pub fn dist_manhattan(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }
}

/// A block face (`net.minecraft.core.Direction`), vanilla's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dir {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Dir {
    pub const ALL: [Dir; 6] = [
        Dir::Down,
        Dir::Up,
        Dir::North,
        Dir::South,
        Dir::West,
        Dir::East,
    ];
    pub fn unit(self) -> [i32; 3] {
        match self {
            Dir::Down => [0, -1, 0],
            Dir::Up => [0, 1, 0],
            Dir::North => [0, 0, -1],
            Dir::South => [0, 0, 1],
            Dir::West => [-1, 0, 0],
            Dir::East => [1, 0, 0],
        }
    }
    pub fn opposite(self) -> Self {
        match self {
            Dir::Down => Dir::Up,
            Dir::Up => Dir::Down,
            Dir::North => Dir::South,
            Dir::South => Dir::North,
            Dir::West => Dir::East,
            Dir::East => Dir::West,
        }
    }
    pub fn is_horizontal(self) -> bool {
        matches!(self, Dir::North | Dir::South | Dir::West | Dir::East)
    }
}

/// `PlacementAxisMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisMode {
    Horizontal,
    Vertical,
    Both,
}

impl AxisMode {
    pub fn is_direction_enabled(self, d: Dir) -> bool {
        match self {
            AxisMode::Both => true,
            AxisMode::Horizontal => d.is_horizontal(),
            AxisMode::Vertical => !d.is_horizontal(),
        }
    }
    fn from_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "horizontal" => Some(AxisMode::Horizontal),
            "vertical" => Some(AxisMode::Vertical),
            "both" => Some(AxisMode::Both),
            _ => None,
        }
    }
    fn name(self) -> &'static str {
        match self {
            AxisMode::Horizontal => "horizontal",
            AxisMode::Vertical => "vertical",
            AxisMode::Both => "both",
        }
    }
    fn label(self) -> &'static str {
        match self {
            AxisMode::Horizontal => "Horizontal",
            AxisMode::Vertical => "Vertical",
            AxisMode::Both => "Both",
        }
    }
    fn next(self) -> Self {
        match self {
            AxisMode::Horizontal => AxisMode::Vertical,
            AxisMode::Vertical => AxisMode::Both,
            AxisMode::Both => AxisMode::Horizontal,
        }
    }
}

/// `PlacementAxisModeOverride` (the crouched case); `Fallback` keeps the base mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisOverride {
    Horizontal,
    Vertical,
    Both,
    Fallback,
}

impl AxisOverride {
    pub fn resolve(self, base: AxisMode) -> AxisMode {
        match self {
            AxisOverride::Fallback => base,
            AxisOverride::Horizontal => AxisMode::Horizontal,
            AxisOverride::Vertical => AxisMode::Vertical,
            AxisOverride::Both => AxisMode::Both,
        }
    }
    fn from_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "horizontal" => Some(AxisOverride::Horizontal),
            "vertical" => Some(AxisOverride::Vertical),
            "both" => Some(AxisOverride::Both),
            "fallback" => Some(AxisOverride::Fallback),
            _ => None,
        }
    }
    fn name(self) -> &'static str {
        match self {
            AxisOverride::Horizontal => "horizontal",
            AxisOverride::Vertical => "vertical",
            AxisOverride::Both => "both",
            AxisOverride::Fallback => "fallback",
        }
    }
    fn label(self) -> &'static str {
        match self {
            AxisOverride::Horizontal => "Horizontal",
            AxisOverride::Vertical => "Vertical",
            AxisOverride::Both => "Both",
            AxisOverride::Fallback => "As above",
        }
    }
    fn next(self) -> Self {
        match self {
            AxisOverride::Horizontal => AxisOverride::Vertical,
            AxisOverride::Vertical => AxisOverride::Both,
            AxisOverride::Both => AxisOverride::Fallback,
            AxisOverride::Fallback => AxisOverride::Horizontal,
        }
    }
}

/// `BridgingAdjacency`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adjacency {
    Disabled,
    Faces,
    Edges,
    Corners,
}

impl Adjacency {
    pub fn enabled(self) -> bool {
        self != Adjacency::Disabled
    }
    fn from_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "disabled" => Some(Adjacency::Disabled),
            "faces" => Some(Adjacency::Faces),
            "edges" => Some(Adjacency::Edges),
            "corners" => Some(Adjacency::Corners),
            _ => None,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Adjacency::Disabled => "disabled",
            Adjacency::Faces => "faces",
            Adjacency::Edges => "edges",
            Adjacency::Corners => "corners",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Adjacency::Disabled => "Off",
            Adjacency::Faces => "Faces",
            Adjacency::Edges => "Edges",
            Adjacency::Corners => "Corners",
        }
    }
    fn next(self) -> Self {
        match self {
            Adjacency::Disabled => Adjacency::Faces,
            Adjacency::Faces => Adjacency::Edges,
            Adjacency::Edges => Adjacency::Corners,
            Adjacency::Corners => Adjacency::Disabled,
        }
    }
}

/// The mod's settings, with upstream's defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub enabled: bool,
    /// Percent, 0..=100 (`minBridgeDistance`).
    pub min_bridge_distance: f64,
    pub only_bridge_when_crouched: bool,
    pub supported_axes: AxisMode,
    pub supported_axes_when_crouched: AxisOverride,
    pub delay_post_bridging: i32,
    pub show_crosshair: bool,
    pub show_outline: bool,
    pub show_outline_even_when_not_bridging: bool,
    pub non_bridge_respects_crouch_rules: bool,
    /// ARGB, upstream's `Color(0, 0, 0, 0.4f)`.
    pub outline_colour: u32,
    pub skip_torch_bridging: bool,
    pub enable_slab_assist: bool,
    pub enable_non_solid_replace: bool,
    pub bridging_snap_strength: f64,
    pub bridging_adjacency: Adjacency,
    pub show_debug_highlight: bool,
    pub show_non_bridging_debug_highlight: bool,
    pub show_debug_trace: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            min_bridge_distance: 20.0,
            only_bridge_when_crouched: false,
            supported_axes: AxisMode::Both,
            supported_axes_when_crouched: AxisOverride::Fallback,
            delay_post_bridging: 4,
            show_crosshair: true,
            show_outline: true,
            show_outline_even_when_not_bridging: false,
            non_bridge_respects_crouch_rules: true,
            outline_colour: 0x66_00_00_00,
            skip_torch_bridging: true,
            enable_slab_assist: true,
            enable_non_solid_replace: true,
            bridging_snap_strength: 1.0,
            bridging_adjacency: Adjacency::Corners,
            show_debug_highlight: true,
            show_non_bridging_debug_highlight: false,
            show_debug_trace: false,
        }
    }
}

impl Config {
    /// From the stored values (`get` reads one key's file); the defaults for
    /// the ones missing or unreadable.
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let defaults = Config::default();
        let flag = |key: &str, default: bool| match get(key).as_deref().map(str::trim) {
            Some("true") => true,
            Some("false") => false,
            _ => default,
        };
        let number = |key: &str, default: f64| {
            get(key)
                .as_deref()
                .map(str::trim)
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .unwrap_or(default)
        };
        Self {
            enabled: flag("enabled", defaults.enabled),
            min_bridge_distance: number("min_bridge_distance", defaults.min_bridge_distance)
                .clamp(0.0, 100.0),
            only_bridge_when_crouched: flag(
                "only_bridge_when_crouched",
                defaults.only_bridge_when_crouched,
            ),
            supported_axes: get("supported_axes")
                .as_deref()
                .and_then(AxisMode::from_name)
                .unwrap_or(defaults.supported_axes),
            supported_axes_when_crouched: get("supported_axes_when_crouched")
                .as_deref()
                .and_then(AxisOverride::from_name)
                .unwrap_or(defaults.supported_axes_when_crouched),
            delay_post_bridging: number("delay_post_bridging", defaults.delay_post_bridging.into())
                .round()
                .clamp(0.0, 20.0) as i32,
            show_crosshair: flag("show_crosshair", defaults.show_crosshair),
            show_outline: flag("show_outline", defaults.show_outline),
            show_outline_even_when_not_bridging: flag(
                "show_outline_even_when_not_bridging",
                defaults.show_outline_even_when_not_bridging,
            ),
            non_bridge_respects_crouch_rules: flag(
                "non_bridge_respects_crouch_rules",
                defaults.non_bridge_respects_crouch_rules,
            ),
            outline_colour: number("outline_colour", f64::from(defaults.outline_colour))
                .clamp(0.0, 0xFFFF_FFFFu64 as f64) as u32,
            skip_torch_bridging: flag("skip_torch_bridging", defaults.skip_torch_bridging),
            enable_slab_assist: flag("enable_slab_assist", defaults.enable_slab_assist),
            enable_non_solid_replace: flag(
                "enable_non_solid_replace",
                defaults.enable_non_solid_replace,
            ),
            bridging_snap_strength: number(
                "bridging_snap_strength",
                defaults.bridging_snap_strength,
            )
            .clamp(0.0, 1.0),
            bridging_adjacency: get("bridging_adjacency")
                .as_deref()
                .and_then(Adjacency::from_name)
                .unwrap_or(defaults.bridging_adjacency),
            show_debug_highlight: flag("show_debug_highlight", defaults.show_debug_highlight),
            show_non_bridging_debug_highlight: flag(
                "show_non_bridging_debug_highlight",
                defaults.show_non_bridging_debug_highlight,
            ),
            show_debug_trace: flag("show_debug_trace", defaults.show_debug_trace),
        }
    }

    /// The settings page's buttons, in the order `click` takes them.
    pub fn labels(&self) -> Vec<String> {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("Bridging assist: {}", on(self.enabled)),
            format!("Only while crouching: {}", on(self.only_bridge_when_crouched)),
            format!("Bridge axes: {}", self.supported_axes.label()),
            format!(
                "Axes while crouching: {}",
                self.supported_axes_when_crouched.label()
            ),
            format!("Minimum distance: {}%", self.min_bridge_distance.round() as i32),
            format!("Delay after bridging: {} ticks", self.delay_post_bridging),
            format!("Crosshair indicator: {}", on(self.show_crosshair)),
            format!("Debug box: {}", on(self.show_outline)),
            format!("Slab assist: {}", on(self.enable_slab_assist)),
            format!(
                "Replace plants and snow: {}",
                on(self.enable_non_solid_replace)
            ),
            format!("Skip torches: {}", on(self.skip_torch_bridging)),
            format!("Missed corners: {}", self.bridging_adjacency.label()),
            format!("F3 debug box: {}", on(self.show_debug_highlight)),
            format!("F3 trace: {}", on(self.show_debug_trace)),
        ]
    }

    /// A click on button `i` of `labels`.
    pub fn click(&mut self, i: usize) {
        match i {
            0 => self.enabled = !self.enabled,
            1 => self.only_bridge_when_crouched = !self.only_bridge_when_crouched,
            2 => self.supported_axes = self.supported_axes.next(),
            3 => self.supported_axes_when_crouched = self.supported_axes_when_crouched.next(),
            4 => {
                self.min_bridge_distance = match self.min_bridge_distance.round() as i32 {
                    d if d < 50 => 50.0,
                    d if d < 100 => 100.0,
                    _ => 0.0,
                }
            }
            5 => {
                self.delay_post_bridging = match self.delay_post_bridging {
                    0 => 2,
                    2 => 4,
                    4 => 0,
                    _ => 4,
                }
            }
            6 => self.show_crosshair = !self.show_crosshair,
            7 => self.show_outline = !self.show_outline,
            8 => self.enable_slab_assist = !self.enable_slab_assist,
            9 => self.enable_non_solid_replace = !self.enable_non_solid_replace,
            10 => self.skip_torch_bridging = !self.skip_torch_bridging,
            11 => self.bridging_adjacency = self.bridging_adjacency.next(),
            12 => self.show_debug_highlight = !self.show_debug_highlight,
            13 => self.show_debug_trace = !self.show_debug_trace,
            _ => {}
        }
    }

    /// The values to store, one file per key.
    pub fn stored(&self) -> Vec<(&'static str, String)> {
        let flag = |b: bool| if b { "true" } else { "false" }.to_owned();
        vec![
            ("enabled", flag(self.enabled)),
            (
                "only_bridge_when_crouched",
                flag(self.only_bridge_when_crouched),
            ),
            ("supported_axes", self.supported_axes.name().to_owned()),
            (
                "supported_axes_when_crouched",
                self.supported_axes_when_crouched.name().to_owned(),
            ),
            (
                "min_bridge_distance",
                format!("{:.1}", self.min_bridge_distance),
            ),
            (
                "delay_post_bridging",
                self.delay_post_bridging.to_string(),
            ),
            ("show_crosshair", flag(self.show_crosshair)),
            ("show_outline", flag(self.show_outline)),
            ("enable_slab_assist", flag(self.enable_slab_assist)),
            (
                "enable_non_solid_replace",
                flag(self.enable_non_solid_replace),
            ),
            ("skip_torch_bridging", flag(self.skip_torch_bridging)),
            (
                "bridging_adjacency",
                self.bridging_adjacency.name().to_owned(),
            ),
            ("show_debug_highlight", flag(self.show_debug_highlight)),
            ("show_debug_trace", flag(self.show_debug_trace)),
        ]
    }
}

/// A direction whose side may be built off of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Assist {
    /// Where the block goes.
    pub pos: Pos,
    /// The checked side: the support block is at `pos.offset(side)`.
    pub side: Dir,
}

/// A synthetic block hit, as the game's use-on-block path wants it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub pos: Pos,
    /// The clicked face; also the direction the item is used in.
    pub face: Dir,
    pub point: [f64; 3],
}

/// What the search asks the world.
pub trait World {
    fn is_air(&self, pos: Pos) -> bool;
    fn is_replaceable(&self, pos: Pos) -> bool;
    fn is_liquid(&self, pos: Pos) -> bool;
    fn player_overlaps_block(&self, pos: Pos) -> bool;
    fn holding_placeable(&self) -> bool;
    fn crouching(&self) -> bool;
}

/// What the slab assist needs.
pub trait SlabWorld {
    /// The block at `pos` is a slab; `Some(true)` its top half.
    fn slab_half(&self, pos: Pos) -> Option<bool>;
    fn holding_slab(&self) -> bool;
    fn holding_trapdoor(&self) -> bool;
}

/// One frame's/tick's answer, and the settings it was computed with.
struct State {
    config: Option<Config>,
    target: Option<Assist>,
    /// `BridgingStateTracker.lastKnownYFrac`.
    y_frac: f64,
    /// A bridging use is in progress: the crosshair was overridden.
    using: bool,
    /// The target of the use in progress.
    use_target: Option<Pos>,
    /// The mod image (the indicator sprites) has been uploaded this session.
    image_uploaded: bool,
    /// The last path, for the F3 trace.
    trace: Vec<Pos>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            config: None,
            target: None,
            y_frac: 0.0,
            using: false,
            use_target: None,
            image_uploaded: false,
            trace: Vec::new(),
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The settings, read once through `get` (one file per key).
pub fn config(get: impl Fn(&str) -> Option<String>) -> Config {
    STATE.with_borrow_mut(|s| *s.config.get_or_insert_with(|| Config::read(get)))
}

/// The settings, if some code read them already.
pub fn current_config() -> Option<Config> {
    STATE.with_borrow(|s| s.config)
}

/// Replaces the settings (the settings page or the key changed them).
pub fn set_config(config: Config) {
    STATE.with_borrow_mut(|s| s.config = Some(config));
}

/// The last computed target, if any.
pub fn target() -> Option<Assist> {
    STATE.with_borrow(|s| s.target)
}

/// The last path (for the F3 trace).
pub fn trace() -> Vec<Pos> {
    STATE.with_borrow(|s| s.trace.clone())
}

/// The player's last known Y fraction.
pub fn y_frac() -> f64 {
    STATE.with_borrow(|s| s.y_frac)
}

pub fn set_y_frac(v: f64) {
    STATE.with_borrow_mut(|s| s.y_frac = v);
}

/// Whether the indicator sprites still need uploading.
pub fn image_uploaded() -> bool {
    STATE.with_borrow(|s| s.image_uploaded)
}

pub fn mark_image_uploaded() {
    STATE.with_borrow_mut(|s| s.image_uploaded = true);
}

/// A bridging use started on `pos`: the crosshair now points at it.
pub fn begin_use(pos: Pos) {
    STATE.with_borrow_mut(|s| {
        s.using = true;
        s.use_target = Some(pos);
    });
}

/// The use that started ended: the target whose crosshair override is still
/// to undo, when one is in progress.
pub fn end_use() -> Option<Pos> {
    STATE.with_borrow_mut(|s| {
        let started = std::mem::replace(&mut s.using, false);
        let target = s.use_target.take();
        started.then_some(target).flatten()
    })
}

/// Forgets the target (disabled, left the world, a screen opened).
pub fn reset() {
    STATE.with_borrow_mut(|s| {
        s.target = None;
        s.trace.clear();
        s.using = false;
        s.use_target = None;
    });
}

/// The target for this frame: `None` unless a bridging target exists.
pub fn refresh<W: World>(
    world: &W,
    config: &Config,
    camera_pos: [f64; 3],
    camera_look: [f64; 3],
    player_eye: [f64; 3],
    player_look: [f64; 3],
    reach: f64,
) -> Option<Assist> {
    let found = closest_assist_target(
        world,
        config,
        camera_pos,
        camera_look,
        player_eye,
        player_look,
        reach,
    );
    let trace = if found.is_some() {
        view_block_path(config, camera_pos, camera_look, player_eye, player_look, reach)
    } else {
        Vec::new()
    };
    STATE.with_borrow_mut(|s| {
        s.target = found;
        s.trace = trace;
    });
    found
}

/// `PathTraversalHandler.getClosestAssistTarget`.
pub fn closest_assist_target<W: World>(
    world: &W,
    config: &Config,
    camera_pos: [f64; 3],
    camera_look: [f64; 3],
    player_eye: [f64; 3],
    player_look: [f64; 3],
    reach: f64,
) -> Option<Assist> {
    if !config.enabled || !world.holding_placeable() {
        return None;
    }
    let path = view_block_path(config, camera_pos, camera_look, player_eye, player_look, reach);
    let valid_sides = valid_assist_sides(camera_look);
    for pos in path {
        if !bridging_placement_allowed_at(world, config, pos) {
            continue;
        }
        if world.player_overlaps_block(pos) {
            continue;
        }
        for side in &valid_sides {
            if can_side_be_built_off_of(world, config, pos, *side) {
                return Some(Assist { pos, side: *side });
            }
        }
    }
    None
}

/// `PathTraversalHandler.getViewBlockPath`.
pub fn view_block_path(
    config: &Config,
    camera_pos: [f64; 3],
    camera_look: [f64; 3],
    player_eye: [f64; 3],
    player_look: [f64; 3],
    reach: f64,
) -> Vec<Pos> {
    let player_view = [
        player_eye[0] + player_look[0] * reach,
        player_eye[1] + player_look[1] * reach,
        player_eye[2] + player_look[2] * reach,
    ];
    let distance = dist(player_view, camera_pos);
    let min_distance = config.min_bridge_distance / 100.0;
    let far_vec = [
        camera_look[0] * distance,
        camera_look[1] * distance,
        camera_look[2] * distance,
    ];
    let near_vec = [
        far_vec[0] * min_distance,
        far_vec[1] * min_distance,
        far_vec[2] * min_distance,
    ];
    let start = Pos::containing([
        camera_pos[0] + near_vec[0],
        camera_pos[1] + near_vec[1],
        camera_pos[2] + near_vec[2],
    ]);
    let end = Pos::containing([
        camera_pos[0] + far_vec[0],
        camera_pos[1] + far_vec[1],
        camera_pos[2] + far_vec[2],
    ]);
    bresenham_voxels(config, start, end)
}

/// `Path.calculateBresenhamVoxels`.
pub fn bresenham_voxels(config: &Config, start: Pos, end: Pos) -> Vec<Pos> {
    let mut points = vec![start];
    let dx = (end.x - start.x).abs();
    let dy = (end.y - start.y).abs();
    let dz = (end.z - start.z).abs();
    let (x_step, y_step, z_step) = (
        if end.x >= start.x { 1 } else { -1 },
        if end.y >= start.y { 1 } else { -1 },
        if end.z >= start.z { 1 } else { -1 },
    );
    let mut working = start;
    let adjacency = config.bridging_adjacency.enabled();
    let push = |points: &mut Vec<Pos>, working: Pos| {
        if adjacency {
            points.extend(missed_points(
                points,
                working,
                start,
                end,
                config.bridging_snap_strength,
            ));
        }
        points.push(working);
    };
    if dx >= dy && dx >= dz {
        let (mut p1, mut p2) = (2 * dy - dx, 2 * dz - dx);
        for _ in 0..dx {
            working.x += x_step;
            if p1 >= 0 {
                working.y += y_step;
                p1 -= 2 * dx;
            }
            if p2 >= 0 {
                working.z += z_step;
                p2 -= 2 * dx;
            }
            p1 += 2 * dy;
            p2 += 2 * dz;
            push(&mut points, working);
        }
    } else if dy >= dx && dy >= dz {
        let (mut p1, mut p2) = (2 * dx - dy, 2 * dz - dy);
        for _ in 0..dy {
            working.y += y_step;
            if p1 >= 0 {
                working.x += x_step;
                p1 -= 2 * dy;
            }
            if p2 >= 0 {
                working.z += z_step;
                p2 -= 2 * dy;
            }
            p1 += 2 * dx;
            p2 += 2 * dz;
            push(&mut points, working);
        }
    } else {
        let (mut p1, mut p2) = (2 * dy - dz, 2 * dx - dz);
        for _ in 0..dz {
            working.z += z_step;
            if p1 >= 0 {
                working.y += y_step;
                p1 -= 2 * dz;
            }
            if p2 >= 0 {
                working.x += x_step;
                p2 -= 2 * dz;
            }
            p1 += 2 * dy;
            p2 += 2 * dx;
            push(&mut points, working);
        }
    }
    points
}

/// `Path.calculateMissedPoints`: blocks the line touches between two diagonal
/// steps, found with a box/line separating-axis test.
fn missed_points(
    points: &[Pos],
    new_point: Pos,
    line_start: Pos,
    line_end: Pos,
    extent_mult: f64,
) -> Vec<Pos> {
    let Some(&last_point) = points.last() else {
        return Vec::new();
    };
    let point_delta = Pos::new(
        new_point.x - last_point.x,
        new_point.y - last_point.y,
        new_point.z - last_point.z,
    );
    let diff = new_point.dist_manhattan(last_point);
    if !(0..=3).contains(&diff) || diff <= 1 {
        return Vec::new();
    }
    let mut review = Vec::new();
    if diff == 2 {
        for d in [
            [point_delta.x, 0, 0],
            [0, point_delta.y, 0],
            [0, 0, point_delta.z],
        ] {
            if d == [0, 0, 0] {
                continue;
            }
            review.push(Pos::new(
                last_point.x + d[0],
                last_point.y + d[1],
                last_point.z + d[2],
            ));
        }
    }
    if diff == 3 {
        for d in [
            [point_delta.x, 0, 0],
            [0, point_delta.y, 0],
            [0, 0, point_delta.z],
            [point_delta.x, point_delta.y, 0],
            [point_delta.x, 0, point_delta.z],
            [0, point_delta.y, point_delta.z],
        ] {
            if d == [0, 0, 0] {
                continue;
            }
            review.push(Pos::new(
                last_point.x + d[0],
                last_point.y + d[1],
                last_point.z + d[2],
            ));
        }
    }
    let extent = [
        CUBE_EXTENT[0] * extent_mult,
        CUBE_EXTENT[1] * extent_mult,
        CUBE_EXTENT[2] * extent_mult,
    ];
    let start = [
        f64::from(line_start.x),
        f64::from(line_start.y),
        f64::from(line_start.z),
    ];
    let end = [
        f64::from(line_end.x),
        f64::from(line_end.y),
        f64::from(line_end.z),
    ];
    review
        .into_iter()
        .filter(|pos| {
            let space = [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)];
            let line_start_d = [start[0] - space[0], start[1] - space[1], start[2] - space[2]];
            let line_end_d = [end[0] - space[0], end[1] - space[1], end[2] - space[2]];
            let mid = [
                (line_start_d[0] + line_end_d[0]) * 0.5,
                (line_start_d[1] + line_end_d[1]) * 0.5,
                (line_start_d[2] + line_end_d[2]) * 0.5,
            ];
            let line = [
                line_start_d[0] - mid[0],
                line_start_d[1] - mid[1],
                line_start_d[2] - mid[2],
            ];
            let line_ext = [line[0].abs(), line[1].abs(), line[2].abs()];
            if mid[0].abs() > extent[0] + line_ext[0] {
                return false;
            }
            if mid[1].abs() > extent[1] + line_ext[1] {
                return false;
            }
            if mid[2].abs() > extent[2] + line_ext[2] {
                return false;
            }
            if (mid[1] * line[2] - mid[2] * line[1]).abs()
                > extent[1] * line_ext[2] + extent[2] * line_ext[1]
            {
                return false;
            }
            if (mid[0] * line[2] - mid[2] * line[0]).abs()
                > extent[0] * line_ext[2] + extent[2] * line_ext[0]
            {
                return false;
            }
            if (mid[0] * line[1] - mid[1] * line[0]).abs()
                > extent[0] * line_ext[1] + extent[1] * line_ext[0]
            {
                return false;
            }
            true
        })
        .collect()
}

/// `PathTraversalHandler.getValidAssistSides`.
pub fn valid_assist_sides(look: [f64; 3]) -> Vec<Dir> {
    let mut sides = Vec::new();
    for d in Dir::ALL {
        let n = d.unit();
        let v = [f64::from(n[0]), f64::from(n[1]), f64::from(n[2])];
        let similarity = look[0] * v[0] + look[1] * v[1] + look[2] * v[2];
        if similarity < DIRECTION_SIMILARITY_THRESHOLD {
            continue;
        }
        sides.push(d.opposite());
    }
    sides
}

/// `PathTraversalHandler.canSideBeBuiltOffOf`.
pub fn can_side_be_built_off_of<W: World>(
    world: &W,
    config: &Config,
    placement_target: Pos,
    check_side: Dir,
) -> bool {
    let mut mode = config.supported_axes;
    if world.crouching() {
        mode = config.supported_axes_when_crouched.resolve(mode);
    }
    if !mode.is_direction_enabled(check_side) {
        return false;
    }
    let support = placement_target.offset(check_side);
    if world.is_air(support) || world.is_liquid(support) {
        return false;
    }
    !world.is_replaceable(support)
}

/// `PathTraversalHandler.isBridgingPlacementAllowedAt`.
pub fn bridging_placement_allowed_at<W: World>(world: &W, config: &Config, pos: Pos) -> bool {
    if config.enable_non_solid_replace {
        world.is_replaceable(pos)
    } else {
        world.is_air(pos)
    }
}

/// `Bridge.getDefaultPlaceAssistTarget`: the synthetic hit for an assist.
pub fn place_assist_target<S: SlabWorld>(
    slabs: &S,
    config: &Config,
    assist: Assist,
    y_frac: f64,
) -> Hit {
    let dir = assist.side.opposite();
    let pos = assist.pos;
    if config.enable_slab_assist && (slabs.holding_slab() || slabs.holding_trapdoor()) {
        if dir.is_horizontal() {
            if let Some(hit) = horizontal_slab_assist(pos, y_frac) {
                return hit;
            }
        } else if let Some(hit) = vertical_slab_assist(slabs, dir, pos) {
            return hit;
        }
    }
    Hit {
        pos,
        face: dir,
        point: pos.center(),
    }
}

/// `Bridge.handleHorizontalSlabAssist`.
fn horizontal_slab_assist(pos: Pos, y_frac: f64) -> Option<Hit> {
    let lower = y_frac > TRAPDOOR_HEIGHT - NEAR_ZERO && y_frac < SLAB_HEIGHT + NEAR_ZERO;
    let c = pos.bottom_center();
    let point = if lower {
        [c[0], c[1] + 0.1, c[2]]
    } else {
        [c[0], c[1] + 0.9, c[2]]
    };
    Some(Hit {
        pos,
        face: if lower { Dir::Up } else { Dir::Down },
        point,
    })
}

/// `Bridge.handleVerticalSlabAssist`: combine two slabs into a double slab.
fn vertical_slab_assist<S: SlabWorld>(slabs: &S, dir: Dir, pos: Pos) -> Option<Hit> {
    let support = pos.offset(dir.opposite());
    let top = slabs.slab_half(support)?;
    if top && dir != Dir::Down {
        return None;
    }
    if !top && dir != Dir::Up {
        return None;
    }
    // The game is told the *existing* slab was clicked, with the target
    // block's centre as the point, so the two slabs combine.
    Some(Hit {
        pos: support,
        face: dir,
        point: pos.center(),
    })
}

/// The indicator sprite for an assist: `PlacementAlignment.from(direction)`.
pub fn placement_alignment(side: Dir) -> &'static str {
    match side {
        Dir::Up => "down",
        Dir::Down => "up",
        _ => "horizontal",
    }
}

/// `Vec3.distanceTo`.
pub fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// `Mth.frac(player.getY())`.
pub fn fraction(y: f64) -> f64 {
    y - y.floor()
}

// --- The game adapter -------------------------------------------------------

impl crate::game::Game<'_> {
    /// The camera's position and view vector, as `GameRenderer.pick` uses them.
    pub(crate) fn bridging_aim(&self) -> Option<([f64; 3], [f64; 3])> {
        self.aim()
    }
    /// The player's eye position.
    pub(crate) fn bridging_eye(&self) -> Option<[f64; 3]> {
        self.eye_position()
    }
    /// The block pick's reach.
    pub(crate) fn bridging_reach(&self) -> f64 {
        self.block_reach
    }
    /// The living player's position.
    pub(crate) fn bridging_player_pos(&self) -> Option<[f64; 3]> {
        self.player
            .as_ref()
            .map(|p| [p.position.x, p.position.y, p.position.z])
    }
    /// The player is on the ground.
    pub(crate) fn bridging_on_ground(&self) -> bool {
        self.player.as_ref().is_some_and(|p| p.ground.on_ground)
    }
    /// Whether the block at `pos` is air.
    pub(crate) fn bridging_air(&self, pos: Pos) -> bool {
        self.world.block(pos.x, pos.y, pos.z) == self.world.air
    }
    /// Whether the block at `pos` can be replaced (grass, snow, water...).
    pub(crate) fn bridging_replaceable(&self, pos: Pos) -> bool {
        let state = self.world.block(pos.x, pos.y, pos.z);
        self.res.facts().get(state).replaceable
    }
    /// Whether the block at `pos` is a liquid.
    pub(crate) fn bridging_liquid(&self, pos: Pos) -> bool {
        let state = self.world.block(pos.x, pos.y, pos.z);
        !matches!(
            self.res.facts().get(state).fluid,
            horizon_client::blocks::Fluid::None
        )
    }
    /// The player's bounding box overlaps the block at `pos`.
    pub(crate) fn bridging_player_overlaps(&self, pos: Pos) -> bool {
        if self.spectator() {
            return false;
        }
        let Some(p) = self.player.as_ref() else {
            return false;
        };
        let half = f64::from(p.dimensions.width) / 2.0;
        let min = [
            p.position.x - half,
            p.position.y,
            p.position.z - half,
        ];
        let max = [
            p.position.x + half,
            p.position.y + f64::from(p.dimensions.height),
            p.position.z + half,
        ];
        let block_min = [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)];
        let block_max = [block_min[0] + 1.0, block_min[1] + 1.0, block_min[2] + 1.0];
        min[0] < block_max[0]
            && max[0] > block_min[0]
            && min[1] < block_max[1]
            && max[1] > block_min[1]
            && min[2] < block_max[2]
            && max[2] > block_min[2]
    }
    /// The default state of the block the held item places, from a hand.
    fn bridging_hand_block(
        &self,
        stack: Option<&horizon_client::items::ItemStack>,
    ) -> Option<horizon_world::registry::BlockStateId> {
        let stack = stack?;
        self.res.items().get(&stack.item).and_then(|info| info.block)
    }
    /// The blocks the two hands place, main hand first.
    pub(crate) fn bridging_held_blocks(
        &self,
    ) -> [Option<horizon_world::registry::BlockStateId>; 2] {
        [
            self.bridging_hand_block(self.held()),
            self.bridging_hand_block(self.offhand.as_ref()),
        ]
    }
    /// The player is crouching.
    pub(crate) fn bridging_crouching(&self) -> bool {
        self.keys
            .held(horizon_client::actions::Action::Sneak)
    }
    /// The block at `pos` is a slab, and which half (`Some(true)` = top).
    pub(crate) fn bridging_slab_half(&self, pos: Pos) -> Option<bool> {
        let state = self.world.block(pos.x, pos.y, pos.z);
        if !self.res.facts().get(state).name.ends_with("_slab") {
            return None;
        }
        let registry_state = self.pack.assets.registry.state(state)?;
        match registry_state.properties.get("type").map(String::as_str) {
            Some("top") => Some(true),
            Some("double") => None,
            _ => Some(false),
        }
    }
    /// The held item places a slab or a trapdoor.
    pub(crate) fn bridging_holds_slab(&self) -> [bool; 2] {
        let [main, off] = self.bridging_held_blocks();
        [main, off].map(|id| {
            id.is_some_and(|id| self.res.facts().get(id).name.ends_with("_slab"))
        })
    }
    /// The held item places a trapdoor.
    pub(crate) fn bridging_holds_trapdoor(&self) -> [bool; 2] {
        let [main, off] = self.bridging_held_blocks();
        [main, off].map(|id| {
            id.is_some_and(|id| self.res.facts().get(id).name.ends_with("_trapdoor"))
        })
    }
    /// Whether the block item at `id` is a torch and torch bridging is off.
    pub(crate) fn bridging_skips_torch(&self, id: Option<horizon_world::registry::BlockStateId>) -> bool {
        id.is_some_and(|id| {
            let name = &self.res.facts().get(id).name;
            name.contains("torch") && self.bridging_config().skip_torch_bridging
        })
    }
    /// The current settings, read once.
    pub(crate) fn bridging_config(&self) -> Config {
        let game = self;
        config(|key| game.native_setting(ID, key))
    }
}

/// The game as the search sees it; `hand` is 0 for the main hand, 1 for
/// the off hand (the slab assist reads the item that would place).
pub(in crate::game) struct GameWorld<'a, 'g> {
    game: &'a crate::game::Game<'g>,
    hand: usize,
}

impl<'a, 'g> GameWorld<'a, 'g> {
    pub(in crate::game) fn new(game: &'a crate::game::Game<'g>, hand: usize) -> Self {
        Self { game, hand }
    }
}

impl World for GameWorld<'_, '_> {
    fn is_air(&self, pos: Pos) -> bool {
        self.game.bridging_air(pos)
    }
    fn is_replaceable(&self, pos: Pos) -> bool {
        self.game.bridging_replaceable(pos)
    }
    fn is_liquid(&self, pos: Pos) -> bool {
        self.game.bridging_liquid(pos)
    }
    fn player_overlaps_block(&self, pos: Pos) -> bool {
        self.game.bridging_player_overlaps(pos)
    }
    fn holding_placeable(&self) -> bool {
        let [main, off] = self.game.bridging_held_blocks();
        let usable = |id: Option<horizon_world::registry::BlockStateId>| {
            id.is_some() && !self.game.bridging_skips_torch(id)
        };
        usable(main) || usable(off)
    }
    fn crouching(&self) -> bool {
        self.game.bridging_crouching()
    }
}

impl SlabWorld for GameWorld<'_, '_> {
    fn slab_half(&self, pos: Pos) -> Option<bool> {
        self.game.bridging_slab_half(pos)
    }
    fn holding_slab(&self) -> bool {
        self.game.bridging_holds_slab()[self.hand]
    }
    fn holding_trapdoor(&self) -> bool {
        self.game.bridging_holds_trapdoor()[self.hand]
    }
}

/// Whether an indicator should be drawn for `config` in this state: the mod
/// is on, and the crouch rule (if any) passes.
pub fn assist_visible(config: &Config, crouching: bool) -> bool {
    config.enabled && (!config.only_bridge_when_crouched || crouching)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    struct TestWorld {
        solid: HashSet<Pos>,
        crouching: bool,
        holding: bool,
    }
    impl TestWorld {
        fn new(solid: &[Pos]) -> Self {
            Self {
                solid: solid.iter().copied().collect(),
                crouching: false,
                holding: true,
            }
        }
    }
    impl World for TestWorld {
        fn is_air(&self, pos: Pos) -> bool {
            !self.solid.contains(&pos)
        }
        fn is_replaceable(&self, pos: Pos) -> bool {
            self.is_air(pos)
        }
        fn is_liquid(&self, _pos: Pos) -> bool {
            false
        }
        fn player_overlaps_block(&self, _pos: Pos) -> bool {
            false
        }
        fn holding_placeable(&self) -> bool {
            self.holding
        }
        fn crouching(&self) -> bool {
            self.crouching
        }
    }
    impl SlabWorld for TestWorld {
        fn slab_half(&self, _pos: Pos) -> Option<bool> {
            None
        }
        fn holding_slab(&self) -> bool {
            false
        }
        fn holding_trapdoor(&self) -> bool {
            false
        }
    }

    #[test]
    fn bresenham_straight_line_is_exact() {
        let path = bresenham_voxels(&Config::default(), Pos::new(0, 0, 0), Pos::new(0, 0, -8));
        assert_eq!(path.len(), 9);
        for (i, p) in path.iter().enumerate() {
            assert_eq!(*p, Pos::new(0, 0, -(i as i32)));
        }
    }

    #[test]
    fn valid_sides() {
        assert_eq!(valid_assist_sides([0.0, 0.0, -1.0]), vec![Dir::South]);
        assert_eq!(valid_assist_sides([0.0, -1.0, 0.0]), vec![Dir::Up]);
        assert_eq!(valid_assist_sides([1.0, 0.0, 0.0]), vec![Dir::West]);
    }

    #[test]
    fn assist_beside_a_wall() {
        let config = Config::default();
        let mut solid = Vec::new();
        for x in -2..=2 {
            for y in 60..=64 {
                for z in -2..=2 {
                    if z == -2 {
                        solid.push(Pos::new(x, y, z));
                    }
                }
                solid.push(Pos::new(x, 60, -1));
                solid.push(Pos::new(x, 60, 0));
            }
        }
        let world = TestWorld::new(&solid);
        let assist = closest_assist_target(
            &world,
            &config,
            [0.5, 62.5, 0.5],
            [0.0, 0.0, -1.0],
            [0.5, 62.5, 0.5],
            [0.0, 0.0, -1.0],
            4.5,
        )
        .expect("an assist beside the wall");
        assert!(valid_assist_sides([0.0, 0.0, -1.0]).contains(&assist.side));
        assert!(world.solid.contains(&assist.pos.offset(assist.side)));
    }

    #[test]
    fn config_round_trip() {
        let stored: Vec<(&str, String)> = Config::default()
            .stored()
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect();
        let get = |key: &str| {
            stored
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(Config::read(get), Config::default());
        let mut c = Config::default();
        c.click(0);
        c.click(2);
        assert_eq!(c.enabled, false);
        assert_eq!(c.supported_axes, AxisMode::Vertical);
    }

    #[test]
    fn place_target_uses_opposite() {
        struct NoSlabs;
        impl SlabWorld for NoSlabs {
            fn slab_half(&self, _pos: Pos) -> Option<bool> {
                None
            }
            fn holding_slab(&self) -> bool {
                false
            }
            fn holding_trapdoor(&self) -> bool {
                false
            }
        }
        let hit = place_assist_target(
            &NoSlabs,
            &Config::default(),
            Assist {
                pos: Pos::new(3, 64, -2),
                side: Dir::Up,
            },
            0.0,
        );
        assert_eq!(hit.pos, Pos::new(3, 64, -2));
        assert_eq!(hit.face, Dir::Down);
        assert_eq!(hit.point, [3.5, 64.5, -1.5]);
    }

    #[test]
    fn fraction_of_y() {
        assert!((fraction(64.25) - 0.25).abs() < 1e-9);
    }
}
