//! BridgingMod for Horizon: reach-around placement ("bridge assist").
//!
//! A port of squeeglii's BridgingMod (MIT) to the Horizon mod SDK, replacing
//! the retired `.hnmod` mixin bundle. `runtime` is the search, the synthetic
//! hit and the settings, unchanged from 1.0.1; `adapter` gives it a world
//! backed by the SDK's `ClientHost`; this file wires the hooks.
//!
//! The bridge click goes out as the game's own `use_item_on` packet, built
//! here and sent through `ClientHost::send_packet`, so the game needs no
//! compiled-in hook and the server sees an ordinary right click on a block.

mod adapter;
mod place;
mod runtime;

/// The player's block reach, as `Game.block_reach` reported it in the 1.0.1
/// bundle: vanilla's 4.5 blocks plus the assist's own margin.
const BLOCK_REACH: f64 = 4.5;

use mod_api::prelude::*;
use mod_api::{
    ClientHost, ClientTickPostEvent, DrawContext, Flow, GuiLayerPostEvent, Hooks, InputKeyEvent,
    InteractionKey, InteractionKeyMappingTriggeredEvent, KeyAction, KeyRegistration, Registrar,
    RegistrationContext, SettingSpec,
};

use adapter::{FrameInputs, GameWorld};
use runtime::{Config, ID, Pos};

use std::sync::Mutex;

/// The key's id within the mod, and its default (upstream's comma).
pub const KEY: &str = "toggle";
pub const DEFAULT_KEY: &str = "key.keyboard.comma";

/// The state kept between frames: the last answer, and the frame's inputs so
/// a click can rebuild the same answer at the moment it happens.
#[derive(Default)]
struct State {
    target: Option<runtime::Assist>,
    inputs: Option<(FrameInputs, Config)>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(State::default))
}

#[derive(Default)]
struct BridgingMod;

/// The settings page under Options > Mods, in the same order as the 1.0.1
/// `.hnmod`, so a returning player finds them where they were.
pub fn settings() -> Vec<SettingSpec> {
    vec![
        SettingSpec::toggle("enabled", "Bridging assist", true),
        SettingSpec::toggle("only_crouch", "Only while crouching", false),
        SettingSpec::choice(
            "axes",
            "Bridge axes",
            ["Both", "Horizontal", "Vertical"],
            0,
        ),
        SettingSpec::choice(
            "axes_crouched",
            "Axes while crouching",
            ["As above", "Both", "Horizontal", "Vertical"],
            0,
        ),
        SettingSpec::range("min_distance", "Minimum distance", 0.0, 100.0, 5.0, 20.0),
        SettingSpec::range("delay", "Delay after bridging", 0.0, 40.0, 1.0, 4.0),
        SettingSpec::toggle("show_crosshair", "Crosshair indicator", true),
        SettingSpec::toggle("show_outline", "Debug box", true),
        SettingSpec::toggle("slab_assist", "Slab assist", true),
        SettingSpec::toggle("replace_plants", "Replace plants and snow", true),
        SettingSpec::toggle("skip_torches", "Skip torches", true),
        SettingSpec::choice(
            "adjacency",
            "Missed corners",
            ["Off", "Faces", "Edges", "Corners"],
            2,
        ),
        SettingSpec::toggle("show_debug_highlight", "F3 debug box", true),
        SettingSpec::toggle("show_debug_trace", "F3 trace", false),
        SettingSpec::toggle("debug_chat", "Debug messages in chat", false),
    ]
}

impl BridgingMod {
    /// One frame: where the crosshair would bridge to, if it hits nothing.
    fn frame(&self, host: &mut dyn ClientHost) {
        if !host.on(ID) {
            with_state(|s| s.target = None);
            return;
        }
        let config = adapter::config(host, ID);
        // The click hook needs this frame's inputs whether or not the search
        // runs or the assist is on: it rebuilds the answer at the click.
        if let Some(inputs) = frame_inputs(host) {
            with_state(|s| s.inputs = Some((inputs, config)));
        }
        if !config.enabled || !host.playing() {
            with_state(|s| s.target = None);
            return;
        }
        let Some((inputs, _)) = with_state(|s| s.inputs) else {
            with_state(|s| s.target = None);
            return;
        };
        // Vanilla can place or interact: no reach-around.
        if inputs.has_target {
            with_state(|s| s.target = None);
            return;
        }
        // The slab assist's Y fraction, as `Mth.frac(player.getY())`.
        if inputs.on_ground {
            runtime::set_y_frac(runtime::fraction(inputs.feet[1]));
        }
        let world = GameWorld::new(host, &inputs, 0);
        runtime::refresh(
            &world,
            &config,
            inputs.eye,
            inputs.look,
            inputs.eye,
            inputs.look,
            inputs.reach,
        );
        let target = runtime::target();
        with_state(|s| s.target = target);
    }

