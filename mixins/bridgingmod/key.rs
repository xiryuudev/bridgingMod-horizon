// The toggle key (comma by default, as the Fabric mod): bridging assist on or
// off, with the action-bar message upstream shows.
fn bridging_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::bridgingmod as bm;
    if !pressed || !game.native_on(bm::ID) || !game.native_playing() {
        return;
    }
    let bound = game.native_mod_key(bm::ID, bm::KEY, bm::DEFAULT_KEY);
    if bound == bm::DEFAULT_KEY || key != bound {
        return;
    }
    let mut config = bm::config(|name| game.native_setting(bm::ID, name));
    config.enabled = !config.enabled;
    bm::set_config(config);
    bm::reset();
    for (name, value) in config.stored() {
        game.native_set_setting(bm::ID, name, &value);
    }
    game.native_chat(if config.enabled { bm::ON } else { bm::OFF });
}
