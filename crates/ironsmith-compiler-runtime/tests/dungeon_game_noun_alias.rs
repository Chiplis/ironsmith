//! cf8 p05: "the dungeon" names the game object (CR 309, 701.49), never a
//! short alias of a card whose name starts with "Dungeon".
//! Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

#[test]
fn venture_into_the_dungeon_is_not_a_self_reference() {
    let rows = support::rows("dungeon_game_noun_alias");
    assert_eq!(rows.len(), 2);
    for row in &rows {
        for definition in support::definitions(row) {
            let debug = support::debug(&definition);
            assert!(debug.contains("Venture"), "{}: venture missing", row["name"]);
            let rendered = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
            assert!(rendered.to_ascii_lowercase().contains("venture into the dungeon"), "{rendered}");
        }
    }
}
