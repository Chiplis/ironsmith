use super::*;
use crate::cards::CardDefinition;
use crate::effects::{EffectContext, execute_effect};
use crate::game_state::GameState;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::object::CounterType;
use crate::types::{Subtype, Supertype};

struct Pay(bool);
impl crate::decision::DecisionMaker for Pay {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.0
    }
}

fn parse(text: &str) -> CardDefinition {
    crate::CardDefinitionBuilder::new(CardId::new(), "Upkeep Maker")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .unwrap()
}

fn upkeep(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    assert!(
        definition.spell_effect.is_none(),
        "an upkeep cost must not become a spell effect"
    );
    assert_eq!(definition.abilities.len(), 1);
    let AbilityKind::Triggered(trigger) = &definition.abilities[0].kind else {
        panic!("expected an upkeep trigger");
    };
    assert!(
        trigger
            .trigger
            .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()
            .is_some_and(|upkeep| upkeep.player == crate::target::PlayerFilter::You)
    );
    assert!(
        trigger.choices.is_empty(),
        "payment instructions must not announce targets"
    );
    trigger
}

fn resolve_upkeep(
    game: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    definition: &CardDefinition,
    pay: bool,
) {
    let mut dm = Pay(pay);
    let mut ctx = EffectContext::new_default(source, player).with_decision_maker(&mut dm);
    for effect in upkeep(definition).effects.all_effects() {
        execute_effect(game, effect, &mut ctx).unwrap();
    }
}

#[test]
fn cumulative_upkeep_creates_snow_elephants_once_per_age_counter() {
    for separator in ["-", "—"] {
        let definition = parse(&format!(
            "Cumulative upkeep {separator} Create a 3/3 green snow Elephant creature token with trample"
        ));
        let rendered = crate::compiled_text_lines(&definition).join("\n");
        assert!(
            rendered.to_ascii_lowercase().contains("cumulative upkeep"),
            "{rendered}"
        );
        assert!(
            rendered.contains("3/3 green snow Elephant creature token with trample"),
            "{rendered}"
        );
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for (age, expected_tokens) in [(1, 1), (2, 3), (3, 6)] {
            resolve_upkeep(&mut game, source, alice, &definition, true);
            assert_eq!(
                game.object(source)
                    .unwrap()
                    .counters
                    .get(&CounterType::Age)
                    .copied(),
                Some(age)
            );
            let tokens: Vec<_> = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| *id != source)
                .collect();
            assert_eq!(tokens.len(), expected_tokens);
            for token in tokens {
                assert!(game.current_has_supertype(token, Supertype::Snow));
                assert!(game.current_has_subtype(token, Subtype::Elephant));
                assert_eq!(game.current_power(token), Some(3));
                assert_eq!(game.current_toughness(token), Some(3));
                assert_eq!(
                    game.current_colors(token),
                    Some(crate::color::ColorSet::GREEN)
                );
                assert!(game.current_has_static_ability_id(
                    token,
                    crate::static_abilities::StaticAbilityId::Trample
                ));
            }
        }
        resolve_upkeep(&mut game, source, alice, &definition, false);
        assert!(!game.battlefield.contains(&source));
        assert_eq!(
            game.battlefield.len(),
            6,
            "declining must not create tokens"
        );
    }
}

#[test]
fn cumulative_upkeep_repeats_composed_non_targeting_effects() {
    let definition = parse("Cumulative upkeep—Create a Treasure token and gain 2 life.");
    let rendered = crate::compiled_text_lines(&definition)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("treasure token") && rendered.contains("gain 2 life"),
        "{rendered}"
    );
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    resolve_upkeep(&mut game, source, alice, &definition, true);
    resolve_upkeep(&mut game, source, alice, &definition, true);
    assert_eq!(game.player(alice).unwrap().life, 26);
    assert_eq!(game.battlefield.len(), 4, "one source and three Treasures");
}

#[test]
fn cumulative_upkeep_rejects_targeted_payment_instructions() {
    let result = crate::CardDefinitionBuilder::new(CardId::new(), "Targeted Upkeep")
        .card_types(vec![CardType::Creature])
        .parse_text("Cumulative upkeep—Destroy target creature.");
    assert!(
        result.is_err(),
        "upkeep payments cannot require announced targets"
    );
}

#[test]
fn cumulative_upkeep_impossible_composed_payment_leaves_no_created_tokens() {
    let definition = parse("Cumulative upkeep—Create a Treasure token and pay 30 life.");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    resolve_upkeep(&mut game, source, alice, &definition, true);
    assert!(!game.battlefield.contains(&source));
    assert!(
        game.battlefield.is_empty(),
        "prospective payment must not leak tokens"
    );
    assert_eq!(game.player(alice).unwrap().life, 20);
}

#[test]
fn cumulative_upkeep_payment_does_not_target_through_a_created_tokens_ability() {
    let definition = parse(
        "Cumulative upkeep—Create a 1/1 blue Wizard creature token with \"{T}: Deal 1 damage to target creature.\"",
    );
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    resolve_upkeep(&mut game, source, alice, &definition, true);
    let token = *game.battlefield.iter().find(|id| **id != source).unwrap();
    assert!(
        game.current_abilities(token)
            .unwrap()
            .iter()
            .any(|ability| {
                matches!(&ability.kind, AbilityKind::Activated(activated)
            if !activated.choices.is_empty())
            }),
        "the created token must retain its targeted activated ability"
    );
}

#[test]
fn cumulative_upkeep_invalid_instructions_do_not_fall_back_to_spell_effects() {
    assert!(
        crate::CardDefinitionBuilder::new(CardId::new(), "Invalid Upkeep")
            .card_types(vec![CardType::Creature])
            .parse_text("Cumulative upkeep—Do a backflip.")
            .is_err()
    );
}

#[test]
fn cumulative_upkeep_supports_alternative_effect_payments() {
    let definition = parse("Cumulative upkeep—Create a Treasure token or gain 2 life.");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    resolve_upkeep(&mut game, source, alice, &definition, true);
    assert!(game.battlefield.contains(&source));
    assert_eq!(game.battlefield.len(), 2);
    assert_eq!(game.player(alice).unwrap().life, 20);
}
