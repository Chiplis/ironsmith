#[test]
fn venture_host_registers_real_dungeon_cards_and_keeps_undercity_restricted() {
    let count = ironsmith_registry::register_builtin_dungeons()
        .expect("real dungeon source cards must compile and register");
    assert_eq!(count, 4);
    let regular = ironsmith::dungeon::venture_dungeon_names(None);
    assert_eq!(regular.len(), 3);
    assert!(!regular.iter().any(|name| name == "Undercity"));
    let initiative = ironsmith::dungeon::venture_dungeon_names(Some("Undercity"));
    assert_eq!(initiative, vec!["Undercity".to_owned()]);
}
