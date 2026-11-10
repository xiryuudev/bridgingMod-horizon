// The mod's settings page: its features, as the Fabric mod's config screen
// groups them.
fn bridging_settings(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::bridgingmod as bm;
    if mod_id != bm::ID {
        return;
    }
    let config = { let game = &*game; bm::config(|key| game.native_setting(bm::ID, key)) };
    buttons.extend(config.labels());
}
