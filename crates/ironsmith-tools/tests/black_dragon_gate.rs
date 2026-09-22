use ironsmith::ability::{AbilityKind, ActivatedAbilityRuntimeExt};
use ironsmith::color::Color;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::ColorsContext;
use ironsmith::mana::ManaSymbol;
use ironsmith::{GameState, PlayerId, Zone};

#[derive(Default)]
struct ChooseBlue {
    prompts: usize,
    suspend: bool,
}

impl DecisionMaker for ChooseBlue {
    fn awaiting_choice(&self) -> bool {
        self.suspend && self.prompts > 0
    }

    fn decide_colors(&mut self, game: &GameState, ctx: &ColorsContext) -> Vec<Color> {
        self.prompts += 1;
        assert_eq!(ctx.player, PlayerId::from_index(0));
        assert_eq!(ctx.count, 1);
        assert_eq!(game.object(ctx.source.unwrap()).unwrap().zone, Zone::Hand);
        assert_eq!(
            ctx.available_colors.as_deref(),
            Some([Color::White, Color::Blue, Color::Red, Color::Green].as_slice())
        );
        vec![Color::Blue]
    }
}

fn definition(artifact_roundtrip: bool) -> ironsmith::cards::CardDefinition {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Black Dragon Gate",
    )
    .unwrap()
    .remove(0);
    if artifact_roundtrip {
        let compiled = ironsmith_compiler::CompilerFacade::new()
            .compile_definition(
                ironsmith_compiler::CardDefinitionBuilder::new(
                    ironsmith::ids::CardId::new(),
                    &payload.name,
                ),
                payload.parse_input.clone(),
                ironsmith_compiler::CompilePolicy {
                    allow_unsupported: false,
                },
            )
            .unwrap();
        let wire =
            serde_json::from_value(serde_json::to_value(&compiled.definition).unwrap()).unwrap();
        ironsmith::artifact_materializer::materialize_definition(wire).unwrap()
    } else {
        ironsmith_tools::compile_definition_from_payload(&payload).unwrap()
    }
}

#[test]
fn black_dragon_gate_prompts_before_entry_and_remembers_nonblack_color() {
    for artifact_roundtrip in [false, true] {
        let definition = definition(artifact_roundtrip);
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let mut dm = ChooseBlue::default();
        let entered = game
            .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
            .unwrap()
            .new_id;
        assert_eq!(
            dm.prompts, 1,
            "entry must ask the controller to choose a nonblack color"
        );
        assert_eq!(game.chosen_color(entered), Some(Color::Blue));
        assert!(game.is_tapped(entered));
        let AbilityKind::Activated(mana) = &definition.abilities[2].kind else {
            panic!("expected mana ability")
        };
        let symbols = mana.inferred_mana_symbols(&game, entered, alice);
        assert_eq!(symbols.len(), 2);
        assert!(symbols.contains(&ManaSymbol::Black));
        assert!(symbols.contains(&ManaSymbol::Blue));
    }
}

#[test]
fn black_dragon_gate_waits_for_the_color_choice_before_entering() {
    let definition = definition(true);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let hand = game.create_object_from_definition(&definition, PlayerId::from_index(0), Zone::Hand);
    let mut dm = ChooseBlue {
        suspend: true,
        ..Default::default()
    };
    assert!(
        game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
            .is_none()
    );
    assert_eq!(dm.prompts, 1);
    assert_eq!(game.object(hand).unwrap().zone, Zone::Hand);
    assert!(game.battlefield.is_empty());
    assert!(game.chosen_color(hand).is_none());

    dm.suspend = false;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .unwrap()
        .new_id;
    assert_eq!(game.chosen_color(entered), Some(Color::Blue));
    assert!(game.is_tapped(entered));
}
