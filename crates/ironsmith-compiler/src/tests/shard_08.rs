use super::*;

#[test]
pub(super) fn distinct_spell_source_lines_survive_as_resolution_provenance()
-> Result<(), CardTextError> {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Source Line Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile target permanent you own or control.\nDraw a card at the beginning of the next turn's upkeep.",
        )?;
    let program = definition
        .spell_effect
        .as_ref()
        .expect("instant should have a spell program");

    assert!(program.segments.len() >= 2, "{program:#?}");
    assert!(!program.segments[0].starts_new_source_line);
    assert!(
        program
            .segments
            .iter()
            .skip(1)
            .any(|segment| segment.starts_new_source_line),
        "{program:#?}"
    );
    Ok(())
}

#[test]
pub(super) fn undiscovered_paradise_uses_the_live_delayed_untap_route() -> Result<(), CardTextError>
{
    let definition =
        CardDefinitionBuilder::new(CardId::new(), "Undiscovered Paradise")
            .card_types(vec![CardType::Land])
            .parse_text(
                "{T}: Add one mana of any color. During your next untap step, as you untap your permanents, return this land to its owner's hand.",
            )?;
    let debug = format!("{definition:#?}");

    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(debug.contains("AsPermanentsUntap"), "{debug}");
    assert!(debug.contains("ReturnToHandEffect"), "{debug}");
    assert!(
        !debug.contains("UntapEffect"),
        "the timing phrase must not lower as an ordinary untap action: {debug}"
    );
    Ok(())
}

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
pub(super) fn inline_token_creation_or_is_a_resolution_choice_with_two_create_modes()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Token Choice Variant")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "When this creature enters, create a Food token or a Treasure token.".to_string(),
        false,
    )?;
    let choice = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| {
                    super::find_nested_effect::<crate::effects::ChooseModeEffect>(effect)
                }),
            _ => None,
        })
        .expect("trigger should lower to an inline token choice");

    assert_eq!(
        choice.chooser,
        Some(crate::target::PlayerFilter::You),
        "the inline or-choice must be made during resolution"
    );
    assert_eq!(choice.modes.len(), 2);
    assert!(choice.modes.iter().all(|mode| {
        matches!(
            mode.effects.as_slice(),
            [effect] if effect
                .downcast_ref::<crate::effects::CreateTokenEffect>()
                .is_some()
        )
    }));
    Ok(())
}

#[test]
pub(super) fn compound_token_creation_keeps_generic_ward_on_its_own_blueprint()
-> Result<(), CardTextError> {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Compound Ward Token Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a 1/1 white Human creature token with ward {2} and a 4/4 white Alien Rhino creature token.",
        )?;
    let spell = definition
        .spell_effect
        .as_ref()
        .expect("sorcery should have a spell effect");
    let [effect] = spell.flattened_default_effects() else {
        panic!(
            "expected one coordinated token-creation effect, got {:#?}",
            spell.flattened_default_effects()
        );
    };
    let sequence = super::find_nested_effect::<crate::effects::SequenceEffect>(effect)
        .expect("compound creation should lower to a coordinated sequence");
    let creates = sequence
        .effects
        .iter()
        .map(|effect| {
            super::find_nested_effect::<crate::effects::CreateTokenEffect>(effect)
                .expect("each coordinated branch should create one token")
        })
        .collect::<Vec<_>>();
    let [human, alien_rhino] = creates.as_slice() else {
        panic!("expected two token blueprints, got {creates:#?}");
    };

    assert_eq!(human.token.card.name, "Human");
    let ward = human
        .token
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::Ward => Some(ability),
            _ => None,
        })
        .expect("the Human token should retain ward");
    assert_eq!(
        ward.payload,
        StaticAbilityPayload::Ward(crate::cost::TotalCost::mana(
            crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])
        ))
    );
    assert_eq!(alien_rhino.token.card.name, "Alien Rhino");
    assert!(
        alien_rhino.token.abilities.iter().all(|ability| {
            !matches!(
                &ability.kind,
                AbilityKind::Static(ability) if ability.id() == StaticAbilityId::Ward
            )
        }),
        "ward must remain scoped to the first token blueprint"
    );
    Ok(())
}

