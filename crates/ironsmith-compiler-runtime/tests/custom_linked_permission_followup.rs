#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::{GameState, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn defs(name: &str, text: &str) -> [ironsmith::CardDefinition; 2] {
    source_support::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap()
}
#[test]
fn enchanted_player_free_price_uses_its_beneficiary_devotion_and_source_lifetime() {
    let text = "Type: Enchantment — Aura\nEnchant player\nEnchanted player may cast blue spells with mana value less than or equal to their devotion to blue without paying their mana costs.";
    for aura in defs("Source-relative casting permission", text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = B;
        game.turn.priority_player = Some(B);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        let owner_devotion =
            defs("Owner devotion", "Mana cost: {U}{U}{U}\nType: Artifact")[0].clone();
        game.create_object_from_definition(&owner_devotion, A, Zone::Battlefield);
        let devotion = defs("Recipient devotion", "Mana cost: {U}\nType: Artifact")[0].clone();
        game.create_object_from_definition(&devotion, B, Zone::Battlefield);
        let id = game.create_object_from_definition(&aura, A, Zone::Battlefield);
        assert!(game.attach_object_to_target(id, ironsmith::object::AttachmentTarget::Player(B)));
        let spell = defs(
            "Blue price witness",
            "Mana cost: {1}{U}\nType: Instant\nDraw a card.",
        )[0]
        .clone();
        let card = game.create_object_from_definition(&spell, B, Zone::Hand);
        let legal = |game: &GameState| {
            ironsmith::decision::compute_legal_actions(game,B).unwrap().into_iter().any(|a|matches!(a,ironsmith::decision::LegalAction::CastSpell{spell_id,..}if spell_id==card))
        };
        assert!(
            !legal(&game),
            "grantor's three devotion cannot substitute for recipient's one"
        );
        game.create_object_from_definition(&devotion, B, Zone::Battlefield);
        assert!(
            legal(&game),
            "recipient's two devotion enables its own blue spell"
        );
        game.move_object_by_effect(id, Zone::Exile).unwrap();
        assert!(!legal(&game), "source departure removes free price");
    }
}
#[test]
fn bounded_player_program_owns_any_number_and_linked_copy_condition() {
    let text = "Type: Sorcery\nFor each of any number of target players, do the following: [If this permanent's back face is a Blessing, create a token that's a copy of its back face attached to that player].";
    let input = serde_json::json!({"name":"Bounded linked player program","executable_text":text,"custom_subtypes":[{"singular":"Blessing","plural":"Blessings","family":"Enchantment"}]});
    for definition in source_support::definitions(&input).unwrap() {
        let effects = definition
            .spell_effect
            .as_ref()
            .unwrap()
            .all_effects_owned();
        assert!(
            effects.iter().any(|effect| effect
                .target_selection_profile()
                .is_some_and(|profile| profile.min_targets == 0 && profile.max_targets.is_none())),
            "announced player list retains any-number cardinality"
        );
    }
}
