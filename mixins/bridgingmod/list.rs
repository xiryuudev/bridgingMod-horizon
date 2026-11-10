// BridgingMod's row in Options > Mods, with its toggle key (comma, as the
// Fabric mod's default).
fn bridging_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::bridgingmod as bm;
    mods.push(crate::game::native_hooks::NativeMod {
        id: bm::ID.into(),
        title: format!("{} {}", bm::NAME, bm::VERSION),
        description: bm::DESCRIPTION.into(),
        keys: vec![[
            bm::KEY.into(),
            "Toggle bridging assist".into(),
            bm::DEFAULT_KEY.into(),
        ]],
    });
}