#[test]
pub(super) fn spry_and_mighty_preserves_exact_choice_and_power_gap_value()
-> Result<(), CardTextError> {
    let oracle = "Choose exactly two creatures you control. You draw X cards and the chosen creatures get +X/+X and gain trample until end of turn, where X is the difference between the chosen creatures' powers.";
    let definition = CardDefinitionBuilder::new(CardId::new(), "Spry and Mighty")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)?;
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("sorcery should have a spell program")
        .flattened_default_effects();

    let choose = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::ChooseObjectsEffect>(effect))
        .expect("the spell should choose its creature pair");
    assert_eq!(choose.count.min, 2);
    assert_eq!(choose.count.max, Some(2));
    assert!(
        choose.count.explicit_exactly,
        "the explicitly authored `exactly` choice surface must survive lowering"
    );
    assert_eq!(
        choose.tag.as_str(),
        crate::cards::builders::CHOSEN_OBJECTS_TAG
    );

    let draw = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::DrawCardsEffect>(effect))
        .expect("the spell should draw the power-gap count");
    assert!(
        draw.count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::Difference),
        "{:#?}",
        draw.count
    );
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("GreatestPower")
            && debug.contains("LeastPower")
            && debug.contains(crate::cards::builders::CHOSEN_OBJECTS_TAG)
            && debug.contains("Trample")
            && debug.contains("ModifyPowerToughness"),
        "the chosen pair must feed draw, pump, and trample: {debug}"
    );
    Ok(())
}

#[test]
pub(super) fn as_enters_opponent_choice_persists_into_static_values_and_triggers()
-> Result<(), CardTextError> {
    let pallimud = CardDefinitionBuilder::new(CardId::new(), "Persistent Choice Value")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "As this creature enters, choose an opponent.\nThis creature's power is equal to the number of tapped lands the chosen player controls.",
        )?;
    let pallimud_debug = format!("{pallimud:#?}");
    assert!(
        pallimud_debug.contains("ChoosePlayerAsEnters")
            && pallimud_debug.contains("filter: Opponent")
            && pallimud_debug.contains("controller: Some(\n")
            && pallimud_debug.contains("ChosenPlayer"),
        "as-enters choice and characteristic value must share persistent chosen-player state: {pallimud_debug}"
    );

    let vise = CardDefinitionBuilder::new(CardId::new(), "Persistent Choice Trigger")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "As this artifact enters, choose an opponent.\nAt the beginning of the chosen player's upkeep, this artifact deals 1 damage to that player.",
        )?;
    let vise_debug = format!("{vise:#?}");
    let vise_compact = format!("{vise:?}");
    assert!(
        vise_debug.contains("ChoosePlayerAsEnters")
            && vise_debug.contains("filter: Opponent")
            && vise_debug.contains("BeginningOfUpkeep {\n")
            && vise_debug.contains("player: ChosenPlayer")
            && !vise_compact.contains("AliasedTarget(ChosenPlayer)"),
        "as-enters choice and possessive trigger must share persistent chosen-player state: {vise_debug}"
    );
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
        .find_map(|effect| super::find_nested_effect::<crate::effects::MillEffect>(effect))
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

#[test]
pub(super) fn aeon_engine_lowers_turn_order_reversal_to_a_typed_effect() -> Result<(), CardTextError>
{
    let definition = CardDefinitionBuilder::new(CardId::new(), "Aeon Engine")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "This artifact enters tapped.\n{T}, Exile this artifact: Reverse the game's turn order. (For example, if play had proceeded clockwise around the table, it now goes counterclockwise.)",
        )?;
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Aeon Engine should have its activated ability");

    assert!(
        activated
            .effects
            .flattened_default_effects()
            .iter()
            .any(|effect| effect
                .downcast_ref::<crate::effects::ReverseTurnOrderEffect>()
                .is_some()),
        "the ability must carry the typed global turn-order transition: {activated:#?}"
    );
    Ok(())
}

#[test]
pub(super) fn fire_magic_lowers_tiered_labels_costs_and_exactly_one_mode()
-> Result<(), CardTextError> {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Fire Magic")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Tiered (Choose one additional cost.)\n\
             • Fire — {0} — Fire Magic deals 1 damage to each creature.\n\
             • Fira — {2} — Fire Magic deals 2 damage to each creature.\n\
             • Firaga — {5} — Fire Magic deals 3 damage to each creature.",
        )?;
    let modal = definition
        .spell_effect
        .as_ref()
        .expect("Fire Magic should have a spell program")
        .all_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseModeEffect>())
        .expect("Tiered should lower to a typed modal effect");

    assert!(modal.spree, "Tiered must use casting-time mode costs");
    assert!(
        modal.tiered,
        "Tiered presentation metadata must be retained"
    );
    assert_eq!(modal.min_choose_count, Value::Fixed(1));
    assert_eq!(modal.choose_count, Value::Fixed(1));
    assert_eq!(modal.modes.len(), 3);
    assert_eq!(
        modal
            .mode_additional_mana_costs
            .iter()
            .map(|cost| cost.to_oracle())
            .collect::<Vec<_>>(),
        vec!["{0}", "{2}", "{5}"]
    );
    assert_eq!(
        modal
            .modes
            .iter()
            .map(|mode| mode.source_text.trim_end_matches('.'))
            .collect::<Vec<_>>(),
        vec![
            "Fire — {0} — Fire Magic deals 1 damage to each creature",
            "Fira — {2} — Fire Magic deals 2 damage to each creature",
            "Firaga — {5} — Fire Magic deals 3 damage to each creature",
        ]
    );
    Ok(())
}
