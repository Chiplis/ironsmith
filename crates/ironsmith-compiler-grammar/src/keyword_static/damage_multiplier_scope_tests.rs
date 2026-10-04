use super::*;

fn parse(text: &str) -> StaticAbility {
    let tokens = crate::lexer::lex_line(text, 0).unwrap();
    let (result, loss) =
        crate::parse_loss::capture(|| parse_double_damage_amount_replacement_line(&tokens));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    result.unwrap().unwrap()
}

#[test]
fn damage_multiplier_recipients_and_combat_scope_are_typed() {
    for (text, expected) in [
        (
            "If a source would deal damage to this creature, it deals double that damage to this creature instead.",
            ObjectFilter::source(),
        ),
        (
            "If a creature would deal combat damage to a creature, it deals double that damage to that creature instead.",
            ObjectFilter::creature(),
        ),
        (
            "If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.",
            { let mut filter = ObjectFilter::creature(); filter.with_attached_object = Some(Box::new(ObjectFilter::source())); filter },
        ),
    ] {
        let ability = parse(text);
        let ironsmith_core::StaticAbilityPayload::DoubleDamageAmountReplacement {
            source_filter,
            target_player_filter,
            target_object_filter,
            factor,
            combat_only,
            noncombat_only,
            ..
        } = ability.payload
        else {
            panic!("wrong typed payload");
        };
        assert_eq!(target_player_filter, None);
        assert_eq!(target_object_filter, Some(expected));
        assert_eq!(factor, 2);
        assert_eq!(combat_only, text.contains("combat"));
        assert!(!noncombat_only);
        if text.contains("another creature") {
            assert!(!source_filter.other);
            assert_eq!(
                source_filter.without_attached_object.as_deref(),
                Some(&ObjectFilter::source())
            );
        }
    }
    let chosen = parse(
        "If a source would deal damage to the chosen player or a permanent they control, it deals double that damage instead.",
    );
    let ironsmith_core::StaticAbilityPayload::DoubleDamageAmountReplacement {
        target_player_filter,
        target_object_filter,
        ..
    } = chosen.payload
    else {
        panic!();
    };
    assert_eq!(target_player_filter, Some(PlayerFilter::ChosenPlayer));
    assert_eq!(
        target_object_filter.unwrap().controller,
        Some(PlayerFilter::ChosenPlayer)
    );
}

#[test]
fn live_condition_and_noncombat_qualifier_are_not_discarded() {
    let ability = parse(
        "If a source you control would deal noncombat damage to a permanent or player while there are four or more card types among cards in your graveyard, it deals double that damage instead.",
    );
    let ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } = ability.payload else {
        panic!("condition missing");
    };
    assert!(matches!(
        ability.payload,
        ironsmith_core::StaticAbilityPayload::DoubleDamageAmountReplacement {
            noncombat_only: true,
            combat_only: false,
            ..
        }
    ));
}

#[test]
fn damage_replacement_rejects_a_changed_recipient_and_static_duration_loss() {
    for text in [
        "If a source would deal damage to you, it deals double that damage to an opponent instead.",
        "If a source would deal damage to a creature, it deals double that damage to that player instead.",
        "If a source would deal damage to this creature, it deals double that damage to equipped creature instead.",
        "If a source would deal damage to a creature this turn, it deals double that damage to that creature instead.",
        "If a source would deal damage to a creature next turn, it deals double that damage to that creature instead.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        assert!(
            parse_double_damage_amount_replacement_line(&tokens)
                .unwrap()
                .is_none(),
            "{text}"
        );
    }
}

#[test]
fn existing_union_recipient_anaphors_remain_supported() {
    let ability = parse(
        "If a source would deal damage to an opponent or a permanent an opponent controls, it deals double that damage to that player or permanent instead.",
    );
    let ironsmith_core::StaticAbilityPayload::DoubleDamageAmountReplacement {
        target_player_filter,
        target_object_filter,
        ..
    } = ability.payload
    else {
        panic!();
    };
    assert_eq!(target_player_filter, Some(PlayerFilter::Opponent));
    assert_eq!(
        target_object_filter.unwrap().controller,
        Some(PlayerFilter::Opponent)
    );
}
