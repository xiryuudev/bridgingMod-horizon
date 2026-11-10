// The direction indicator over the crosshair, as BridgingMod's own sprites:
// 32x32 white chevrons drawn 1:1 at the screen's centre, pushed down 15 GUI
// pixels while the F3 debug screen covers the middle. The three sprites live
// in one 32x96 mod image (`data/indicator.rgba`, in the mod's own order).
fn bridging_crosshair(
    game: &mut crate::game::Game<'_>,
    layer: &str,
    batch: &mut horizon_client::hud::HudBatch,
) {
    use crate::game::bridgingmod as bm;
    if layer != "minecraft:crosshair" {
        return;
    }
    if !game.native_on(bm::ID) {
        return;
    }
    let config = bm::config(|key| game.native_setting(bm::ID, key));
    if !config.show_crosshair {
        return;
    }
    let Some(target) = bm::target() else {
        return;
    };
    if !bm::assist_visible(&config, game.bridging_crouching()) {
        return;
    }
    const IMAGE: &[u8; 12288] = horizon_client::native_data!("bridgingmod", "indicator.rgba");
    if !bm::image_uploaded() {
        if !game.native_mod_image(32, 96, IMAGE) {
            return;
        }
        bm::mark_image_uploaded();
    }
    // `PlacementAlignment.from(direction)`: the side is target -> support, so
    // a support above (UP) draws the "down" sprite.
    let src_y = match bm::placement_alignment(target.side) {
        "down" => 0.0,
        "up" => 32.0,
        _ => 64.0,
    };
    let icon = 32.0;
    let x = ((batch.width - icon + 1.0) / 2.0).floor();
    let mut y = ((batch.height - icon + 1.0) / 2.0).floor();
    if game.debug {
        y += 15.0;
    }
    game.native_sprite(
        batch,
        horizon_client::hud::MOD_IMAGE_SPRITE,
        [0.0, src_y, icon, icon],
        [x, y, icon, icon],
    );
}
