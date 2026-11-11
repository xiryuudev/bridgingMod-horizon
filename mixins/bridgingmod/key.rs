// The toggle key (comma by default, as the Fabric mod): bridging assist on or
// off, with the action-bar message upstream shows.
fn bridging_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::bridgingmod as bm;
    if !game.native_on(bm::ID) || !game.native_playing() {
        return;
    }
    // The key the player bound, or the mod's default (comma) until they
    // change it: the default acts too, which is the only way a fresh install
    // can toggle.
    let bound = game.native_mod_key(bm::ID, bm::KEY, bm::DEFAULT_KEY);
    if !bm::toggles_on(&bound, key, pressed) {
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
