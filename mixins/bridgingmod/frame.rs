// Once a frame, after the frame's ticks: what the crosshair would bridge to.
//
// Mirrors `BridgingStateTracker.tick`: only when the crosshair hits nothing
// (an entity counts as a hit) and a hand holds a placeable item. The answer
// is kept for the click (`use.rs`) and the indicator (`crosshair.rs`).
fn bridging_frame(game: &mut crate::game::Game<'_>) {
    use crate::game::bridgingmod as bm;
    let config = bm::config(|key| game.native_setting(bm::ID, key));
    if !game.native_on(bm::ID) || !config.enabled {
        bm::reset();
        return;
    }
    let playing = game.screen == crate::game::Screen::Playing
        && game.sleeping.is_none()
        && game.player.is_some();
    if !playing {
        bm::reset();
        return;
    }
    // The slab assist's Y fraction: the feet's fraction while on the ground
    // (`Mth.frac(player.getY())`, as `Minecraft.tick` keeps it).
    if game.bridging_on_ground() {
        if let Some(pos) = game.bridging_player_pos() {
            bm::set_y_frac(bm::fraction(pos[1]));
        }
    }
    // A valid block hit, or an entity in front, means vanilla can place:
    // no reach-around.
    if game.pick().is_some() || game.remote_use_targets_entity() {
        bm::reset();
        return;
    }
    let Some((camera_pos, camera_look)) = game.bridging_aim() else {
        bm::reset();
        return;
    };
    let Some(eye) = game.bridging_eye() else {
        bm::reset();
        return;
    };
    let reach = game.bridging_reach();
    let world = bm::GameWorld::new(&*game, 0);
    bm::refresh(
        &world,
        &config,
        camera_pos,
        camera_look,
        eye,
        camera_look,
        reach,
    );
}
