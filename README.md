# BridgingMod — Horizon Native Mod

Reach-around placement ("bridge assist") for [Horizon](https://github.com/horizon-rust/horizon), the Minecraft Java
Edition rewrite in Rust: a port of [squeeglii's BridgingMod](https://github.com/squeeglii/BridgingMod) (Fabric/NeoForge)
to a Horizon native mixin bundle, written in Rust.

When the crosshair hits nothing and you hold a block, the mod looks through the gap along your view line: if a block
you would place there can be built off a neighbour — a wall in front of you, a floor under you, a ceiling above you —
a small chevron appears at the crosshair and the right click places *there* instead. Under F3 an outline shows the
exact position, with the voxels the search walked if you turn the trace on.

## Features

- **Reach-around placement**: bridge forwards, up (under a ceiling) and down (into a gap below you) without looking
  at the surface you build off.
- **Direction indicator**: one of the original 32×32 chevrons at the crosshair — `up`, `down` or `horizontal` — so
  you know which way the block goes. Pushed down 15 pixels while F3 is open, like the original.
- **F3 outline**: the assisted block position is boxed (always-on outline outside F3 too, as the original defaults).
- **Slab & trapdoor assist**: places the correct half of a slab/trapdoor, and combines two slabs vertically.
- **Crouch rules**: bridging only while crouching, with a separate allowed-axes set while crouched.
- **The original's settings**, in Options › Mods › BridgingMod: axes, minimum distance, delay after bridging,
  replacing plants/snow, skipping torches, corner search, F3 box/trace, and the toggle key (**comma**).

## Install

1. Open Horizon Launcher → **Mods**.
2. **Add a native mod from GitHub** and enter:
   `https://github.com/xiryuudev/bridgingMod-horizon`
3. Trust the native code when asked (native mods are not sandboxed — read the source first).
4. Build the native build (Mods page → **Native build of … → Build**).
5. In game: **Options › Mods › BridgingMod** for the settings; the toggle key is **comma** by default.

Native builds compile the game from source with the launcher's own Rust toolchain: on Linux this needs
`build-essential pkg-config libasound2-dev libudev-dev` (the launcher names anything missing), and the first build can
take 20–30 minutes.

## How the port works

Horizon native mods are *mixin bundles*: Rust helpers injected into named functions of the game's own source before it
is compiled. This bundle pins Horizon commit `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca` (the "26.3" source of
November 8, 2026) and uses both Horizon's native hook points and two functions that are not hook points:

| File | Injected into | Role |
|---|---|---|
| `runtime.rs` | a module in `crates/horizon-client/src/game.rs` | the whole search, hit construction and settings; no game hooks |
| `frame.rs` | `native_frame` (HEAD) | per frame: keep the target for the indicator and the click |
| `use.rs` | `remote_use` (HEAD) | on a use with a missing crosshair: point the tick's crosshair at the target |
| `use_restore.rs` | `remote_use` (RETURN) | put the crosshair back and apply the delay |
| `crosshair.rs` | `native_gui_layer_post` (`minecraft:crosshair`) | draw the chevron from the mod's own 32×96 image |
| `outline.rs` | `native_world_primitives` (HEAD) | the F3 box (and trace) |
| `key.rs`, `list.rs`, `settings.rs`, `settings_click.rs` | the settings/mod hooks | toggle key, page and its buttons |
| `data/indicator.rgba` | — | the three original 32×32 chevrons, `up`/`down`/`horizontal`, stacked |

The search itself is a faithful port of the Fabric mod:

1. The view line runs from the camera to the end of the player's reach (`blockInteractionRange`, 4.5 blocks), and the
   first 20 % (`minBridgeDistance`) is skipped. A 3-D dominant-axis Bresenham walk yields the voxels on it, with
   `bridgingAdjacency`'s "missed points" inserting the diagonal neighbours its line touches.
2. The first voxel that is air (or `canBeReplaced` with *replace plants and snow* on) and that does not intersect the
   player's box is the candidate.
3. Its valid sides are the six axes whose normal is within ~84° of the view direction, each flipped to point from the
   target back at the support; the first one whose support block is solid, not a fluid and not replaceable, and whose
   axis the settings allow, wins.
4. The synthetic hit is the original's: click the target block at its centre with the *support's inner face*
   (`side.opposite()`), so the game places at the target with the face's orientation. Slab/trapdoor overrides use the
   original's 0.1 / 0.9 Y points and the vertical slab combine on the support block.
5. While the click is being handled, Horizon's tick crosshair (`tick_hit`) is pointed at that hit, so the game's own
   use-on-block path runs (prediction, sound, the server packet). Restored on every return of `remote_use`.

## Settings

Options › Mods › BridgingMod (stored in `horizon-mods/.horizon-mod-data/bridgingmod/`):

| Button | Default | Meaning |
|---|---|---|
| Bridging assist | ON | master switch (also the comma key) |
| Only while crouching | OFF | the original's `onlyBridgeWhenCrouched` |
| Bridge axes | Both | horizontal / vertical / both |
| Axes while crouching | As above | a separate set while crouched |
| Minimum distance | 20 % | part of the view line skipped at the camera |
| Delay after bridging | 4 ticks | vanilla's right-click delay, or shorter |
| Crosshair indicator | ON | the chevron |
| Debug box | ON | the outline (the "F3 box" also has its own toggle) |
| Slab assist | ON | slab/trapdoor half + double-slab combine |
| Replace plants and snow | ON | targets any `canBeReplaced` block, not only air |
| Skip torches | ON | torches never bridge |
| Missed corners | Corners | the supercover search on/off |
| F3 debug box | ON | outline while F3 is open |
| F3 trace | OFF | every voxel the search walked |

## Building it yourself

The launcher does everything; for a quick type-check without touching the launcher's installed versions, stage the
bundle and check the game crate:

```sh
# horizon-mixins from the launcher's tools folder, on the pinned Horizon source:
horizon-mixins apply --workspace <horizon-source> --mods <bundle-dir> --out <stage>
cargo check --manifest-path <stage>/Cargo.toml --locked --profile fast \
  -p horizon-client --bin horizon-client
```

The real build is the launcher's: `horizon-launcher native-build <bundle-dir> --instance <name> --trust-native-code`
(`--profile dev` makes rebuilds ~6× faster while developing).

## Credits and licence

- Original mod: [BridgingMod](https://github.com/squeeglii/BridgingMod) by **squeeglii** (Will Scully), MIT. The search,
  the hit formula, the settings and the indicator sprites are theirs; this port reproduces them in Rust.
- BridgingMod before 2.0 came from Quark's "Reach-Around Placement" ([VazkiiMods/Quark](https://github.com/VazkiiMods/Quark)).
- Horizon native mod format: [horizon-mixins](https://github.com/horizon-rust/horizon-mixins) by levzzz5154.
- This port: MIT, © 2026 xiryuudev. See [LICENSE](LICENSE).
