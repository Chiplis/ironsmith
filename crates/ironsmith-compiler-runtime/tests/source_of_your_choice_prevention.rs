//! "Prevent all damage a [red] source of your choice would deal [to you] this
//! turn": one source, limited by the descriptor, is chosen on resolution
//! (CR 609.7a) and the shield then follows that object.
//! Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, PreventAllDamageEffect, execute_effect};
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::events::DamageTarget;
use ironsmith::{CardId, CardType, ColorSet, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/source_of_your_choice_prevention.json.fixture"
    ))
    .unwrap()
}

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
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
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}

fn all_effects(definition: &CardDefinition) -> Vec<Effect> {
    let mut all = Vec::new();
    if let Some(program) = &definition.spell_effect {
        for effect in program.all_effects() {
            collect(effect, &mut all);
        }
    }
    for ability in &definition.abilities {
        if let AbilityKind::Activated(activated) = &ability.kind {
            for effect in activated.effects.all_effects() {
                collect(effect, &mut all);
            }
        }
    }
    all
}

#[test]
fn each_source_choice_shield_keeps_its_recipient_and_choice_limit_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 4);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, row["text"].as_str().unwrap()) {
            let all = all_effects(&definition);
            let shields: Vec<_> = all
                .iter()
                .filter_map(|effect| effect.downcast_ref::<PreventAllDamageEffect>())
                .collect();
            assert_eq!(shields.len(), 1, "{name}");
            let shield = shields[0];
            assert!(shield.source_of_your_choice, "{name}: the source is chosen");
            assert!(!shield.source_choice_shares_activation_mana_color, "{name}");
            assert!(shield.source_target.is_none(), "{name}: not a targeted source");
            assert_eq!(shield.until, ironsmith::effect::Until::EndOfTurn, "{name}");
            let expected_target = if name == "Auriok Replica" {
                ironsmith::prevention::PreventionTarget::You
            } else {
                ironsmith::prevention::PreventionTarget::All
            };
            assert_eq!(shield.target, expected_target, "{name}");
            match name {
                "Burrenton Forge-Tender" => {
                    let filter = shield.damage_filter.from_source.as_ref().expect("red limit");
                    assert_eq!(filter.colors, Some(ColorSet::RED));
                }
                _ => assert!(shield.damage_filter.from_source.is_none(), "{name}"),
            }
        }
    }
}

fn creature(game: &mut GameState, owner: PlayerId, colors: ColorSet) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Damage source")
            .card_types(vec![CardType::Creature])
            .color_indicator(colors)
            .power_toughness(PowerToughness::fixed(3, 10))
            .build(),
        owner,
        Zone::Battlefield,
    )
}

fn damage(game: &mut GameState, source: ObjectId, target: ObjectId) -> u32 {
    let result = process_damage_assignments_with_event_with_source_snapshot_opts(
        game,
        source,
        DamageTarget::Object(target),
        3,
        false,
        false,
        EventCause::effect(),
        None,
    )
    .unwrap();
    result.assignments.iter().map(|assignment| assignment.amount).sum()
}

#[test]
fn burrenton_choice_is_limited_to_red_sources_and_follows_the_chosen_source() {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == "Burrenton Forge-Tender")
        .unwrap();
    for definition in definitions("Burrenton Forge-Tender", row["text"].as_str().unwrap()) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let tender = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        // Created first, so an unrestricted first-candidate choice would pick it.
        let green = creature(&mut game, B, ColorSet::GREEN);
        let red = creature(&mut game, B, ColorSet::RED);
        let recipient = creature(&mut game, A, ColorSet::WHITE);
        let shield = all_effects(&definition)
            .into_iter()
            .find(|effect| effect.downcast_ref::<PreventAllDamageEffect>().is_some())
            .unwrap();
        let mut dm = SelectFirstDecisionMaker;
        execute_effect(&mut game, &shield, &mut EffectContext::new(tender, A, &mut dm)).unwrap();
        assert_eq!(damage(&mut game, red, recipient), 0, "the chosen red source is prevented");
        assert_eq!(damage(&mut game, green, recipient), 3, "other sources still deal damage");
        // The descriptor only limited the choice: a later color change does
        // not end the shield on the chosen object.
        game.object_mut(red).unwrap().color_override = Some(ColorSet::BLUE);
        assert_eq!(damage(&mut game, red, recipient), 0);
    }
}
