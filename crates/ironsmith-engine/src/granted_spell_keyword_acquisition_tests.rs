use crate::ability::Ability;
use crate::cards::builders::CardDefinitionBuilder;
use crate::continuous::{EffectTarget, Modification};
use crate::effects::{ApplyContinuousEffect, EffectExecutor, ExecutionContext};
use crate::game_state::GameState;
use crate::granted_spell_keywords::{acquire_spell_keywords_for_cast, granted_spell_keywords};
use crate::ids::{CardId, PlayerId};
use crate::static_abilities::StaticAbility;
use crate::target::ObjectFilter;
use crate::types::CardType;
use crate::{Until, Zone};
use ironsmith_core::{GrantedSpellKeyword, GrantedSpellKeywordKind};

fn grant(game: &mut GameState, player: PlayerId) -> crate::ids::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Keyword grant fixture")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(StaticAbility::grant_spell_keyword(
            ObjectFilter::instant_or_sorcery().cast_by(crate::target::PlayerFilter::You),
            GrantedSpellKeyword::intrinsic(GrantedSpellKeywordKind::Demonstrate),
            None,
            "Demonstrate",
        )))
        .build();
    game.create_object_from_definition(&definition, player, Zone::Battlefield)
}

fn spell(game: &mut GameState, player: PlayerId) -> crate::ids::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Keyword spell fixture")
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_definition(&definition, player, Zone::Stack)
}

#[test]
fn acquired_instances_survive_the_granter_and_do_not_discover_later_grants() {
    for player in [PlayerId(0), PlayerId(1)] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let original = grant(&mut game, player);
        let proposed = spell(&mut game, player);
        acquire_spell_keywords_for_cast(&mut game, proposed, player);
        let first = granted_spell_keywords(&game, proposed, player, true);
        assert_eq!(first.len(), 1);
        game.move_object_by_game_rule(original, Zone::Graveyard)
            .unwrap();
        assert_eq!(
            granted_spell_keywords(&game, proposed, player, false)[0].identity,
            first[0].identity
        );
        grant(&mut game, player);
        assert_eq!(
            granted_spell_keywords(&game, proposed, player, false).len(),
            1
        );
        acquire_spell_keywords_for_cast(&mut game, proposed, player);
        assert_eq!(
            granted_spell_keywords(&game, proposed, player, false).len(),
            1
        );
        let later = spell(&mut game, player);
        acquire_spell_keywords_for_cast(&mut game, later, player);
        assert_eq!(granted_spell_keywords(&game, later, player, false).len(), 1);
        let departed = game
            .move_object_by_game_rule(proposed, Zone::Graveyard)
            .unwrap();
        assert!(game.acquired_spell_keywords(proposed).is_none());
        assert!(game.acquired_spell_keywords(departed).is_none());
    }
}

#[test]
fn a_new_ability_removal_effect_can_remove_the_gained_keyword_occurrence() {
    let player = PlayerId(0);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = grant(&mut game, player);
    let proposed = spell(&mut game, player);
    acquire_spell_keywords_for_cast(&mut game, proposed, player);
    assert_eq!(
        granted_spell_keywords(&game, proposed, player, false).len(),
        1
    );
    ApplyContinuousEffect::new(
        EffectTarget::Specific(proposed),
        Modification::RemoveAllAbilities,
        Until::Forever,
    )
    .execute(
        &mut game,
        &mut ExecutionContext::new_default(source, player),
    )
    .unwrap();
    assert!(granted_spell_keywords(&game, proposed, player, false).is_empty());
}

#[test]
fn native_acquisition_checkpoints_are_isolated_between_clones() {
    let player = PlayerId(0);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = grant(&mut game, player);
    let proposed = spell(&mut game, player);
    let mut clone = game.clone();
    clone
        .move_object_by_game_rule(source, Zone::Graveyard)
        .unwrap();
    acquire_spell_keywords_for_cast(&mut clone, proposed, player);
    acquire_spell_keywords_for_cast(&mut game, proposed, player);
    assert!(granted_spell_keywords(&clone, proposed, player, false).is_empty());
    assert_eq!(
        granted_spell_keywords(&game, proposed, player, false).len(),
        1
    );
}