    /// The key: bridging assist on or off, with the action-bar line.
    fn key(&self, host: &mut dyn ClientHost, key: &str, pressed: bool) {
        if !host.on(ID) || !host.playing() {
            return;
        }
        let Some((bound, _)) = host.mod_key(ID, KEY) else {
            return;
        };
        if !runtime::toggles_on(&bound, key, pressed) {
            return;
        }
        let mut config = adapter::config(host, ID);
        config.enabled = !config.enabled;
        runtime::set_config(config);
        runtime::reset();
        host.set_setting(
            ID,
            "enabled",
            if config.enabled { "true" } else { "false" },
        );
        host.send_chat(if config.enabled {
            "Bridging assist: ON"
        } else {
            "Bridging assist: OFF"
        })
        .ok();
    }

    /// The chevron over the crosshair, from the mod's own 32x96 RGBA image.
    fn crosshair(&self, host: &dyn ClientHost, layer: &str, draw: &mut dyn DrawContext) {
        if layer != "minecraft:crosshair" || !host.on(ID) {
            return;
        }
        let config = adapter::config(host, ID);
        if !config.show_crosshair {
            return;
        }
        let Some(target) = with_state(|s| s.target) else {
            return;
        };
        // The direction the block goes, from the side the support was found on:
        // a support above the gap draws the "down" chevron.
        let alignment = runtime::placement_alignment(target.side);
        let icon = 32.0;
        let [w, h] = draw.size();
        let x = ((w - icon + 1.0) / 2.0).floor();
        let mut y = ((h - icon + 1.0) / 2.0).floor();
        // The F3 debug screen covers the middle of the screen: the 1.0.1
        // bundle pushed the chevron down 15 GUI pixels there.
        if host.hud_mode() == mod_api::HudMode::NoHud {
            y += 15.0;
        }
        chevron(draw, [x, y], icon, alignment, runtime::INDICATOR_TINT);
    }
}

/// Draw the direction chevron: two thick bars meeting at a point, as the
/// mod's own 32x32 sprites draw it, in one colour.
fn chevron(draw: &mut dyn DrawContext, at: [f32; 2], size: f32, alignment: &str, argb: u32) {
    // The arrow points the way the block goes: "up" means the block lands
    // above the crosshair, so the bar points up.
    let (dx, dy) = match alignment {
        "up" => (0.0, -1.0),
        "down" => (0.0, 1.0),
        _ => (1.0, 0.0),
    };
    let [x, y] = at;
    let half = size / 2.0;
    let cx = x + half;
    let cy = y + half;
    // The tip, and the two ends of the arms, `thickness` wide.
    let arm = half * 0.55;
    let thick = size * 0.16;
    let tip = [cx + dx * arm, cy + dy * arm];
    // Perpendicular to the direction.
    let (px, py) = (-dy, -dx);
    for side in [-1.0_f32, 1.0] {
        let base = [cx - dx * arm * 0.15 + px * arm * side, cy - dy * arm * 0.15 + py * arm * side];
        // A bar from the tip to a base corner, drawn as a thin quad.
        let (nx, ny) = ((base[0] - tip[0]), (base[1] - tip[1]));
        let len = (nx * nx + ny * ny).sqrt().max(0.0001);
        let (ux, uy) = ((nx / len) * thick / 2.0, (ny / len) * thick / 2.0);
        draw.quad(
            [
                [tip[0] + ux, tip[1] + uy],
                [base[0] + ux, base[1] + uy],
                [base[0] - ux, base[1] - uy],
                [tip[0] - ux, tip[1] - uy],
            ],
            argb,
        );
    }
}

