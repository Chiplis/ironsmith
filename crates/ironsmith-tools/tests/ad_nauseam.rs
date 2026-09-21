use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::BooleanContext;
use ironsmith::effects::{EffectContext, RepeatProcessEffect, execute_effect};
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, PlayerId, Zone};

fn definition(name: &str) -> ironsmith::cards::CardDefinition {
    // Use the same small source asset as the browser, but compile its oracle
    // text afresh instead of trusting its baked executable artifact.
    let route = name.to_ascii_lowercase().replace(' ', "-");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../web/ui/public/cards")
        .join(format!("{route}.json"));
    let asset: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let oracle = asset["scryfall"]["oracle_text"].as_str().unwrap();
    let payload = ironsmith_tools::CardPayload {
        name: name.into(),
        parse_name: None,
        oracle_text: oracle.into(),
        raw_oracle_text: oracle.into(),
        metadata_lines: Vec::new(),
        parse_input: asset["group"]["block"].as_str().unwrap().into(),
        other_face_name: None,
        linked_face_layout: None,
    };
    ironsmith_tools::compile_definition_from_payload(&payload).unwrap()
}

struct Choices {
    answers: std::vec::IntoIter<bool>,
    life_at_prompt: Vec<i32>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, game: &GameState, _: &BooleanContext) -> bool {
        self.life_at_prompt
            .push(game.player(PlayerId::from_index(0)).unwrap().life);
        self.answers
            .next()
            .expect("unexpected duplicate repeat prompt")
    }
}

#[test]
fn optional_repeat_cards_compile_to_real_loops() {
    for name in ["Ad Nauseam", "Forbidden Ritual", "Kindle the Carnage"] {
        let def = definition(name);
        let effects = def
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects();
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<RepeatProcessEffect>().is_some()),
            "{name} must contain an executable loop: {effects:#?}"
        );
    }
}

fn resolve(answers: Vec<bool>, starting_life: i32, expected_life: &[i32]) {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], starting_life);
    let def = definition("Ad Nauseam");
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    // Library order is bottom to top: land, no-cost spell, then a 3-mana spell
    // are revealed in that order. A fourth card proves declining stops the loop.
    for name in ["Divination", "Divination", "Ancestral Vision", "Island"] {
        game.create_object_from_definition(&definition(name), alice, Zone::Library);
    }
    let mut choices = Choices {
        answers: answers.into_iter(),
        life_at_prompt: Vec::new(),
    };
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut choices);
    for effect in def
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
    {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(choices.life_at_prompt, expected_life);
    assert!(
        choices.answers.next().is_none(),
        "every scripted choice must be consumed"
    );
    assert_eq!(game.player(alice).unwrap().hand.len(), expected_life.len());
    assert_eq!(
        game.player(alice).unwrap().library.len(),
        4 - expected_life.len()
    );
    assert_eq!(
        game.player(alice).unwrap().life,
        *expected_life.last().unwrap()
    );
}

#[test]
fn ad_nauseam_repeats_through_lands_and_spells_without_mana_costs() {
    resolve(vec![true, true, false], 20, &[20, 20, 17]);
}

#[test]
fn ad_nauseam_declining_stops_after_the_mandatory_first_card() {
    resolve(vec![false], 20, &[20]);
}

#[test]
fn ad_nauseam_can_continue_at_nonpositive_life_during_resolution() {
    resolve(vec![true, true, true, false], 2, &[2, 2, -1, -4]);
}

#[test]
fn mana_value_of_a_live_card_without_mana_cost_is_zero() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let def = CardDefinitionBuilder::new(CardId::new(), "No-cost fixture")
        .card_types(vec![CardType::Land])
        .build();
    let id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let ctx = EffectContext::new_default(id, alice);
    let value = ironsmith::effect::Value::ManaValueOf(Box::new(
        ironsmith::target::ChooseSpec::SpecificObject(id),
    ));
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(),
        0
    );
}
