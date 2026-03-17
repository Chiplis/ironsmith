use ironsmith::{
    ability::AbilityKind,
    cards::CardDefinitionBuilder,
    continuous::Modification,
    effect::Value,
    effects::{
        ApplyContinuousEffect, ForEachTaggedEffect, HauntExileEffect, ScheduleDelayedTriggerEffect,
        continuous::RuntimeModification,
    },
    ids::CardId,
    static_abilities::StaticAbilityId,
    target::PlayerFilter,
    types::CardType,
    types::Subtype,
};

#[test]
fn parser_feature_smoke_spell_line_parses() {
    let text = "Destroy target creature.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Parser Smoke")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("parser smoke spell should parse");
    assert!(def.spell_effect.is_some());
}

#[test]
fn parser_feature_smoke_trigger_line_parses() {
    let text = "Whenever this creature deals combat damage to a player, draw a card.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Parser Trigger Smoke")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("parser smoke trigger should parse");
    assert!(!def.abilities.is_empty());
}

#[test]
fn parser_feature_smoke_haunt_linkage_stitches_into_haunt_ability() {
    let text = "Haunt\nWhen this creature enters or the creature it haunts dies, draw a card.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Haunt Smoke")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("haunt smoke should parse");

    let haunt_ability = def
        .abilities
        .iter()
        .find(|ability| ability.text.as_deref() == Some("Haunt"))
        .expect("haunt keyword ability should be present");

    let AbilityKind::Triggered(triggered) = &haunt_ability.kind else {
        panic!("haunt keyword should lower to a triggered ability");
    };
    assert_eq!(triggered.effects.len(), 1);
    assert!(
        triggered.effects[0]
            .downcast_ref::<HauntExileEffect>()
            .is_some(),
        "haunt keyword should carry the delayed haunt exile effect"
    );
}

#[test]
fn parser_feature_smoke_spell_trigger_line_becomes_delayed_effect() {
    let text = "Whenever you cast a creature spell this turn, draw a card.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Delayed Trigger Smoke")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("spell delayed trigger smoke should parse");

    assert!(def.abilities.is_empty());
    let effects = def
        .spell_effect
        .expect("spell should lower to spell effects");
    assert_eq!(effects.len(), 1);
    let delayed = effects[0]
        .downcast_ref::<ScheduleDelayedTriggerEffect>()
        .expect("spell trigger should lower to a delayed trigger effect");
    assert!(delayed.until_end_of_turn);
}

#[test]
fn parser_feature_smoke_channel_parses_life_for_mana_sequence() {
    let text = "Until end of turn, any time you could activate a mana ability, you may pay 1 life. If you do, add {C}.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Channel Smoke")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("channel smoke should parse");
    let rendered = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        rendered.contains("loselifeeffect") && rendered.contains("addmanaeffect"),
        "expected Channel Smoke to lower into a lose-life then add-mana sequence, got {rendered}"
    );
}

#[test]
fn parser_feature_smoke_take_to_the_streets_merges_citizen_bonus() {
    let text = "Creatures you control get +2/+2 until end of turn. Citizens you control get an additional +1/+1 and gain vigilance until end of turn.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Take to the Streets Smoke")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("take to the streets smoke should parse");

    let effects = def
        .spell_effect
        .expect("take to the streets should lower to spell effects");
    let apply = effects[1]
        .downcast_ref::<ApplyContinuousEffect>()
        .expect("citizen rider should lower to a continuous effect");

    let filter = match &apply.target {
        ironsmith::continuous::EffectTarget::Filter(filter) => filter,
        other => panic!("expected filter-targeted continuous effect, got {other:?}"),
    };
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.subtypes.contains(&Subtype::Citizen));
    assert!(apply.modification.as_ref().is_some_and(|modification| {
        matches!(
            modification,
            Modification::AddAbility(ability) if ability.id() == StaticAbilityId::Vigilance
        )
    }));
    assert!(apply.runtime_modifications.iter().any(|modification| {
        matches!(
            modification,
            RuntimeModification::ModifyPowerToughness {
                power: Value::Fixed(1),
                toughness: Value::Fixed(1),
            }
        )
    }));
}

#[test]
fn parser_feature_smoke_chaotic_transformation_uses_generic_for_each_tagged_lowering() {
    let text = "Exile up to one target artifact, up to one target creature, up to one target enchantment, up to one target planeswalker, and/or up to one target land. For each permanent exiled this way, its controller reveals cards from the top of their library until they reveal a card that shares a card type with it, puts that card onto the battlefield, then shuffles.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Chaotic Transformation Smoke")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("chaotic transformation smoke should parse");

    let effects = def
        .spell_effect
        .expect("chaotic transformation should lower to spell effects");
    assert!(
        effects
            .iter()
            .any(|effect| effect.downcast_ref::<ForEachTaggedEffect>().is_some()),
        "chaotic transformation should use generic for-each-tagged lowering"
    );
}
