// A use starts (`Minecraft.startUseItem`): when the crosshair hits nothing
// and a bridge target exists, point the tick's crosshair at it for this use.
// The game's own use-on-block path then places there (`use_restore.rs` puts
// the crosshair back afterwards).
//
// Faithful to `MinecraftClientMixin.onItemUse`: the assist only happens when
// every other way to interact is exhausted, and the crouch rule applies.
fn bridging_use(game: &mut crate::game::Game<'_>) {
    use crate::game::bridgingmod as bm;
    if !game.native_on(bm::ID) {
        return;
    }
    let config = bm::config(|key| game.native_setting(bm::ID, key));
    if !bm::assist_visible(&config, game.bridging_crouching()) {
        return;
    }
    // Vanilla has something to place on, or something to interact with
    // (an entity, a fence knot): no reach-around.
    if game.pick().is_some() || game.remote_use_targets_entity() || !game.may_build() {
        return;
    }
    let Some((camera_pos, camera_look)) = game.bridging_aim() else {
        return;
    };
    let Some(eye) = game.bridging_eye() else {
        return;
    };
    let reach = game.bridging_reach();
    // The hand whose item would place: the main hand's, or the off hand's
    // when the main hand holds no block (vanilla tries them in that order).
    let hand = usize::from(game.bridging_held_blocks()[0].is_none());
    let target = {
        let world = bm::GameWorld::new(&*game, hand);
        bm::closest_assist_target(
            &world,
            &config,
            camera_pos,
            camera_look,
            eye,
            camera_look,
            reach,
        )
    };
    let Some(target) = target else {
        return;
    };
    let hit = {
        let world = bm::GameWorld::new(&*game, hand);
        bm::place_assist_target(&world, &config, target, bm::y_frac())
    };
    // The virtual click: the replaceable target block, its support's inner
    // face, and the target's centre as the point.
    let target_pos = bm::Pos::new(hit.pos.x, hit.pos.y, hit.pos.z);
    let hit = crate::game::Hit {
        pos: [hit.pos.x, hit.pos.y, hit.pos.z],
        face: hit.face.unit(),
        point: hit.point,
    };
    // Only bridge when the game's own placement rules accept the synthetic
    // click (reach, survival, can_place_on, the item's own rules): a refused
    // one would rubber-band or be rolled back by the server.
    let stack = if hand == 0 {
        game.held().cloned()
    } else {
        game.offhand.clone()
    };
    let Some(stack) = stack else {
        return;
    };
    if game.remote_can_place(&hit, &stack).is_none() {
        return;
    }
    game.tick_hit = Some(Some(hit));
    bm::begin_use(target_pos);
}