/// Whether the assist may act: the mod's own rules, and the crouch rule of
/// the 1.0.1 bundle (`assist_visible`).
fn playing_behaviour(config: &Config, crouching: bool) -> bool {
    config.enabled && (!config.only_bridge_when_crouched || crouching)
}

/// What a frame reads before the search runs.
fn frame_inputs(host: &dyn ClientHost) -> Option<FrameInputs> {
    let player = host.local_player()?;
    let level = player.level(host)?;
    let entity = player.entity();
    let eye = entity.eye_position(host)?;
    let look = entity.view_vector(host)?;
    let feet = entity.position(host)?;
    let on_ground = entity.on_ground(host).unwrap_or(false);
    let crossing = host.crosshair_target();
    // The sneak key's held state, as vanilla reads it for the crouch rules.
    let crouching = host
        .mapping("key.sneak")
        .ok()
        .flatten()
        .is_some_and(|m| m.down)
        || entity.is_crouching(host).unwrap_or(false);
    Some(FrameInputs {
        level,
        player,
        eye,
        look,
        reach: BLOCK_REACH,
        crouching,
        on_ground,
        feet,
        has_target: crossing.is_some(),
    })
}

#[mod_api::horizon_mod]
impl Hooks for BridgingMod {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        r.register_key_mapping(
            KeyRegistration::new(KEY, DEFAULT_KEY).label("Toggle bridging assist"),
        );
        for setting in settings() {
            let _ = r.register_setting(setting);
        }
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        self.frame(host);
    }

    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        let pressed = matches!(event.action, KeyAction::Press);
        self.key(host, event.key, pressed);
    }

    /// The right click is about to act on a crosshair that hit nothing: place
    /// in the gap instead, and cancel the click so the game does not also
    /// swing at the air.
    fn interaction_key_mapping_triggered(
        &self,
        host: &mut dyn ClientHost,
        event: &mut InteractionKeyMappingTriggeredEvent,
    ) -> Flow {
        if event.key != InteractionKey::Use || !host.on(ID) || event.cancelled {
            return Flow::Continue;
        }
        let Some((inputs, config)) = with_state(|s| s.inputs) else {
            return Flow::Continue;
        };
        let debug = host.setting_bool("debug_chat").unwrap_or(false);
        if debug {
            let line = format!(
                "[bm] use: has_target={} swing={} crouch={} enabled={} eye={:?} look={:?}",
                inputs.has_target, event.swing_hand, inputs.crouching, config.enabled, inputs.eye, inputs.look
            );
            host.send_chat(&line).ok();
        }
        if inputs.has_target || !playing_behaviour(&config, inputs.crouching) {
            return Flow::Continue;
        }
        let hand = adapter::placing_hand(host, inputs.player);
        let world = GameWorld::new(host, &inputs, hand);
        let Some(target) = runtime::closest_assist_target(
            &world,
            &config,
            inputs.eye,
            inputs.look,
            inputs.eye,
            inputs.look,
            inputs.reach,
        ) else {
            return Flow::Continue;
        };
        let hit = runtime::place_assist_target(&world, &config, target, runtime::y_frac());
        if debug {
            let line = format!(
                "[bm] target={:?} side={:?} hit pos=({},{},{}) face={:?} hand={}",
                target.pos, target.side, hit.pos.x, hit.pos.y, hit.pos.z, hit.face, hand
            );
            host.send_chat(&line).ok();
        }
        if place::place(host, inputs.player, hand, &hit) {
            if debug {
                let line = format!("[bm] sent use_item_on at ({},{},{})", hit.pos.x, hit.pos.y, hit.pos.z);
                host.send_chat(&line).ok();
            }
            with_state(|s| s.target = None);
            event.cancelled = true;
            return Flow::Cancel;
        }
        if debug {
            host.send_chat("[bm] place refused").ok();
        }
        Flow::Continue
    }

    fn gui_layer_post(&self, host: &dyn ClientHost, event: &mut GuiLayerPostEvent<'_>) {
        self.crosshair(host, event.layer, event.draw);
    }
}

/// The settings page's current values, for the tests.
pub fn default_config() -> Config {
    Config::default()
}
