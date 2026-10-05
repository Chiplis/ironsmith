//! Source-authored only; no scenarios have been compiled or executed.
use ironsmith::ability::Ability;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::card::PowerToughness;
use ironsmith::effects::{CreateTokenEffect, EffectContext, EffectExecutor, PoisonCountersEffect, PutCountersEffect};
use ironsmith::events::cause::EventCause;
use ironsmith::object::CounterType;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::types::Subtype;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
fn a() -> PlayerId { PlayerId::from_index(0) }
fn b() -> PlayerId { PlayerId::from_index(1) }
fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into()], 20) }
fn definitions(name: &str) -> [CardDefinition; 2] {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/scoped_counter_replacements.json.fixture")).unwrap();
    let card = cards.iter().find(|card| card["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", card["mana_cost"].as_str().unwrap(), card["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) { text.push_str(&format!("Power/Toughness: {power}/{toughness}\n")); }
    text.push_str(card["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text()); artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored); [direct, materialize_artifact(&restored).unwrap()]
}
fn object(game: &mut GameState, owner: PlayerId, kind: CardType, subtype: Option<Subtype>) -> ObjectId {
    let card = CardDefinitionBuilder::new(CardId::new(), "Counter recipient probe").card_types(vec![kind])
        .subtypes(subtype.into_iter().collect()).power_toughness(PowerToughness::fixed(2, 2)).build();
    game.create_object_from_definition(&card, owner, Zone::Battlefield)
}
fn placed(game: &mut GameState, source: ObjectId, actor: PlayerId, target: ObjectId, cost: bool, amount: i32) -> u32 {
    let before = game.object(target).unwrap().counters.get(&CounterType::Charge).copied().unwrap_or(0);
    let mut ctx = EffectContext::new_default(source, actor);
    ctx.cause = if cost {EventCause::from_cost(source, actor)} else {EventCause::from_effect(source, actor)};
    PutCountersEffect::new(CounterType::Charge, amount, ChooseSpec::SpecificObject(target)).execute(game, &mut ctx).unwrap();
    game.object(target).unwrap().counters.get(&CounterType::Charge).copied().unwrap_or(0) - before
}
#[test]
fn all_three_whole_cards_preserve_secondary_abilities_and_round_trip() {
    for name in ["Doc Samson, Super Psychiatrist", "Lae'zel, Vlaakith's Champion", "Loading Zone"] {
        for definition in definitions(name) {
            assert_eq!(definition.card.name, name);
            if name == "Loading Zone" { assert!(definition.alternative_casts.iter().any(|method| matches!(method, ironsmith::alternative_cast::AlternativeCastingMethod::Warp { .. }))); }
            else { assert!(definition.abilities.len() >= 2); }
        }
    }
}
#[test]
fn active_counter_addition_requires_the_actor_and_matching_controlled_recipient() {
    for name in ["Doc Samson, Super Psychiatrist", "Lae'zel, Vlaakith's Champion"] {
        for definition in definitions(name) {
            let mut game = game(); let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let actor_source = object(&mut game, b(), CardType::Artifact, None);
            for kind in [CardType::Creature, CardType::Planeswalker, CardType::Artifact, CardType::Land] {
                let yours = object(&mut game, a(), kind, None); let theirs = object(&mut game, b(), kind, None);
                let covered = name.starts_with("Doc") || matches!(kind, CardType::Creature | CardType::Planeswalker);
                for cost in [false, true] {
                    assert_eq!(placed(&mut game, actor_source, a(), yours, cost, 1), if covered {2} else {1});
                    assert_eq!(placed(&mut game, actor_source, b(), yours, cost, 1), 1);
                    assert_eq!(placed(&mut game, actor_source, a(), theirs, cost, 1), 1);
                    assert_eq!(placed(&mut game, actor_source, a(), yours, cost, 0), 0);
                }
            }
            let target = object(&mut game, a(), CardType::Creature, None);
            game.move_object_by_effect(actor_source, Zone::Exile).unwrap();
            assert_eq!(placed(&mut game, actor_source, a(), target, false, 1), 2, "captured actor remains valid after its source leaves");
            game.phase_out(host); assert_eq!(placed(&mut game, actor_source, a(), target, false, 1), 1);
            game.phase_in(host); game.move_object_by_effect(host, Zone::Exile).unwrap();
            assert_eq!(placed(&mut game, actor_source, a(), target, false, 1), 1);
        }
    }
}
#[test]
fn laezel_player_scope_includes_only_counters_you_put_on_yourself() {
    for definition in definitions("Lae'zel, Vlaakith's Champion") {
        let mut game = game(); let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        for (actor, recipient, expected) in [(a(), a(), 2), (b(), a(), 1), (a(), b(), 1), (b(), b(), 1)] {
            let before = game.player(recipient).unwrap().poison_counters;
            PoisonCountersEffect::new(1, PlayerFilter::Specific(recipient)).execute(&mut game, &mut EffectContext::new_default(host, actor)).unwrap();
            assert_eq!(game.player(recipient).unwrap().poison_counters - before, expected);
        }
    }
}
#[test]
fn loading_zone_is_a_union_and_does_not_restrict_the_placement_actor_or_cause() {
    for definition in definitions("Loading Zone") {
        let mut game = game(); let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        for (kind, subtype, covered) in [(CardType::Creature, None, true), (CardType::Artifact, Some(Subtype::Spacecraft), true), (CardType::Land, Some(Subtype::Planet), true), (CardType::Artifact, None, false), (CardType::Planeswalker, None, false)] {
            for owner in [a(), b()] { for actor in [a(), b()] { for cost in [false, true] {
                let target = object(&mut game, owner, kind, subtype);
                assert_eq!(placed(&mut game, host, actor, target, cost, 2), if covered && owner == a() {4} else {2});
            } } }
        }
    }
}
#[test]
fn additive_entry_counters_apply_once_to_each_kind_even_with_multiple_contributions() {
    for name in ["Doc Samson, Super Psychiatrist", "Lae'zel, Vlaakith's Champion"] {
        for definition in definitions(name) {
            let mut game = game(); let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let entering = CardDefinitionBuilder::new(CardId::new(), "Multiple contribution token").token().card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .with_ability(Ability::static_ability(StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, 1)))
                .with_ability(Ability::static_ability(StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, 2)))
                .with_ability(Ability::static_ability(StaticAbility::enters_with_counters(CounterType::Shield, 1))).build();
            let ids = CreateTokenEffect::you(entering, 1).execute(&mut game, &mut EffectContext::new_default(host, a())).unwrap().result_objects().unwrap().to_vec();
            let counters = &game.object(ids[0]).unwrap().counters;
            assert_eq!(counters.get(&CounterType::PlusOnePlusOne), Some(&4)); assert_eq!(counters.get(&CounterType::Shield), Some(&2));
        }
    }
}
#[test]
fn live_control_rebinds_both_counter_actor_and_recipient() {
    for definition in definitions("Doc Samson, Super Psychiatrist") {
        let mut game = game(); let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let yours = object(&mut game, a(), CardType::Artifact, None); let theirs = object(&mut game, b(), CardType::Artifact, None);
        game.set_current_controller(host, b()).unwrap();
        assert_eq!(placed(&mut game, host, a(), yours, false, 1), 1);
        assert_eq!(placed(&mut game, host, a(), theirs, false, 1), 1);
        assert_eq!(placed(&mut game, host, b(), theirs, false, 1), 2);
    }
}
