//! "Casualty X. The copy isn't legendary and has starting loyalty X."
//! (CR 702.153a, CR 707.10). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const OB_NIXILIS: &str = "Mana cost: {1}{B}{R}\nType: Legendary Planeswalker — Nixilis\nLoyalty: 3\nCasualty X. The copy isn't legendary and has starting loyalty X. (As you cast this spell, you may sacrifice a creature with power X. When you do, copy this spell. The copy becomes a token.)\n+1: Each opponent loses 2 life unless they discard a card. If you control a Demon or Devil, you gain 2 life.\n−2: Create a 1/1 red Devil creature token with \"When this token dies, it deals 1 damage to any target.\"\n−7: Target player draws seven cards and loses 7 life.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(
            !ironsmith::cards::generated_definition_has_unimplemented_content(definition),
            "{name}: unimplemented content"
        );
    }
    [direct, decoded]
}

#[test]
fn ob_nixilis_casualty_copy_is_supported_and_rendered() {
    for definition in definitions("Ob Nixilis, the Adversary", OB_NIXILIS) {
        let debug = format!("{definition:?}");
        assert!(
            debug.contains("VariableCasualtyPlaneswalkerCopy"),
            "{debug}"
        );
    }
}

#[test]
fn printed_casualty_receipt_survives_compilation_materialization_and_payment() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use ironsmith::effects::{
        EffectContext as ExecutionContext, EffectExecutor, VariableCasualtyPlaneswalkerCopyEffect,
    };
    use ironsmith::{CardId, GameState, PlayerId, Zone};
    struct PayCasualty;
    impl DecisionMaker for PayCasualty {
        fn decide_options(
            &mut self,
            game: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx.description.starts_with("Choose optional costs for") {
                return ctx
                    .options
                    .iter()
                    .filter(|option| option.legal)
                    .take(1)
                    .map(|option| option.index)
                    .collect();
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &ironsmith::decisions::context::SelectObjectsContext,
        ) -> Vec<ironsmith::ObjectId> {
            SelectFirstDecisionMaker.decide_objects(game, ctx)
        }
    }
    for definition in definitions("Ob Nixilis, the Adversary", OB_NIXILIS) {
        let effect = definition
            .abilities
            .iter()
            .find_map(|ability| {
                let AbilityKind::Triggered(trigger) = &ability.kind else {
                    return None;
                };
                trigger
                    .effects
                    .iter()
                    .find_map(|effect| {
                        effect.downcast_ref::<VariableCasualtyPlaneswalkerCopyEffect>()
                    })
                    .cloned()
            })
            .expect("casualty copy adapter");
        assert!(effect.cost_ref.discriminator.is_some());
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let player = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, player, Zone::Hand);
        let creature = ironsmith::card::CardBuilder::new(CardId::new(), "Paid creature")
            .card_types(vec![ironsmith::types::CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(7, 7))
            .build();
        game.create_object_from_card(&creature, player, Zone::Battlefield);
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let cast_id = {
            let mut dm = PayCasualty;
            let mut ctx = ExecutionContext::new(source, player, &mut dm);
            ctx.set_tagged_objects("cast_fixture", vec![snapshot]);
            let cast = ironsmith::effects::CastTaggedEffect::new(
                "cast_fixture",
                ironsmith::target::PlayerFilter::You,
            )
            .without_paying_mana_cost()
            .execute(&mut game, &mut ctx)
            .unwrap();
            cast.objects().expect("cast completes")[0]
        };
        let mut ctx = ExecutionContext::new_default(cast_id, player);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        let copy = game
            .object(outcome.objects().expect("copy result")[0])
            .unwrap();
        assert_eq!(copy.base_loyalty, Some(7));
        assert!(
            !copy
                .supertypes
                .contains(&ironsmith::types::Supertype::Legendary)
        );
    }
}
