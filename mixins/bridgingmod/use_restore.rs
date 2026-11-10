// The use is over (its own end and every early return): put the tick's
// crosshair back where vanilla left it, and, when the block actually went
// down, wait the configured delay before the next use (`delayPostBridging`).
fn bridging_use_restore(game: &mut crate::game::Game<'_>, value: ()) {
    let _ = value;
    use crate::game::bridgingmod as bm;
    let Some(pos) = bm::end_use() else {
        return;
    };
    // The tick's crosshair was a miss; give it back.
    game.tick_hit = Some(None);
    if !game.bridging_air(pos) {
        let delay = bm::current_config().map_or(4, |c| c.delay_post_bridging.max(0)) as u32;
        game.use_delay = delay;
    }
}
