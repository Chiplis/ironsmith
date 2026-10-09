//! Filtered ETB replacements whose entering subject is an open object filter
//! (CR 614.1c/614.12). The grammars existed but were unreachable from these
//! subject heads. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{CardId, CardType, CounterType, GameState, ObjectId, PlayerId, Subtype, Supertype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const GOND_GATE: &str = "Type: Land — Gate\nGates you control enter untapped.\n{T}: Add {C}.\n{T}: Add one mana of any color that a Gate you control could produce.";
const PHYREXIAN_CENSOR: &str = "Mana cost: {2}{W}\nType: Creature — Phyrexian Wizard\nPower/Toughness: 3/3\nEach player can't cast more than one non-Phyrexian spell each turn.\nNon-Phyrexian creatures enter tapped.";
const BARD_CLASS: &str = "Mana cost: {R}{G}\nType: Enchantment — Class\n(Gain the next level as a sorcery to add its ability.)\nLegendary creatures you control enter with an additional +1/+1 counter on them.\n{R}{G}: Level 2\nLegendary spells you cast cost {R}{G} less to cast. This effect reduces only the amount of colored mana you pay.\n{3}{R}{G}: Level 3\nWhenever you cast a legendary spell, exile the top two cards of your library. You may play them this turn.";

fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

fn static_count(definition: &CardDefinition, id: StaticAbilityId) -> usize {
    definition
        .abilities
        .iter()
        .filter(|ability| matches!(&ability.kind, AbilityKind::Static(ability) if ability.id() == id))
        .count()
}

fn permanent_card(
    name: &str,
    types: Vec<CardType>,
    supertypes: Vec<Supertype>,
    subtypes: Vec<Subtype>,
) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(types.clone())
        .supertypes(supertypes)
        .subtypes(subtypes);
    if types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

/// Puts a card from `owner`'s hand onto the battlefield through the full ETB
/// replacement pipeline and returns the new permanent.
fn enter(game: &mut GameState, owner: PlayerId, definition: &CardDefinition) -> ObjectId {
    let hand = game.create_object_from_definition(definition, owner, Zone::Hand);
    let receipt = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut SelectFirstDecisionMaker)
        .unwrap();
    assert!(!receipt.pending);
    receipt.original.into_result().unwrap().new_id
}

fn plus_counters(game: &GameState, id: ObjectId) -> u32 {
    game.object(id)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0)
}

#[test]
fn open_subject_entry_replacements_compile_on_both_routes() {
    for definition in routes("Gond Gate", GOND_GATE) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert_eq!(static_count(&definition, StaticAbilityId::EnterUntappedForFilter), 1);
    }
    for definition in routes("Phyrexian Censor", PHYREXIAN_CENSOR) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert_eq!(static_count(&definition, StaticAbilityId::EnterTappedForFilter), 1);
    }
    for definition in routes("Bard Class", BARD_CLASS) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert_eq!(static_count(&definition, StaticAbilityId::EnterWithCountersForFilter), 1);
    }
}

#[test]
fn gond_gate_untaps_only_gates_its_controller_controls() {
    for definition in routes("Gond Gate", GOND_GATE) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        enter(&mut game, A, &definition);
        // A Gate that would enter tapped by its own replacement.
        let tapped_gate_text = "Type: Land — Gate\nThis land enters tapped.\n{T}: Add {C}.";
        let tapped_gate = routes("Tapped gate", tapped_gate_text)[0].clone();
        let mine = enter(&mut game, A, &tapped_gate);
        assert!(!game.is_tapped(mine), "CR 614.12: Gond Gate's replacement applies to Gates you control");
        let theirs = enter(&mut game, B, &tapped_gate);
        assert!(game.is_tapped(theirs), "an opponent's Gate is outside the filter");
        let tapland = routes("Tapped land", "Type: Land\nThis land enters tapped.\n{T}: Add {C}.")[0].clone();
        let other = enter(&mut game, A, &tapland);
        assert!(game.is_tapped(other), "non-Gate lands are outside the filter");
    }
}

#[test]
fn phyrexian_censor_taps_only_non_phyrexian_creatures_for_every_player() {
    for definition in routes("Phyrexian Censor", PHYREXIAN_CENSOR) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let censor = enter(&mut game, A, &definition);
        assert!(!game.is_tapped(censor), "the Censor itself is Phyrexian");
        let human = permanent_card("Human", vec![CardType::Creature], vec![], vec![Subtype::Human]);
        let phyrexian =
            permanent_card("Phyrexian", vec![CardType::Creature], vec![], vec![Subtype::Phyrexian]);
        let land = permanent_card("Land", vec![CardType::Land], vec![], vec![]);
        for player in [A, B] {
            let id = enter(&mut game, player, &human);
            assert!(game.is_tapped(id));
            let id = enter(&mut game, player, &phyrexian);
            assert!(!game.is_tapped(id));
            let id = enter(&mut game, player, &land);
            assert!(!game.is_tapped(id), "noncreature permanents are outside the filter");
        }
    }
}

#[test]
fn bard_class_adds_one_counter_to_legendary_creatures_you_control_only() {
    for definition in routes("Bard Class", BARD_CLASS) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        enter(&mut game, A, &definition);
        let legend = permanent_card(
            "Legend",
            vec![CardType::Creature],
            vec![Supertype::Legendary],
            vec![Subtype::Human],
        );
        let plain = permanent_card("Plain", vec![CardType::Creature], vec![], vec![Subtype::Human]);
        let mine = enter(&mut game, A, &legend);
        assert_eq!(plus_counters(&game, mine), 1);
        let ordinary = enter(&mut game, A, &plain);
        assert_eq!(plus_counters(&game, ordinary), 0);
        let theirs = enter(&mut game, B, &legend);
        assert_eq!(plus_counters(&game, theirs), 0, "only legendary creatures you control");
    }
}
