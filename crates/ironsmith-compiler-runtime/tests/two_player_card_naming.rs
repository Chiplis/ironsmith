//! Null Chamber: you and an opponent each name a nonbasic card name; spells
//! and lands with either name can't be cast or played. Source-authored, UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const NULL_CHAMBER: &str = "Mana cost: {3}{W}\nType: World Enchantment\nAs this enchantment enters, you and an opponent each choose a card name other than a basic land card name.\nSpells with the chosen names can't be cast and lands with the chosen names can't be played.";

#[test]
fn null_chamber_records_two_names_and_prohibits_both() {
    for definition in support::definitions("Null Chamber", NULL_CHAMBER) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("opponent_also_chooses: true"), "{debug}");
        assert!(debug.contains("exclude_basic_land_names: true"), "{debug}");
        assert!(debug.contains("CastSpellsMatching"), "{debug}");
        assert!(debug.contains("PlayLandsMatching"), "{debug}");
        assert!(debug.contains("{chosen name}"), "{debug}");
    }
}

#[test]
fn multiplayer_controller_selects_the_opponent_who_names_the_second_card() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use ironsmith::decisions::context::{SelectOptionsContext, TextInputContext};
    #[derive(Default)]
    struct Names { prompts: Vec<PlayerId> }
    impl DecisionMaker for Names {
        fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
            if context.description == "Choose an opponent to name a card" {
                assert_eq!(context.player, PlayerId::from_index(0));
                vec![1]
            } else { SelectFirstDecisionMaker.decide_options(game, context) }
        }
        fn decide_text(&mut self, _: &GameState, context: &TextInputContext) -> String {
            self.prompts.push(context.player);
            if context.player == PlayerId::from_index(0) { "Lightning Bolt".into() }
            else { "Counterspell".into() }
        }
    }
    for definition in support::definitions("Null Chamber", NULL_CHAMBER) {
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        let a = PlayerId::from_index(0);
        let card = game.create_object_from_definition(&definition, a, Zone::Hand);
        let mut names = Names::default();
        let receipt = game.move_object_with_etb_processing_with_dm(card, Zone::Battlefield, &mut names).unwrap();
        assert!(!receipt.pending);
        let permanent = receipt.original.into_result().unwrap().new_id;
        assert_eq!(names.prompts, vec![a, PlayerId::from_index(2)]);
        assert_eq!(game.chosen_named_option(permanent), Some("Lightning Bolt\nCounterspell"));
    }
}
