use super::*;

#[test]
pub(super) fn modal_header_tracks_distinct_player_targets_per_mode() {
    let header = parse_modal_header_for_test(
        "When this creature enters, choose one or both. Each mode must target a different player.",
    )
    .expect("modal header should parse")
    .expect("modal header should be recognized");

    assert_eq!(header.min, Value::Fixed(1));
    assert_eq!(header.max, Some(Value::Fixed(2)));
    assert!(header.distinct_player_targets_per_mode, "{header:#?}");
}

#[test]
pub(super) fn modal_distinct_player_rule_lowers_to_choose_mode_metadata()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Distinct Player Modal Variant")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "When this creature enters, choose one or both. Each mode must target a different player.\n\
         • Target player draws a card.\n\
         • Target player loses 1 life."
            .to_string(),
        false,
    )?;
    let modal = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseModeEffect>()),
            _ => None,
        })
        .expect("trigger should lower to a modal effect");

    assert_eq!(modal.min_choose_count, Value::Fixed(1));
    assert_eq!(modal.choose_count, Value::Fixed(2));
    assert!(modal.distinct_player_targets_per_mode, "{modal:#?}");
    Ok(())
}

#[test]
pub(super) fn chandras_fury_full_card_keeps_player_or_planeswalker_controller_fanout()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Chandra's Fury")
        .card_types(vec![CardType::Instant]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Chandra's Fury deals 4 damage to target player or planeswalker and 1 damage to each creature that player or that planeswalker's controller controls."
            .to_string(),
        false,
    )?;
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("Chandra's Fury should have a spell program")
        .flattened_default_effects();
    let effects = match effects {
        [effect] => effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .filter(|sequence| sequence.surface == ironsmith_core::SequenceSurface::Coordinated)
            .map_or(effects, |sequence| sequence.effects.as_slice()),
        _ => effects,
    };

    let target_damage = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DealDamageEffect>())
        .expect("Chandra's Fury should deal damage to its target");
    assert_eq!(target_damage.amount, Value::Fixed(4));
    let target_is_player_or_planeswalker = match target_damage.target.unhinted() {
        crate::target::ChooseSpec::PlayerOrPlaneswalker(_) => true,
        crate::target::ChooseSpec::Target(inner) => matches!(
            inner.unhinted(),
            crate::target::ChooseSpec::PlayerOrPlaneswalker(_)
        ),
        _ => false,
    };
    assert!(
        target_is_player_or_planeswalker,
        "target damage should retain the player-or-planeswalker target: {target_damage:#?}"
    );

    let fanout = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ForEachObject>())
        .expect("Chandra's Fury should fan out over the chosen player's creatures");
    assert_eq!(fanout.filter.card_types, vec![CardType::Creature]);
    assert_eq!(
        fanout.filter.controller,
        Some(crate::target::PlayerFilter::TargetPlayerOrControllerOfTarget),
        "planeswalker targets must bind the fanout to that planeswalker's controller"
    );
    Ok(())
}

#[test]
pub(super) fn geth_full_card_mills_the_selected_graveyard_cards_owner() -> Result<(), CardTextError>
{
    let builder = CardDefinitionBuilder::new(CardId::new(), "Geth, Lord of the Vault")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)\n{X}{B}: Put target artifact or creature card with mana value X from an opponent's graveyard onto the battlefield under your control tapped. Then that player mills X cards."
            .to_string(),
        false,
    )?;
    let effects = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => {
                Some(activated.effects.flattened_default_effects())
            }
            _ => None,
        })
        .expect("Geth should have an activated ability");
    let mill = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::MillEffect>())
        .expect("Geth's activated ability should mill");

    assert_eq!(mill.count, Value::X);
    assert!(
        matches!(
            &mill.player,
            crate::target::PlayerFilter::AliasedOwnerOf(
                crate::target::ObjectRef::Target | crate::target::ObjectRef::Tagged(_)
            )
        ),
        "the player milled must be the exact owner of the selected graveyard card: {mill:#?}"
    );
    Ok(())
}

#[test]
pub(super) fn red_death_full_card_draws_for_the_goaded_creatures_controller()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Red Death, Shipwrecker")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Alluring Eyes — {T}: Goad target creature an opponent controls. That player draws a card. You add {R}. (Until your next turn, that creature attacks each combat if able and attacks a player other than you if able.)"
            .to_string(),
        false,
    )?;
    let effects = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => {
                Some(activated.effects.flattened_default_effects())
            }
            _ => None,
        })
        .expect("Red Death should have an activated ability");
    let draw = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DrawCardsEffect>())
        .expect("Red Death's activated ability should draw a card");

    assert_eq!(draw.count, Value::Fixed(1));
    assert!(
        matches!(
            &draw.player,
            crate::target::PlayerFilter::AliasedControllerOf(
                crate::target::ObjectRef::Target | crate::target::ObjectRef::Tagged(_)
            )
        ),
        "the exact controller of the goaded creature must draw: {draw:#?}"
    );
    Ok(())
}

#[test]
pub(super) fn steam_vines_full_card_preserves_controller_as_attachment_chooser()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Steam Vines")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Enchant land\nWhen enchanted land becomes tapped, destroy it and this Aura deals 1 damage to that land's controller. That player attaches this Aura to a land of their choice."
            .to_string(),
        false,
    )?;
    let effects = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                Some(triggered.effects.flattened_default_effects())
            }
            _ => None,
        })
        .expect("Steam Vines should have a triggered ability");
    let choose = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        .expect("Steam Vines should ask the damaged land's controller to choose a land");
    let attach = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::AttachObjectsEffect>())
        .expect("Steam Vines should attach to the chosen land");

    assert_eq!(choose.filter.card_types, vec![CardType::Land]);
    assert_eq!(choose.filter.zone, Some(Zone::Battlefield));
    assert!(
        matches!(
            &choose.chooser,
            crate::target::PlayerFilter::AliasedControllerOf(crate::target::ObjectRef::Tagged(_))
        ),
        "the destroyed land's exact controller must make the attachment choice: {choose:#?}"
    );
    assert_eq!(
        attach.target,
        crate::target::ChooseSpec::Tagged(choose.tag.clone()),
        "the Aura must attach to the land chosen by that player"
    );
    Ok(())
}
