//! CR 702.82c "Devour [quality] N" and Thromok's "Devour X, where X is the
//! number of creatures devoured this way". Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::effects::DevourEffect;
use ironsmith::object::CounterType;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};

#[path = "p02_line_families/compile.rs"]
mod compile;

const A: PlayerId = PlayerId::from_index(0);

const CAPRICHROME: &str = "Mana cost: {3}{W}\nType: Artifact Creature — Goat\nPower/Toughness: 2/2\nFlash\nVigilance\nDevour artifact 1 (As this creature enters, you may sacrifice any number of artifacts. It enters with that many +1/+1 counters on it.)";
const FEASTING_HOBBIT: &str = "Mana cost: {1}{G}\nType: Creature — Halfling Citizen\nPower/Toughness: 2/2\nDevour Food 3 (As this creature enters, you may sacrifice any number of Foods. It enters with three times that many +1/+1 counters on it.)\nCreatures with power less than this creature's power can't block it.";
const FAMISHED_WORLDSIRE: &str = "Mana cost: {5}{G}{G}{G}\nType: Creature — Leviathan\nPower/Toughness: 0/0\nWard {3}\nDevour land 3 (As this creature enters, you may sacrifice any number of lands. It enters with three times that many +1/+1 counters on it.)\nWhen this creature enters, look at the top X cards of your library, where X is this creature's power. Put any number of land cards from among them onto the battlefield tapped, then shuffle.";
const THROMOK: &str = "Mana cost: {3}{R}{G}\nType: Legendary Creature — Hellion\nPower/Toughness: 0/0\nDevour X, where X is the number of creatures devoured this way (As this creature enters, you may sacrifice any number of creatures. It enters with X +1/+1 counters on it for each of those creatures.)";

fn devour_effect(definition: &CardDefinition) -> DevourEffect {
    fn find(effect: &ironsmith::effect::Effect) -> Option<DevourEffect> {
        if let Some(devour) = effect.downcast_ref::<DevourEffect>() { return Some(devour.clone()); }
        let mut found = None;
        effect.visit_child_effects(&mut |child| { if found.is_none() { found = find(child); } });
        found
    }
    let mut abilities = definition.abilities.clone();
    for ability in &definition.abilities {
        if let AbilityKind::Static(static_ability) = &ability.kind {
            if let Some(model) = static_ability.compiled_model() {
                if let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) = &model.payload {
                    abilities.push(ironsmith::static_abilities::StaticAbilityModelInterpreter::ability_from_model(&grant.ability));
                    abilities.extend(grant.additional_abilities.iter().map(ironsmith::static_abilities::StaticAbilityModelInterpreter::ability_from_model));
                }
            }
        }
    }
    abilities.iter()
        .find_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            let ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram { program, .. } =
                &static_ability.compiled_model()?.payload
            else {
                return None;
            };
            program
                .flattened_default_effects()
                .into_iter()
                .find_map(find)
        })
        .unwrap_or_else(|| panic!("{}: devour must lower to an as-enters devour program", definition.card.name))
}

#[test]
fn devour_quality_and_devoured_count_variants_compile_on_both_routes() {
    for (name, text, multiplier, card_type, subtype, squared) in [
        ("Caprichrome", CAPRICHROME, 1, Some(CardType::Artifact), None, false),
        ("Feasting Hobbit", FEASTING_HOBBIT, 3, None, Some(Subtype::Food), false),
        ("Famished Worldsire", FAMISHED_WORLDSIRE, 3, Some(CardType::Land), None, false),
        ("Thromok the Insatiable", THROMOK, 1, None, None, true),
    ] {
        for definition in compile::compile_both(name, text) {
            let devour = devour_effect(&definition);
            assert_eq!(devour.multiplier, multiplier, "{name}");
            assert_eq!(devour.multiplier_is_devoured_count, squared, "{name}");
            match (card_type, subtype) {
                (None, None) => assert!(devour.quality.is_none(), "{name}: creatures"),
                (card_type, subtype) => {
                    let quality = devour.quality.as_ref().expect("quality filter");
                    if let Some(card_type) = card_type {
                        assert_eq!(quality.card_types, vec![card_type], "{name}");
                    }
                    if let Some(subtype) = subtype {
                        assert_eq!(quality.subtypes, vec![subtype], "{name}");
                    }
                }
            }
        }
    }
}

struct DevourAll;
impl DecisionMaker for DevourAll {
    fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        context
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect()
    }
}

fn permanent(game: &mut GameState, types: Vec<CardType>, subtypes: Vec<Subtype>) -> ObjectId {
    let mut builder = CardBuilder::new(CardId::new(), "Devour fodder")
        .card_types(types.clone())
        .subtypes(subtypes);
    if types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(1, 1));
    }
    game.create_object_from_card(&builder.build(), A, Zone::Battlefield)
}

fn enter(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let hand = game.create_object_from_definition(definition, A, Zone::Hand);
    let receipt = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut DevourAll)
        .unwrap();
    let id = receipt.original.into_result().unwrap().new_id;
    game.refresh_continuous_state().unwrap();
    id
}

#[test]
fn devour_artifact_sacrifices_only_artifacts_and_counts_them() {
    for definition in compile::compile_both("Caprichrome", CAPRICHROME) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let artifact = permanent(&mut game, vec![CardType::Artifact], vec![]);
        let artifact_creature =
            permanent(&mut game, vec![CardType::Artifact, CardType::Creature], vec![]);
        let creature = permanent(&mut game, vec![CardType::Creature], vec![]);
        let goat = enter(&mut game, &definition);
        assert_eq!(game.counter_count(goat, CounterType::PlusOnePlusOne), 2);
        assert!(game.object(artifact).is_none_or(|object| object.zone != Zone::Battlefield));
        assert!(
            game.object(artifact_creature)
                .is_none_or(|object| object.zone != Zone::Battlefield)
        );
        assert_eq!(
            game.object(creature).map(|object| object.zone),
            Some(Zone::Battlefield),
            "a nonartifact creature is not devourable by devour artifact"
        );
    }
}

#[test]
fn devour_food_three_gives_three_counters_per_food() {
    for definition in compile::compile_both("Feasting Hobbit", FEASTING_HOBBIT) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        permanent(&mut game, vec![CardType::Artifact], vec![Subtype::Food]);
        permanent(&mut game, vec![CardType::Artifact], vec![Subtype::Food]);
        let other_artifact = permanent(&mut game, vec![CardType::Artifact], vec![]);
        let hobbit = enter(&mut game, &definition);
        assert_eq!(game.counter_count(hobbit, CounterType::PlusOnePlusOne), 6);
        assert_eq!(
            game.object(other_artifact).map(|object| object.zone),
            Some(Zone::Battlefield)
        );
    }
}

#[test]
fn thromok_gets_the_devoured_count_squared() {
    for definition in compile::compile_both("Thromok the Insatiable", THROMOK) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        for _ in 0..3 {
            permanent(&mut game, vec![CardType::Creature], vec![]);
        }
        let thromok = enter(&mut game, &definition);
        assert_eq!(game.counter_count(thromok, CounterType::PlusOnePlusOne), 9);
    }
}
