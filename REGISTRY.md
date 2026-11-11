# Registry submission (playhorizon.dev/mods/submit)

Everything the submit form asks for, ready to paste.

## File

- Kind: **native mod**
- File: `bridgingmod-1.0.1.hnmod` (19,966 bytes)
- SHA-256: `3a6b7e5aadcd897fed978f395997b9b83c39900bc7da81f01580ef4b7cdbbece`
- id: `bridgingmod`
- Version: `1.0.1`
- Minecraft: `26.3`
- Changelog: see below
- Download: https://github.com/xiryuudev/bridgingMod-horizon/releases/download/v1.0.1/bridgingmod-1.0.1.hnmod

## Page

- **Name:** BridgingMod
- **Summary:** Reach-around block placement: build a bridge off the block beside the gap you are looking through.
- **Authors:** xiryuudev (Horizon port); squeeglii (original BridgingMod)
- **Licence:** MIT
- **Source:** https://github.com/xiryuudev/bridgingMod-horizon
- **Categories (pick 2):** `utility`, `game-mechanics`
- **Tags:** `building`, `placement`, `reach-around`, `bridge`, `client`
- **Icon:** optional, square PNG <= 256 KiB (not made yet)
- **Screenshots:** optional, up to 8 https URLs (not made yet)

## Signature

- Public key (minisign):

```
RWTmX3D9C28IymzslPHR9Nv6c6lEYdx7eBxU44E+SyfgdJ0XglLHkppK
```

- Signature file: `bridgingmod-1.0.1.hnmod.minisig`
  https://github.com/xiryuudev/bridgingMod-horizon/releases/download/v1.0.1/bridgingmod-1.0.1.hnmod.minisig

Verified with:

```sh
cargo horizon-mod check bridgingmod-1.0.1.hnmod \
  --key "RWTmX3D9C28IymzslPHR9Nv6c6lEYdx7eBxU44E+SyfgdJ0XglLHkppK" \
  --signature bridgingmod-1.0.1.hnmod.minisig
```

-> `signature verified`

## Description (Markdown, for the project page)

BridgingMod adds **reach-around placement** ("bridge assist") to Horizon: a port of
[BridgingMod](https://github.com/squeeglii/BridgingMod) 2.7.0 by squeeglii, rewritten in Rust as a Horizon native mod.

When the crosshair hits nothing and you hold a block, the mod walks the view line out to the end of your reach, finds
the first gap cell that has a solid neighbour to build off, and places there instead. A small chevron at the crosshair
shows which way the block goes.

### What it does

- Reach-around placement forwards, up (under a ceiling) and down (into a gap below you).
- A 32x32 direction chevron at the crosshair: `up`, `down` or `horizontal`, pushed down 15 px while F3 is open.
- An outline of the assisted cell (always on outside F3 if enabled, and under F3 with its own toggle).
- Optional F3 trace: every voxel the search walked.
- Slab and trapdoor assist: the correct half, and combining two slabs vertically.
- Crouch rules: bridge only while crouching, with a separate set of allowed axes while crouched.
- Settings in Options > Mods > BridgingMod: axes, minimum distance, delay after bridging, replacing plants/snow,
  skipping torches, corner search, F3 box and trace. Toggle key: comma.

### What it does not do

- It does not add blocks, items or recipes: it is a client-side placement helper only.
- It does not bypass the game's placement rules. Every assisted click is validated by the client's own
  `remote_can_place` (reach, survival rules, `can_place_on`, item rules) before it is sent; a refused click falls back to
  vanilla behaviour, so nothing is rubber-banded or rolled back.
- It does not change world state on its own and does not talk to any server beyond the ordinary use-on-block packet.
- It does not include the original mod's optional compat hooks (Dank Storage, Sable, Dynamic Crosshair, free-look
  perspective detection).
- It does not work while an entity is in front of the crosshair, or while a screen is open.

### How it works (for review)

A Horizon native mod is a mixin bundle: Rust source injected into the game's own source before it compiles. This
bundle adds one module and nine injections:

| Injection | Target | Role |
|---|---|---|
| module `runtime` | `crates/horizon-client/src/game.rs` | the search, the synthetic hit and the settings; no hooks |
| `frame` | `native_frame` (hook) | once a frame: compute and keep the assist target |
| `use` | `horizon_client::game::remote::Game::remote_use` (HEAD, not a hook point) | on a use with a missing crosshair, point the tick's crosshair at the assist hit |
| `use-restore` | `remote_use` (RETURN) | put the crosshair back on every return, and apply the configured delay |
| `crosshair` | `native_gui_layer_post` (hook, `minecraft:crosshair`) | draw the chevron from the mod's own 32x96 RGBA image |
| `outline` | `native_world_primitives` (not a hook point) | the F3 box and the optional trace |
| `key`, `list`, `settings`, `settings-click` | the settings/mod hooks | toggle key, Options > Mods row, settings page |

The search is a faithful port of the Fabric original: a 3-D dominant-axis Bresenham walk of the view line (with the
original's "missed points" supercover), a greedy first valid gap cell, then the first view-aligned side whose support
block is solid, not a fluid and not replaceable. The synthetic hit uses the original's exact form: the gap cell as the
clicked position, the support's inner face as the face, and the cell centre as the point, so orientation-sensitive
blocks (stairs, slabs, trapdoors, logs, pistons) turn out right.

### Compatibility

Built and tested on Horizon commit `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca` (Minecraft 26.3, 8 November 2026) with
horizon-mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`. It pins that commit because two of its injections target
functions that are not named hook points (`remote_use`, `native_world_primitives`); a Horizon change there needs a
rebuild of the mod. When that happens the launcher reports the mod as not building and leaves it out.

### Credits

- Original mod: [BridgingMod](https://github.com/squeeglii/BridgingMod) by squeeglii (Will Scully), MIT. The algorithm,
  the hit formula, the settings and the chevron sprites are theirs.
- Before 2.0, BridgingMod came from Quark's "Reach-Around Placement".
- Horizon native mod format: [horizon-mixins](https://github.com/horizon-rust/horizon-mixins).

## Changelog

```
1.0.1 - the advertised comma toggle now works

Fixes the default-key defect reported in review: `bridging_key` returned early when the
resolved key equalled the mod's own default, and `native_mod_key` answers that default
whenever the player set no override, so on a fresh install (or an explicit comma binding)
the toggle never ran.

- `runtime.rs`: new `toggles_on(bound, key, pressed)` decides whether the toggle key acts;
  it accepts the default, rejects only an unbound or empty key, and is unit-tested.
- `key.rs`: uses `toggles_on`, so the default comma key toggles like a rebound key.
- Tests: default key toggles, a rebound key toggles and the stale default does not, an
  unbound key never toggles. Verified in game: a comma press writes
  `horizon-mods/.horizon-mod-data/bridgingmod/enabled` false -> true with the
  "Bridging assist: OFF/ON" action-bar message on a fresh profile.

1.0.0 - first release

Port of BridgingMod 2.7.0 (Minecraft 26.3) to a Horizon native mod, pinned to Horizon fdc87ae.

- Reach-around placement forwards, up and down, with the original's search, hit construction and
  missed-point supercover.
- Crosshair chevron (up/down/horizontal), F3 outline and optional F3 trace.
- Slab and trapdoor assist, including the vertical double-slab combine.
- Crouch-only mode and crouch axis override, minimum distance, post-bridge delay, replace
  plants/snow, skip torches, corner search.
- Settings page in Options > Mods; toggle key: comma.
- Verified in game: a right click through a ledge gap places at the assisted cell, and the
  chevron appears only when an assist target exists.
```
