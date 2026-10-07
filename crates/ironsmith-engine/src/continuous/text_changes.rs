//! Authored-word transformation in layer 3. This is deliberately independent
//! of Oracle strings and rendered labels. Unimplemented model domains return
//! an error through checked characteristic calculation, never a partial edit.

use super::CalculatedCharacteristics;
use crate::ability::{Ability, AbilityKind, ProtectionFrom};
use crate::static_abilities::{CompiledStaticAbility, LandwalkKind, StaticAbility, StaticAbilityId};
use ironsmith_core::{ObjectFilter, TextChange};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextChangeDomainError {
    SpellProgram,
    Value,
    Condition,
    Cost,
    Effect,
    ActivatedAbility,
    TriggeredAbility,
    Attachment,
    StaticAbility(StaticAbilityId),
    ObjectFilter,
    ProtectionReference,
    TokenDefinition,
}

impl std::fmt::Display for TextChangeDomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value => f.write_str("unresolved or ambiguously authored value"),
            Self::Condition => f.write_str("unmodeled condition"),
            Self::Cost => f.write_str("unmodeled cost"),
            Self::Effect => f.write_str("unmodeled effect program"),
            Self::SpellProgram => f.write_str("missing retained spell-program evidence"),
            Self::ActivatedAbility => f.write_str("activated ability program/cost/choice model"),
            Self::TriggeredAbility => f.write_str("triggered ability program/trigger/condition model"),
            Self::Attachment => f.write_str("attachment metadata"),
            Self::StaticAbility(id) => write!(f, "static ability {id:?}"),
            Self::ObjectFilter => f.write_str("object filter outside the typed word domain"),
            Self::ProtectionReference => f.write_str("referenced protection quality"),
            Self::TokenDefinition => f.write_str("missing or incomplete token-definition word roles"),
        }
    }
}
impl std::error::Error for TextChangeDomainError {}

