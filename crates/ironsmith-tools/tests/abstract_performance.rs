//! Frozen atlas observation 25231510: fixed library piles and resolution-time choices.
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext, ViewCardsContext};
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};
struct Piles {
    pile: usize,
    opponent: PlayerId,
    accept: bool,
    pile_prompts: usize,
    cast_prompts: usize,
}
impl DecisionMaker for Piles {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.options.iter().any(|o| o.description == "First pile") {
            assert_eq!(ctx.player, self.opponent);
            assert_eq!(ctx.options.len(), 2);
            assert!(
                ctx.options.iter().all(|o| o.legal),
                "empty piles remain choices"
            );
            for &id in &game.exile {
                if game.is_face_down(id) {
                    for player in &game.players {
                        assert!(!game.can_player_look_at_face_down_exiled_card(id, player.id));
                    }
                }
            }
            self.pile_prompts += 1;
            vec![self.pile]
        } else if let Some(option) = ctx
            .options
            .iter()
            .find(|o| o.description == game.player(self.opponent).unwrap().name.as_ref())
        {
            assert_eq!(ctx.player, PlayerId::from_index(0));
            vec![option.index]
        } else {
            vec![ctx.options.iter().find(|o| o.legal).unwrap().index]
        }
    }
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        assert_eq!(ctx.player, PlayerId::from_index(0));
        self.cast_prompts += 1;
        self.accept
    }
    fn view_cards(
        &mut self,
        game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        _: &ViewCardsContext,
    ) {
        if cards.iter().any(|id| game.is_face_down(*id)) {
            assert_eq!(viewer, PlayerId::from_index(0));
            assert_eq!(
                self.pile_prompts, 1,
                "private viewing follows the opponent's pile choice"
            );
        }
    }
}
#[test]
fn canonical_fixed_piles_preserve_order_empty_choices_and_cast_remainder() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::ids::CardId;
    let payloads = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Abstract Performance",
    )
    .unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice = PlayerId::from_index(0);
    let carol = PlayerId::from_index(2);
    for library_size in 0usize..=9 {
        for pile in 0..2 {
            for accept in [false, true] {
                let mut game =
                    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                let mut stable = Vec::new();
                for index in 0..library_size {
                    let fixture = CardDefinitionBuilder::new(
                        CardId::new(),
                        format!("Library fixture {index}"),
                    )
                    .card_types(vec![CardType::Artifact])
                    .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![
                        ironsmith::mana::ManaSymbol::Generic(7),
                    ]]))
                    .build();
                    let id = game.create_object_from_definition(&fixture, alice, Zone::Library);
                    stable.push(game.object(id).unwrap().stable_id);
                }
                let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
                game.push_to_stack(ironsmith::game_state::StackEntry::new(source, alice));
                let mut dm = Piles {
                    pile,
                    opponent: carol,
                    accept,
                    pile_prompts: 0,
                    cast_prompts: 0,
                };
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(dm.pile_prompts, 1, "size={library_size} pile={pile}");
                let first = library_size.min(4);
                let second = library_size.saturating_sub(4).min(4);
                let other_count = if pile == 0 { second } else { first };
                let cast = usize::from(accept && other_count > 0);
                assert_eq!(
                    game.stack.len(),
                    cast,
                    "size={library_size} pile={pile} accept={accept}"
                );
                assert_eq!(game.player(alice).unwrap().hand.len(), other_count - cast);
                let mut cast_count = 0;
                for (index, id) in stable.iter().enumerate() {
                    let current = game.find_object_by_stable_id(*id).unwrap();
                    let zone = game.object(current).unwrap().zone;
                    let from_top = library_size - 1 - index;
                    if from_top >= 8 {
                        assert_eq!(zone, Zone::Library);
                    } else if (from_top < 4) == (pile == 0) {
                        assert_eq!(zone, Zone::Graveyard);
                        assert!(!game.is_face_down(current));
                    } else {
                        assert!(matches!(zone, Zone::Hand | Zone::Stack));
                        if zone == Zone::Stack {
                            cast_count += 1;
                        }
                    }
                }
                assert_eq!(cast_count, cast);
                assert!(game.exile.is_empty());
            }
        }
    }
}
