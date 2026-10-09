//! "Until end of turn, if a [nontoken] creature would enter [and it wasn't
//! cast], exile it instead" is a turn-scoped replacement (CR 614.12), never
//! a one-shot "Exile it." Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, RegisterFutureZoneReplacementEffect, execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const HALLOWED_MOONLIGHT: &str = "Mana cost: {1}{W}\nType: Instant\nUntil end of turn, if a creature would enter and it wasn't cast, exile it instead.\nDraw a card.";
const MISTCALLER: &str = "Mana cost: {U}\nType: Creature — Merfolk Wizard\nPower/Toughness: 1/1\nSacrifice this creature: Until end of turn, if a nontoken creature would enter and it wasn't cast, exile it instead.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

fn replacement(definition: &CardDefinition) -> RegisterFutureZoneReplacementEffect {
    fn collect(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| collect(child, out));
    }
    let mut all = Vec::new();
    if let Some(program) = &definition.spell_effect {
        for effect in program.all_effects() {
            collect(effect, &mut all);
        }
    }
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind {
            for effect in activated.effects.all_effects() {
                collect(effect, &mut all);
            }
        }
    }
    assert!(
        !all.iter().any(|effect| format!("{effect:?}").contains("MoveToZone")),
        "no one-shot exile of an antecedent"
    );
    all.iter()
        .find_map(|effect| effect.downcast_ref::<RegisterFutureZoneReplacementEffect>().cloned())
        .expect("turn-scoped replacement registration")
}

fn move_to_battlefield(game: &mut GameState, source: ObjectId, object: ObjectId) -> Zone {
    let stable = game.object(object).unwrap().stable_id;
    let mut dm = SelectFirstDecisionMaker;
    execute_effect(
        game,
        &Effect::move_to_zone(ChooseSpec::SpecificObject(object), Zone::Battlefield, false),
        &mut EffectContext::new(source, A, &mut dm),
    )
    .unwrap();
    let moved = game.find_object_by_stable_id(stable).unwrap();
    game.object(moved).unwrap().zone
}

#[test]
fn hallowed_moonlight_and_mistcaller_exile_uncast_entries_this_turn() {
    for (name, text) in [("Hallowed Moonlight", HALLOWED_MOONLIGHT), ("Mistcaller", MISTCALLER)] {
        for definition in definitions(name, text) {
            let register = replacement(&definition);
            assert_eq!(register.to_zone, Some(Zone::Battlefield));
            assert_eq!(register.replacement_zone, Zone::Exile);
            assert_eq!(register.mode, ironsmith::effects::ReplacementApplyMode::UntilEndOfTurn);
            assert_eq!(register.filter.nontoken, name == "Mistcaller");

            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = Phase::FirstMain;
            let bear =
                compile_to_runtime_definition("Bear", "Type: Creature — Bear\nPower/Toughness: 2/2", false).unwrap();
            let source = game.create_object_from_definition(&bear, A, Zone::Battlefield);
            let mut dm = SelectFirstDecisionMaker;
            execute_effect(&mut game, &Effect::new(register.clone()), &mut EffectContext::new(source, A, &mut dm)).unwrap();
            // Reanimated (not cast): exiled instead, and again for a second one.
            for _ in 0..2 {
                let card = game.create_object_from_definition(&bear, A, Zone::Graveyard);
                assert_eq!(move_to_battlefield(&mut game, source, card), Zone::Exile);
            }
            game.effect_store.replacement_effects.clear_until_end_of_turn_effects();
            let card = game.create_object_from_definition(&bear, A, Zone::Graveyard);
            assert_eq!(move_to_battlefield(&mut game, source, card), Zone::Battlefield, "expires with the turn");
        }
    }
}
