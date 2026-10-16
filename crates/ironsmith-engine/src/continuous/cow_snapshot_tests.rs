use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::game_state::GameState;
use crate::ids::{CardId, PlayerId};
use crate::snapshot::SpellProgramState;
use crate::types::CardType;
use crate::zone::Zone;
use ironsmith_core::{Color, ColorSet, TextChange};

fn fixture() -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Shared snapshot fixture")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .build();
    let id = game.create_object_from_card(&card, PlayerId::from_index(0), Zone::Battlefield);
    game.object_mut(id).unwrap().abilities = vec![Ability::static_ability(
        StaticAbility::protection(crate::ability::ProtectionFrom::Color(ColorSet::BLACK)),
    )]
    .into();
    game.refresh_continuous_state().unwrap();
    (game, id)
}

#[test]
fn cow_snapshots_share_frozen_payloads_and_detach_nested_face_edits() {
    let (game, id) = fixture();
    let mut original = game.current_characteristics(id).unwrap();
    Arc::make_mut(original.copy_values.as_mut().unwrap()).other_copy_face =
        Some(Box::new(CopiableValues {
            name: "Other face".into(),
            ..Default::default()
        }));
    let mut branch = original.clone();
    assert!(Arc::ptr_eq(
        original.copy_values.as_ref().unwrap(),
        branch.copy_values.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(&original.spell_effect, &branch.spell_effect));
    branch.power = Some(9);
    branch.controller = PlayerId::from_index(1);
    let values = Arc::make_mut(branch.copy_values.as_mut().unwrap());
    values.name = "Branch copy".into();
    values.other_copy_face.as_mut().unwrap().name = "Changed face".into();
    assert_eq!(original.power, Some(2));
    assert_eq!(original.controller, PlayerId::from_index(0));
    assert_eq!(
        original.copy_values.as_ref().unwrap().name,
        "Shared snapshot fixture"
    );
    assert_eq!(
        original
            .copy_values
            .as_ref()
            .unwrap()
            .other_copy_face
            .as_ref()
            .unwrap()
            .name,
        "Other face"
    );
    assert!(!Arc::ptr_eq(
        original.copy_values.as_ref().unwrap(),
        branch.copy_values.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(&original.spell_effect, &branch.spell_effect));
}

#[test]
fn cow_text_rewrite_detaches_program_but_keeps_layer_one_values_frozen() {
    let (game, id) = fixture();
    let mut original = game.current_characteristics(id).unwrap();
    let mut filter = crate::target::ObjectFilter::creature();
    filter.colors = Some(ColorSet::BLACK);
    original.spell_effect = Arc::new(SpellProgramState::Present(
        crate::resolution::ResolutionProgram::from_effects(vec![crate::effect::Effect::new(
            crate::effects::DrawCardsEffect::you(crate::effect::Value::Count(filter)),
        )]),
    ));
    let captured = original.spell_effect.clone();
    let mut branch = original.clone();
    text_changes::apply_text_change(
        &mut branch,
        TextChange::color(Color::Black, Color::Blue).unwrap(),
        game.object(id).unwrap(),
    );
    assert!(branch.text_change_error.is_none());
    assert!(Arc::ptr_eq(&captured, &original.spell_effect));
    assert!(!Arc::ptr_eq(&original.spell_effect, &branch.spell_effect));
    assert_ne!(original.spell_effect, branch.spell_effect);
    assert!(Arc::ptr_eq(
        original.copy_values.as_ref().unwrap(),
        branch.copy_values.as_ref().unwrap()
    ));
    assert_eq!(
        original.static_abilities[0].protection_from(),
        Some(&crate::ability::ProtectionFrom::Color(ColorSet::BLACK))
    );
    assert_eq!(
        branch.static_abilities[0].protection_from(),
        Some(&crate::ability::ProtectionFrom::Color(ColorSet::BLUE))
    );
}

#[test]
fn cow_game_branch_and_retained_snapshot_match_full_recomputation() {
    let (game, id) = fixture();
    let captured = game.current_characteristics(id).unwrap();
    let mut branch = game.clone();
    branch.effect_store.continuous_effects.add_effect(
        ContinuousEffect::from_resolution(
            id,
            PlayerId::from_index(0),
            vec![id],
            Modification::RewriteText(TextChange::color(Color::Black, Color::Blue).unwrap()),
        )
        .until(crate::effect::Until::EndOfTurn),
    );
    branch.tap(id);
    branch
        .object_mut(id)
        .unwrap()
        .add_counters(crate::object::CounterType::PlusOnePlusOne, 2);
    branch.refresh_continuous_state().unwrap();
    for expire in [false, true] {
        if expire {
            branch.effect_store.continuous_effects.cleanup_end_of_turn();
        }
        branch.refresh_continuous_state().unwrap();
        let fast = branch.current_characteristics(id).unwrap();
        let mut full = branch.clone();
        full.mark_continuous_state_dirty();
        full.refresh_continuous_state().unwrap();
        assert_eq!(
            format!("{fast:?}"),
            format!("{:?}", full.current_characteristics(id).unwrap())
        );
        assert_eq!(fast.power, Some(4));
        assert_eq!(
            fast.static_abilities[0].protection_from(),
            Some(&crate::ability::ProtectionFrom::Color(if expire {
                ColorSet::BLACK
            } else {
                ColorSet::BLUE
            }))
        );
        assert_eq!(
            format!("{captured:?}"),
            format!("{:?}", game.current_characteristics(id).unwrap())
        );
    }
}
