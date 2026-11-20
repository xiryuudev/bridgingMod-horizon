# Registry submission (pull request to horizon-mods)

The registry now takes **`.hmod`** packages only; the old `.hnmod` mixin bundles are archived and no launcher
loads them. This is the SDK port.

## File

- Kind: **native-mod**
- File: `bridgingmod.hmod` (20,848 bytes)
- SHA-256: `a80efa912e322301813e388518fa909019a79dd8671e55b38dc5e4f42952d29e`
- id: `bridgingmod_horizon`
- Version: `2.0.0`
- Minecraft: `26.3`
- Pins: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`, horizon-mixins
  `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`
- Download: attach `bridgingmod.hmod` to a GitHub release, then link it in the pull request

## Signature

- Public key (minisign):

```
RWTmX3D9C28IymzslPHR9Nv6c6lEYdx7eBxU44E+SyfgdJ0XglLHkppK
```

- Signature: `bridgingmod.hmod.minisig`
- Verified with the registry checker:

```sh
cargo horizon-mod check bridgingmod.hmod \
  --key "RWTmX3D9C28IymzslPHR9Nv6c6lEYdx7eBxU44E+SyfgdJ0XglLHkppK" \
  --signature bridgingmod.hmod.minisig
```

## Page

- **Name:** BridgingMod
- **Summary:** Reach-around block placement: build a bridge off the block beside the gap you are looking through.
- **Authors:** xiryuudev (Horizon port); squeeglii (original BridgingMod)
- **Licence:** MIT
- **Source:** https://github.com/xiryuudev/bridgingMod-horizon
- **Categories:** `utility`
- **Tags:** `building`, `placement`, `bridge`, `port`

## The id is taken

The id `bridgingmod_horizon` is held in the live index by another port (owner `AdLf0208`, v0.2.2), and the old submission
(#627) was closed when the format was retired. A reactivation needs the archived owner's numeric id. So this
submits under a **new id**: `bridgingmod_horizon` (settle the exact spelling with the reviewers), and the entry
points at this repository.