/// Rewrite an authored predicate only when every nontrivial field is within
/// this owner's admitted domain. The residual comparison is typed equality,
/// not a test against display text or an assumption that unknown fields are
/// harmless. New filter fields with nondefault contents remain held.
pub(crate) fn rewrite_filter_words(filter: &ObjectFilter, change: TextChange)
    -> Result<ObjectFilter, TextChangeDomainError>
{
    super::text_change_predicates::rewrite_filter_words(filter, change)
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
        ProtectionFrom::ColorsAmong { filter, reference_source } => ProtectionFrom::ColorsAmong {
            filter: rewrite_filter_words(filter, change)?, reference_source: *reference_source,
        },
        ProtectionFrom::ColorsAmongAtResolution(filter) =>
            ProtectionFrom::ColorsAmongAtResolution(rewrite_filter_words(filter, change)?),
        ProtectionFrom::ColorsOf(spec) => ProtectionFrom::ColorsOf(Box::new(
            super::text_change_predicates::rewrite_choose_spec_words(spec, change)?)),
        // These are rules concepts or runtime choices, not authored color
        // words. In particular, "all colors" doesn't contain five words.
        ProtectionFrom::Colorless | ProtectionFrom::AllColors | ProtectionFrom::Creatures
        | ProtectionFrom::CardType(_) | ProtectionFrom::ChosenPlayer | ProtectionFrom::ChosenColor
        | ProtectionFrom::ColorsOutsideCommanderIdentity | ProtectionFrom::ManaValuesOtherThanChosenNumber
        | ProtectionFrom::Everything | ProtectionFrom::OwnColors => from.clone(),
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

pub(crate) fn rewrite_core_landwalk_words(
    kind: ironsmith_core::LandwalkKind,
    change: TextChange,
) -> ironsmith_core::LandwalkKind {
    match kind {
        ironsmith_core::LandwalkKind::Subtype { mut subtype, snow } => {
            change.replace_subtype_word(&mut subtype);
            ironsmith_core::LandwalkKind::Subtype { subtype, snow }
        }
        ironsmith_core::LandwalkKind::AnyLand
        | ironsmith_core::LandwalkKind::NonbasicLand
        | ironsmith_core::LandwalkKind::ArtifactLand => kind,
    }
}

pub(crate) fn rewrite_attachment_words(filter: &ironsmith_core::AuraAttachmentFilter, change: TextChange)
    -> Result<ironsmith_core::AuraAttachmentFilter, TextChangeDomainError>
{
    Ok(match filter {
        ironsmith_core::AuraAttachmentFilter::Object(filter) =>
            ironsmith_core::AuraAttachmentFilter::Object(rewrite_filter_words(filter, change)?),
        ironsmith_core::AuraAttachmentFilter::Player(player) =>
            ironsmith_core::AuraAttachmentFilter::Player(super::text_change_predicates::rewrite_player_filter_words(player, change)?),
    })
}

pub(crate) fn rewrite_static_model(
    model: &CompiledStaticAbility,
    change: TextChange,
) -> Result<Option<StaticAbility>, TextChangeDomainError> {
    let rewritten = super::text_change_statics::rewrite_static_model_words(model, change)?;
    // No whole-model equality: nested runtime costs and abilities have legacy
    // presentation equality. The immutable static wrapper memoizes this full
    // definition, including every newly materialized child occurrence.
    Ok(Some(StaticAbility::from_model(rewritten)))
}

fn rewrite_ability(ability: &Ability, change: TextChange) -> Result<Ability, TextChangeDomainError> {
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && triggered.trigger.acquired_identity(triggered.effects.retained_trigger_definition()).is_none()
    {
        return Err(TextChangeDomainError::TriggeredAbility);
    }
    super::text_change_programs::rewrite_ability_words(ability, change)
}

pub(crate) fn apply_text_change(
    chars: &mut CalculatedCharacteristics,
    change: TextChange,
    object: &crate::object::Object,
) {
    let result = (|| {
        // Artifact 7 and current native definitions carry authored abilities
        // only. Basic-land mana is supplied later by its own type-line origin.
        let attachment = chars.aura_attach_filter.as_ref()
            .map(|filter| rewrite_attachment_words(filter, change)).transpose()?;
        let program = match &chars.spell_effect {
            crate::snapshot::SpellProgramState::Unavailable if object.zone == crate::zone::Zone::Stack =>
                return Err(TextChangeDomainError::SpellProgram),
            crate::snapshot::SpellProgramState::Present(program) => crate::snapshot::SpellProgramState::Present(
                super::text_change_programs::rewrite_program_words(program, change)?),
            other => other.clone(),
        };
        let mut abilities = chars.abilities.clone();
        abilities.try_map_rules_text(|ability| rewrite_ability(ability, change))?;
        let mut subtypes = chars.subtypes.to_vec();
        change.replace_subtype_words(&mut subtypes);
        chars.abilities = abilities;
        chars.aura_attach_filter = attachment;
        chars.spell_effect = program;
        chars.text_changes.push(change);
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
        filter.ability_markers.push("unmodeled native rule".into());
        assert_eq!(rewrite_filter_words(&filter, TextChange::color(Color::Black, Color::White).unwrap()),
            Err(TextChangeDomainError::ObjectFilter));
    }

    #[test]
    fn held_program_domain_publishes_neither_partial_types_nor_partial_abilities() {
        #[derive(Debug, Clone)]
        struct UnmodeledWords;
        impl crate::effects::EffectExecutor for UnmodeledWords {
            fn execute(&self, _: &mut GameState, _: &mut crate::effects::ExecutionContext)
                -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError>
            { Ok(crate::effect::EffectOutcome::resolved()) }
        }
        let (game, id) = body(vec![Ability::static_ability(StaticAbility::protection(ProtectionFrom::Color(ColorSet::RED))),
            Ability::activated(crate::cost::TotalCost::free(), vec![crate::effect::Effect::new(UnmodeledWords)])], vec![Subtype::Human]);
        let object = game.object(id).unwrap();
        let mut chars = super::super::initial_characteristics(object, game.turn.turn_number);
        apply_text_change(&mut chars, TextChange::creature_type(Subtype::Human, Subtype::Vampire).unwrap(), object);
        assert_eq!(chars.text_change_error, Some(TextChangeDomainError::Effect));
        assert_eq!(chars.subtypes.as_slice(), &[Subtype::Human]);
        assert!(matches!(chars.validate_numeric_range(), Err(crate::static_ability_processor::StaticEffectDiscoveryError::TextChangeDomain(_))));
    }
}
