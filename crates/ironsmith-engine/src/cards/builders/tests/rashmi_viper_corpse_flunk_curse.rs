#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn visit_effects(effect: &Effect, visit: &mut impl FnMut(&Effect)) {
    visit(effect);
    effect.visit_child_effects(&mut |child| visit_effects(child, visit));
}

fn all_effects(definition: &CardDefinition) -> Vec<&Effect> {
    let mut roots = Vec::new();
    if let Some(spell) = definition.spell_effect.as_ref() {
        roots.extend(spell.flattened_default_effects());
    }
    for ability in &definition.abilities {
        match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                roots.extend(triggered.effects.flattened_default_effects())
            }
            AbilityKind::Activated(activated) => {
                roots.extend(activated.effects.flattened_default_effects())
            }
            _ => {}
        }
    }
    roots
}

#[test]
fn rashmi_and_ragavan_keep_ordinal_turn_scope_and_one_exiled_card_identity() {
    let definition = parse_oracle_card_definition("Rashmi and Ragavan");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Whenever you cast your first spell during each of your turns, exile the top card of target opponent's library and create a Treasure token. Then you may cast the exiled card without paying its mana cost if it's a spell with mana value less than the number of artifacts you control. If you don't cast it this way, you may cast it this turn."
        ]
    );
    let (triggered, cast_trigger) = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::SpellCastTrigger>()
                .map(|trigger| (triggered, trigger)),
            _ => None,
        })
        .expect("Rashmi and Ragavan should retain a spell-cast trigger");
    assert_eq!(cast_trigger.exact_spells_this_turn, Some(1));
    assert_eq!(cast_trigger.during_turn, Some(PlayerFilter::You));

    let mut cast_tags = Vec::new();
    let mut grant_tags = Vec::new();
    for root in triggered.effects.flattened_default_effects() {
        visit_effects(root, &mut |effect| {
            if let Some(cast) = effect.downcast_ref::<crate::effects::CastTaggedEffect>() {
                cast_tags.push(cast.tag.clone());
            }
            if let Some(grant) = effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>() {
                grant_tags.push(grant.tag.clone());
            }
        });
    }
    assert_eq!(cast_tags.len(), 1, "{:#?}", triggered.effects);
    assert_eq!(
        grant_tags, cast_tags,
        "free cast and fallback must share one exiled card"
    );
}

#[test]
fn corpse_augur_declares_one_player_target_for_both_x_uses() {
    let definition = parse_oracle_card_definition("Corpse Augur");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "When this creature dies, you draw X cards and you lose X life, where X is the number of creature cards in target player's graveyard."
        ]
    );
    let mut target_count = 0;
    let mut values = Vec::new();
    for root in all_effects(&definition) {
        visit_effects(root, &mut |effect| {
            if effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some()
            {
                target_count += 1;
            }
            if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
                values.push(draw.count.clone());
            }
            if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
                values.push(lose.amount.clone());
            }
        });
    }
    assert_eq!(target_count, 1);
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].unhinted(), values[1].unhinted());
}

#[test]
fn flunk_counts_only_cards_in_the_target_creature_controllers_hand() {
    let definition = parse_oracle_card_definition("Flunk");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Target creature gets -X/-X until end of turn, where X is 7 minus the number of cards in that creature's controller's hand."
        ]
    );
    let debug = format!("{:#?}", definition.spell_effect);
    assert!(debug.contains("ControllerOf(\n"), "{debug}");
    assert!(debug.contains("Target"), "{debug}");
    assert!(
        !debug.contains("explicit_card_type_noun: Some(\n"),
        "{debug}"
    );
}

#[test]
fn curse_of_thirst_counts_player_attachments_not_attached_cards() {
    let definition = parse_oracle_card_definition("Curse of Thirst");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Enchant player",
            "At the beginning of enchanted player's upkeep, this Aura deals damage to that player equal to the number of Curses attached to them."
        ]
    );
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("attached_to_player: Some"), "{debug}");
    assert!(debug.contains("Curse"), "{debug}");
    assert!(!debug.contains("attached_to_object: Some"), "{debug}");
}
