// A click on the settings page: change the setting and keep it.
fn bridging_settings_click(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    button: usize,
) {
    use crate::game::bridgingmod as bm;
    if mod_id != bm::ID {
        return;
    }
    let mut config = bm::config(|key| game.native_setting(bm::ID, key));
    config.click(button);
    bm::set_config(config);
    for (key, value) in config.stored() {
        game.native_set_setting(bm::ID, key, &value);
    }
}
