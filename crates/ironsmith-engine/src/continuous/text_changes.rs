//! Authored-word transformation in layer 3. This is deliberately independent
//! of Oracle strings and rendered labels. Unimplemented model domains return
//! an error through checked characteristic calculation, never a partial edit.

use super::CalculatedCharacteristics;
use crate::ability::{Ability, AbilityKind, ProtectionFrom};
use crate::static_abilities::{CompiledStaticAbility, LandwalkKind, StaticAbility, StaticAbilityId};
use ironsmith_core::{ObjectFilter, StaticAbilityPayload, TextChange};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextChangeDomainError {
    SpellProgram,
    ActivatedAbility,
    TriggeredAbility,
    Attachment,
    StaticAbility(StaticAbilityId),
    ObjectFilter,
    ProtectionReference,
}

impl std::fmt::Display for TextChangeDomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpellProgram => f.write_str("spell program and stack-to-permanent transfer"),
            Self::ActivatedAbility => f.write_str("activated ability program/cost/choice model"),
            Self::TriggeredAbility => f.write_str("triggered ability program/trigger/condition model"),
            Self::Attachment => f.write_str("attachment metadata"),
            Self::StaticAbility(id) => write!(f, "static ability {id:?}"),
            Self::ObjectFilter => f.write_str("object filter outside the typed word domain"),
            Self::ProtectionReference => f.write_str("referenced protection quality"),
        }
    }
}
impl std::error::Error for TextChangeDomainError {}

/// Rewrite an authored predicate only when every nontrivial field is within
/// this owner's admitted domain. The residual comparison is typed equality,
/// not a test against display text or an assumption that unknown fields are
/// harmless. New filter fields with nondefault contents remain held.
pub(crate) fn rewrite_filter_words(
    filter: &ObjectFilter,
    change: TextChange,
) -> Result<ObjectFilter, TextChangeDomainError> {
    let mut residual = filter.clone();
    let empty = ObjectFilter::default();
    macro_rules! admit_fields {
        ($($field:ident),* $(,)?) => { $(residual.$field = empty.$field.clone();)* };
    }
    admit_fields!(
        zone, source, other, token, nontoken, tapped, untapped,
        card_types, all_card_types, excluded_card_types, type_or_subtype_union,
        subtypes, all_subtypes, excluded_subtypes, supertypes, excluded_supertypes,
        colors, required_colors, excluded_colors, colorless, multicolored, monocolored,
        chosen_color, chosen_land_type, chosen_creature_type, excluded_chosen_creature_type,
        has_basic_land_type, has_nonbasic_land_type,
        name, name_surface, excluded_name, excluded_name_surface,
        exact_mana_cost, has_mana_cost, has_phyrexian_mana_symbol, no_x_in_cost, has_x_in_cost,
        any_of, union_surface,
    );
    // The noun is presentation provenance, and may contain no behavioral
    // substitute for one of the checked subtype predicates above.
    residual.set_explicit_card_type_noun(None);
    if residual != empty { return Err(TextChangeDomainError::ObjectFilter); }
    let mut rewritten = filter.clone();
    for words in [&mut rewritten.colors, &mut rewritten.required_colors] {
        if let Some(words) = words { change.replace_color_words(words); }
    }
    change.replace_color_words(&mut rewritten.excluded_colors);
    change.replace_subtype_words(&mut rewritten.subtypes);
    change.replace_subtype_words(&mut rewritten.all_subtypes);
    change.replace_subtype_words(&mut rewritten.excluded_subtypes);
    rewritten.any_of = filter.any_of.iter()
        .map(|inner| rewrite_filter_words(inner, change)).collect::<Result<_, _>>()?;
    Ok(rewritten)
}

