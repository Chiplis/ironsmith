//! Winter Orb: "As long as this artifact is untapped, players can't untap more
//! than one land during their untap steps."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Winter Orb",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
    let text = snapshot.compiled_text.clone().unwrap_or_default();
    assert!(
        text.starts_with("As long as this artifact is untapped, players can't untap"),
        "{text}"
    );
}

struct PickNamed(&'static str, usize);

impl DecisionMaker for PickNamed {
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.1 += 1;
        ctx.candidates
            .iter()
            .filter(|c| c.name == self.0)
            .map(|c| c.id)
            .collect()
    }
}

/// Bob has three tapped lands; Alice controls Winter Orb (tapped or not).
/// Returns which of Bob's lands are untapped after his untap step, and how
/// many limit prompts were asked.
fn untap(orb_tapped: bool) -> (Vec<bool>, usize) {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 4;
    game.turn.active_player = bob;
    let orb = game.create_object_from_definition(
        &ironsmith_tools::compile_definition_from_payload(&payload()).unwrap(),
        alice,
        Zone::Battlefield,
    );
    if orb_tapped {
        game.tap(orb);
    }
    let lands: Vec<_> = ["Land A", "Land B", "Land C"]
        .into_iter()
        .map(|name| {
            let def = CardDefinitionBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Land])
                .build();
            let id = game.create_object_from_definition(&def, bob, Zone::Battlefield);
            game.tap(id);
            id
        })
        .collect();
    game.refresh_continuous_state();
    let mut dm = PickNamed("Land B", 0);
    game.turn.phase = ironsmith::game_state::Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Untap);
    ironsmith::turn::execute_untap_step_with(&mut game, &mut dm);
    (lands.iter().map(|id| !game.is_tapped(*id)).collect(), dm.1)
}

#[test]
fn while_untapped_only_one_land_untaps() {
    let (untapped, prompts) = untap(false);
    assert_eq!(untapped, vec![false, true, false]);
    assert_eq!(prompts, 1);
}

#[test]
fn while_tapped_lands_untap_normally() {
    let (untapped, prompts) = untap(true);
    assert_eq!(untapped, vec![true, true, true]);
    assert_eq!(prompts, 0);
}
