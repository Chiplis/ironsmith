//! Repeated processes whose later rounds exclude earlier rounds' choices
//! ("repeat this process except that opponent can't choose a card already
//! chosen for ~", Forgotten Lore, Shrouded Lore), and the coin-flip process
//! whose loss branch repeats only when its unless-payment is made (Crooked
//! Scales: the per-sentence SourceSentence wrapper reached lowering with the
//! marker unconverted). Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::BooleanContext;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, RepeatProcessEffect, ResolvedTarget, execute_effect};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/repeat_process_choice_history.json.fixture"
    ))
    .unwrap()
}

fn row(name: &str) -> serde_json::Value {
    fixtures().into_iter().find(|row| row["name"] == name).unwrap()
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

fn repeat_of(effects: &[Effect]) -> RepeatProcessEffect {
    let mut all = Vec::new();
    for effect in effects {
        collect(effect, &mut all);
    }
    all.iter()
        .find_map(|effect| effect.downcast_ref::<RepeatProcessEffect>())
        .cloned()
        .expect("one repeated process")
}

#[test]
fn lore_cards_accumulate_the_chosen_cards_across_rounds() {
    for name in ["Forgotten Lore", "Shrouded Lore"] {
        for definition in definitions(name, row(name)["text"].as_str().unwrap()) {
            let spell = definition.spell_effect.as_ref().expect("sorcery effects");
            let repeat = repeat_of(spell.flattened_default_effects());
            let [history] = repeat.choice_history.as_slice() else {
                panic!("{name}: one accumulated choice: {repeat:?}");
            };
            assert_eq!(
                history.previously_chosen.as_str(),
                ironsmith::tag::PRIOR_PROCESS_CHOICES_TAG
            );
            let body = format!("{:?}", repeat.effects);
            assert!(body.contains("ChooseObjectsEffect"), "{name}: {body}");
            assert!(body.contains("IsNotTaggedObject"), "{name}: {body}");
            assert!(body.contains("PayManaEffect"), "{name}: {body}");
            assert_eq!(repeat.predicate, ironsmith::effect::EffectPredicate::Happened);
            // The move to hand follows the loop and reads the final round.
            let all = format!("{:?}", spell.flattened_default_effects());
            assert!(all.contains(history.chosen.as_str()), "{name}: {all}");
            assert!(all.contains("Hand"), "{name}: {all}");
        }
    }
}

struct Script {
    booleans: Vec<bool>,
}

impl DecisionMaker for Script {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        if self.booleans.is_empty() { false } else { self.booleans.remove(0) }
    }
}

fn graveyard_card(game: &mut GameState, name: &str) -> ObjectId {
    let definition =
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, "Type: Artifact", false)
            .unwrap();
    game.create_object_from_definition(&definition, A, Zone::Graveyard)
}

#[test]
fn forgotten_lore_never_offers_a_card_chosen_in_an_earlier_round() {
    let text = row("Forgotten Lore")["text"].as_str().unwrap().to_string();
    for definition in definitions("Forgotten Lore", &text) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let first = graveyard_card(&mut game, "First");
        let second = graveyard_card(&mut game, "Second");
        let third = graveyard_card(&mut game, "Third");
        let third_stable = game.object(third).unwrap().stable_id;
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::ManaSymbol::Green, 2);
        let spell = game.create_object_from_definition(&definition, A, Zone::Stack);
        // Pay {G} twice, then stop: three rounds, each choosing the first
        // legal card. Without the history every round would pick `first`.
        let mut dm = Script { booleans: vec![true, true, false] };
        let mut ctx = EffectContext::new(spell, A, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(B)]);
        for effect in definition
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects()
        {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0, "both {{G}} paid");
        assert_eq!(game.object(first).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(second).unwrap().zone, Zone::Graveyard);
        let in_hand = game.player(A).unwrap().hand.clone();
        assert_eq!(in_hand.len(), 1, "only the last chosen card moves");
        assert_eq!(
            game.object(in_hand[0]).unwrap().stable_id,
            third_stable,
            "the third round chose the only card not chosen before"
        );
    }
}

#[test]
fn crooked_scales_loss_branch_repeats_on_the_declined_destruction() {
    let text = row("Crooked Scales")["text"].as_str().unwrap().to_string();
    for definition in definitions("Crooked Scales", &text) {
        let activated = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("activated ability");
        let repeat = repeat_of(activated.effects.flattened_default_effects());
        assert_eq!(repeat.predicate, ironsmith::effect::EffectPredicate::WasDeclined);
        let body = format!("{:?}", repeat.effects);
        assert!(body.contains("FlipCoinEffect"), "{body}");
        assert!(body.contains("UnlessPaysEffect"), "{body}");
        assert!(!body.contains("RepeatThisProcess"), "{body}");
        assert!(repeat.choice_history.is_empty());
    }
}