pub(crate) fn rewrite_protection_words(
    from: &ProtectionFrom,
    change: TextChange,
) -> Result<ProtectionFrom, TextChangeDomainError> {
    Ok(match from {
        ProtectionFrom::Color(words) => {
            let mut words = *words;
            change.replace_color_words(&mut words);
            ProtectionFrom::Color(words)
        }
        ProtectionFrom::Permanents(filter) =>
            ProtectionFrom::Permanents(rewrite_filter_words(filter, change)?),
        ProtectionFrom::EachManaValueAmong(filter) =>
            ProtectionFrom::EachManaValueAmong(rewrite_filter_words(filter, change)?),
        ProtectionFrom::ColorsOf(_) => return Err(TextChangeDomainError::ProtectionReference),
        // These are rules concepts or runtime choices, not authored color
        // words. In particular, "all colors" doesn't contain five words.
        ProtectionFrom::Colorless | ProtectionFrom::AllColors | ProtectionFrom::Creatures
        | ProtectionFrom::CardType(_) | ProtectionFrom::ChosenPlayer | ProtectionFrom::ChosenColor
        | ProtectionFrom::ColorsOutsideCommanderIdentity | ProtectionFrom::ManaValuesOtherThanChosenNumber
        | ProtectionFrom::Everything => from.clone(),
    })
}

pub(crate) fn rewrite_landwalk_words(kind: LandwalkKind, change: TextChange) -> LandwalkKind {
    match kind {
        LandwalkKind::Subtype { mut subtype, snow } => {
            change.replace_subtype_word(&mut subtype);
            LandwalkKind::Subtype { subtype, snow }
        }
        LandwalkKind::AnyLand | LandwalkKind::NonbasicLand | LandwalkKind::ArtifactLand => kind,
    }
}

fn wordless_keyword(id: Option<StaticAbilityId>) -> bool {
    matches!(id, Some(
        StaticAbilityId::Flying | StaticAbilityId::FirstStrike | StaticAbilityId::DoubleStrike
        | StaticAbilityId::Deathtouch | StaticAbilityId::Defender | StaticAbilityId::Flash
        | StaticAbilityId::Haste | StaticAbilityId::Hexproof | StaticAbilityId::Indestructible
        | StaticAbilityId::Intimidate | StaticAbilityId::Lifelink | StaticAbilityId::Menace
        | StaticAbilityId::Banding | StaticAbilityId::Reach | StaticAbilityId::Shroud
        | StaticAbilityId::Trample | StaticAbilityId::Vigilance | StaticAbilityId::Fear
        | StaticAbilityId::Skulk | StaticAbilityId::Wither | StaticAbilityId::Infect
        | StaticAbilityId::Changeling | StaticAbilityId::Phasing
    ))
}

pub(crate) fn rewrite_static_model(
    model: &CompiledStaticAbility,
    change: TextChange,
) -> Result<Option<StaticAbility>, TextChangeDomainError> {
    let mut rewritten = model.clone();
    match &mut rewritten.payload {
        StaticAbilityPayload::None | StaticAbilityPayload::SelfSubjectSurface { .. }
            if wordless_keyword(model.id) => return Ok(None),
        StaticAbilityPayload::SourceLineKeywordGroup { .. }
        | StaticAbilityPayload::SourceLineStaticGroup { .. } => return Ok(None),
        StaticAbilityPayload::Protection(from) => {
            *from = rewrite_protection_words(from, change)?;
            if let StaticAbilityPayload::Protection(original) = &model.payload {
                if from == original { return Ok(None); }
            }
        }
        StaticAbilityPayload::Landwalk(kind) => {
            let previous = *kind;
            *kind = rewrite_landwalk_words(*kind, change);
            if previous == *kind { return Ok(None); }
        }
        StaticAbilityPayload::HexproofFrom(filter) => {
            *filter = rewrite_filter_words(filter, change)?;
            if let StaticAbilityPayload::HexproofFrom(original) = &model.payload {
                if filter == original { return Ok(None); }
            }
        }
        _ => return Err(TextChangeDomainError::StaticAbility(
            model.id.unwrap_or(StaticAbilityId::RuleFallbackText))),
    }
    Ok(Some(StaticAbility::from_model(rewritten)))
}

fn rewrite_ability(ability: &Ability, change: TextChange) -> Result<Ability, TextChangeDomainError> {
    let mut rewritten = ability.clone();
    match &ability.kind {
        AbilityKind::Static(ability) => rewritten.kind = AbilityKind::Static(ability.with_text_change(change)?),
        AbilityKind::Activated(_) => return Err(TextChangeDomainError::ActivatedAbility),
        AbilityKind::Triggered(_) => return Err(TextChangeDomainError::TriggeredAbility),
    }
    Ok(rewritten)
}

