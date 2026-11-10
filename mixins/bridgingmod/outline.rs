// The debug box at the assisted position, as `OutlineRendererMixin` adds one
// in front of `submitBlockOutline`: the vanilla selection outline still
// draws; this is an extra cube.
//
// `showOutline` (default on) shows it while playing; with F3 open
// `showDebugHighlight` (default on) governs it instead, and `showDebugTrace`
// draws every voxel the search walked.
fn bridging_outline(
    game: &crate::game::Game<'_>,
    partial: f32,
    camera: [f64; 3],
    primitives: &mut horizon_client::world_primitives::WorldPrimitives,
) {
    use crate::game::bridgingmod as bm;
    let _ = (partial, camera);
    if !game.native_on(bm::ID) {
        return;
    }
    let config = bm::config(|key| game.native_setting(bm::ID, key));
    let Some(target) = bm::target() else {
        return;
    };
    if !bm::assist_visible(&config, game.bridging_crouching()) {
        return;
    }
    let shown = if game.debug {
        config.show_debug_highlight
    } else {
        config.show_outline
    };
    let pos = target.pos;
    let min = [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)];
    let max = [min[0] + 1.0, min[1] + 1.0, min[2] + 1.0];
    if shown {
        primitives.box_outline(min, max, config.outline_colour, 1.5);
    }
    if game.debug && config.show_debug_trace {
        for voxel in bm::trace() {
            let lo = [
                f64::from(voxel.x),
                f64::from(voxel.y),
                f64::from(voxel.z),
            ];
            let hi = [lo[0] + 1.0, lo[1] + 1.0, lo[2] + 1.0];
            primitives.box_outline(lo, hi, 0x16_33_33_33, 1.0);
        }
    }
}
