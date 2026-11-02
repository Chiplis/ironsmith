//! Actual source operands retain prior-choice identity and dynamic library legality.
#[path = "custom_final_next_wave_support.rs"]
mod source;
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/revolution-solidarity-complete.json"
    ))
    .unwrap();
    let card = data["mechanics"][0]["representative_cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|card| card["name"] == name)
        .unwrap();
    source::definitions(card).unwrap()
}
fn creature(game: &mut GameState, owner: PlayerId, zone: Zone, types: &str) -> ObjectId {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Type witness",
        &format!("Mana cost: {{0}}\nType: Creature — {types}\nPower/Toughness: 1/1"),
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn can_cast(game: &GameState, id: ObjectId) -> bool {
    ironsmith::decision::compute_legal_actions(game,A).unwrap().iter().any(|action|matches!(action,ironsmith::decision::LegalAction::CastSpell{spell_id,..}if *spell_id==id))
}
#[test]
fn actual_library_permission_accepts_partial_overlap_and_rechecks_the_controlled_union() {
    for definition in definitions("Sharesh, Leader of the Lost") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        creature(&mut game, A, Zone::Battlefield, "Human");
        creature(&mut game, B, Zone::Battlefield, "Elf");
        let buried = creature(&mut game, A, Zone::Library, "Human Elf");
        let top = creature(&mut game, A, Zone::Library, "Human Elf");
        assert!(!can_cast(&game, buried), "permission remains top-card-only");
        assert!(
            can_cast(&game, top),
            "Human overlap allowed: Elf absent among your creatures"
        );
        let cover = creature(&mut game, A, Zone::Battlefield, "Elf");
        assert!(!can_cast(&game, top), "all candidate types now represented");
        game.move_object_by_effect(cover, Zone::Graveyard).unwrap();
        assert!(
            can_cast(&game, top),
            "controlled union queried again after departure"
        );
    }
}
struct TypeChoices(usize);
impl ironsmith::decision::DecisionMaker for TypeChoices {
    fn decide_options(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        let name = if self.0 == 0 { "Zombie" } else { "Elf" };
        self.0 += 1;
        vec![
            context
                .options
                .iter()
                .find(|option| option.description.eq_ignore_ascii_case(name))
                .unwrap()
                .index,
        ]
    }
}
#[test]
fn actual_pump_then_become_uses_one_prior_choice_and_preserves_each_completed_type_change() {
    use ironsmith::effects::{EffectContext, execute_effect};
    for definition in definitions("Ragecloak Shifter") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let actor = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let program = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::AbilityKind::Activated(activated) => Some(activated.effects.clone()),
                _ => None,
            })
            .unwrap();
        let mut choices = TypeChoices(0);
        for _ in 0..2 {
            let mut context =
                EffectContext::new_default(actor, A).with_decision_maker(&mut choices);
            for effect in program.all_effects_owned() {
                execute_effect(&mut game, &effect, &mut context).unwrap();
            }
        }
        let types = game.calculated_subtypes(actor);
        assert!(
            types.contains(&ironsmith::Subtype::Shapeshifter)
                && types.contains(&ironsmith::Subtype::Shaman)
        );
        assert_eq!(choices.0, 2, "exactly one authored choice per activation");
        assert!(
            types.contains(&ironsmith::Subtype::Zombie) && types.contains(&ironsmith::Subtype::Elf),
            "each completed modification retains its own chosen operand"
        );
        assert_eq!(
            game.calculated_power(actor),
            Some(3),
            "both ordinary pumps retained alongside type changes"
        );
    }
}