pub(crate) fn apply_text_change(
    chars: &mut CalculatedCharacteristics,
    change: TextChange,
    object: &crate::object::Object,
) {
    let result = (|| {
        // CalculatedCharacteristics doesn't yet own the current spell program.
        // Never change its type line while leaving its instructions unchanged.
        if object.zone == crate::zone::Zone::Stack {
            return Err(TextChangeDomainError::SpellProgram);
        }
        if chars.aura_attach_filter.is_some() {
            return Err(TextChangeDomainError::Attachment);
        }
        let mut abilities = chars.abilities.clone();
        abilities.try_map_rules_text(|ability| rewrite_ability(ability, change))?;
        let mut subtypes = chars.subtypes.to_vec();
        change.replace_subtype_words(&mut subtypes);
        chars.abilities = abilities;
        chars.subtypes = subtypes.into();
        chars.static_abilities = crate::ability::extract_static_abilities(&chars.abilities).into();
        Ok::<_, TextChangeDomainError>(())
    })();
    if let Err(error) = result { chars.text_change_error = Some(error); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::effect::Until;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;
    use ironsmith_core::{Color, ColorSet};

    fn body(abilities: Vec<Ability>, subtypes: Vec<Subtype>) -> (GameState, ObjectId) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let card = CardBuilder::new(CardId::new(), "Black Human")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
            .power_toughness(PowerToughness::fixed(2, 3)).build();
        let id = game.create_object_from_card(&card, PlayerId::from_index(0), Zone::Battlefield);
        let object = game.object_mut(id).unwrap();
        object.abilities = abilities.into();
        object.subtypes = subtypes.into();
        (game, id)
    }
    fn add(game: &mut GameState, id: ObjectId, change: TextChange, duration: Until) {
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(
            id, PlayerId::from_index(0), vec![id], Modification::RewriteText(change)).until(duration));
    }
    fn protection(chars: &CalculatedCharacteristics) -> Vec<ProtectionFrom> {
        chars.static_abilities.iter().filter_map(|ability| ability.protection_from().cloned()).collect()
    }

    #[test]
    fn native_word_replacement_changes_protection_and_type_line_only() {
        let original = StaticAbility::protection(ProtectionFrom::Color(ColorSet::BLACK));
        let original_id = original.instance_id();
        let (mut game, id) = body(vec![Ability::static_ability(original.clone()),
            Ability::static_ability(StaticAbility::fear())], vec![Subtype::Human]);
        add(&mut game, id, TextChange::color(Color::Black, Color::Blue).unwrap(), Until::Forever);
        add(&mut game, id, TextChange::creature_type(Subtype::Human, Subtype::Vampire).unwrap(), Until::Forever);
        game.refresh_continuous_state().unwrap();
        let chars = game.calculated_characteristics(id).unwrap();
        assert_eq!(protection(&chars), vec![ProtectionFrom::Color(ColorSet::BLUE)]);
        assert_eq!(chars.static_abilities[0].instance_id(), original_id);
        assert_eq!(chars.abilities.origin(0), Some(&crate::continuous::AbilityOrigin::Printed(0)));
        assert_eq!(chars.subtypes.as_slice(), &[Subtype::Vampire]);
        assert_eq!(chars.name.as_ref(), "Black Human");
        assert_eq!(chars.colors, ColorSet::BLACK);
        assert_eq!(chars.mana_cost, game.object(id).unwrap().mana_cost_owned());
        assert_eq!((chars.power, chars.toughness), (Some(2), Some(3)));
        assert!(chars.static_abilities.iter().any(|ability| ability.id() == StaticAbilityId::Fear));
        assert_eq!(original.protection_from(), Some(&ProtectionFrom::Color(ColorSet::BLACK)),
            "previous immutable captures keep their original text");
    }

    #[test]
    fn acquired_grants_are_excluded_and_expiry_reveals_the_original() {
        let original = StaticAbility::protection(ProtectionFrom::Color(ColorSet::RED));
        let (mut game, id) = body(vec![Ability::static_ability(original.clone())], vec![Subtype::Human]);
        let granted = StaticAbility::protection(ProtectionFrom::Color(ColorSet::RED));
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::grant_ability(
            id, PlayerId::from_index(0), id, granted.clone(), Until::Forever));
        add(&mut game, id, TextChange::color(Color::Red, Color::Green).unwrap(), Until::EndOfTurn);
        game.refresh_continuous_state().unwrap();
        let chars = game.calculated_characteristics(id).unwrap();
        assert_eq!(protection(&chars), vec![ProtectionFrom::Color(ColorSet::GREEN),
            ProtectionFrom::Color(ColorSet::RED)]);
        assert_eq!(chars.static_abilities[0].instance_id(), original.instance_id());
        assert_eq!(chars.static_abilities[1].instance_id(), granted.instance_id());
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state().unwrap();
        let chars = game.calculated_characteristics(id).unwrap();
        assert_eq!(protection(&chars), vec![ProtectionFrom::Color(ColorSet::RED),
            ProtectionFrom::Color(ColorSet::RED)]);
    }

    #[test]
    fn layer_three_changes_copied_text_but_is_not_itself_copiable() {
        let (mut game, id) = body(vec![Ability::static_ability(StaticAbility::landwalk(Subtype::Island))], vec![Subtype::Human]);
        let values = crate::snapshot::CopiableValues::from_object(game.object(id).unwrap());
        let receiver = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Receiver")
            .card_types(vec![CardType::Creature]).build(), PlayerId::from_index(0), Zone::Battlefield);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(id, PlayerId::from_index(0),
            EffectTarget::Specific(receiver), Modification::CopyOf {
                target_id: id, copiable_values: Box::new(values), preserve_source_abilities: false,
                name_override: None, name_override_surface: None, add_supertypes: vec![],
            }));
        add(&mut game, receiver, TextChange::basic_land_type(Subtype::Island, Subtype::Swamp).unwrap(), Until::Forever);
        game.refresh_continuous_state().unwrap();
        let chars = game.calculated_characteristics(receiver).unwrap();
        assert_eq!(chars.static_abilities[0].landwalk_kind(), Some(LandwalkKind::Subtype { subtype: Subtype::Swamp, snow: false }));
        let copy = crate::continuous::copiable_values_with_effects(receiver, game.objects_map(),
            game.effect_store.continuous_effects.effects(), &game.battlefield, game.commander_objects(), &game).unwrap();
        let AbilityKind::Static(ability) = &copy.abilities[0].kind else { panic!("static"); };
        assert_eq!(ability.landwalk_kind(), Some(LandwalkKind::Subtype { subtype: Subtype::Island, snow: false }));
    }

    #[test]
    fn predicate_negation_names_and_symbols_are_typed_and_unknown_filters_are_held() {
        let mut filter = ObjectFilter::default();
        filter.excluded_colors = ColorSet::BLACK;
        filter.name = Some("Black Knight".into());
        filter.exact_mana_cost = Some(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]));
        let changed = rewrite_filter_words(&filter, TextChange::color(Color::Black, Color::White).unwrap()).unwrap();
        assert_eq!(changed.excluded_colors, ColorSet::WHITE);
        assert_eq!(changed.name, filter.name);
        assert_eq!(changed.exact_mana_cost, filter.exact_mana_cost);
        filter.controller_controls = Some(Box::new(ObjectFilter::default()));
        assert_eq!(rewrite_filter_words(&filter, TextChange::color(Color::Black, Color::White).unwrap()),
            Err(TextChangeDomainError::ObjectFilter));
    }

    #[test]
    fn held_program_domain_publishes_neither_partial_types_nor_partial_abilities() {
        let (game, id) = body(vec![Ability::static_ability(StaticAbility::protection(ProtectionFrom::Color(ColorSet::RED))),
            Ability::activated(crate::cost::TotalCost::free(), vec![crate::effect::Effect::draw(1)])], vec![Subtype::Human]);
        let object = game.object(id).unwrap();
        let mut chars = super::super::initial_characteristics(object, game.turn.turn_number);
        apply_text_change(&mut chars, TextChange::creature_type(Subtype::Human, Subtype::Vampire).unwrap(), object);
        assert_eq!(chars.text_change_error, Some(TextChangeDomainError::ActivatedAbility));
        assert_eq!(chars.subtypes.as_slice(), &[Subtype::Human]);
        assert!(matches!(chars.validate_numeric_range(), Err(crate::static_ability_processor::StaticEffectDiscoveryError::TextChangeDomain(_))));
    }
}
