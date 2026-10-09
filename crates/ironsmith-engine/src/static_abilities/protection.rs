//! Protection-related static abilities.
//!
//! This includes Protection, Ward, and conditional Hexproof.

use super::{StaticAbilityId, StaticAbilityKind, text_utils::join_with_and};
use crate::ability::ProtectionFrom;
use crate::color::Color;
use crate::cost::TotalCost;
use crate::target::ObjectFilter;

/// Protection from [quality].
///
/// A creature with protection from [quality] can't be:
/// - Damaged by sources with that quality
/// - Enchanted/equipped by permanents with that quality
/// - Blocked by creatures with that quality
/// - Targeted by spells/abilities with that quality
#[derive(Debug, Clone, PartialEq)]
pub struct Protection {
    pub from: ProtectionFrom,
}

impl Protection {
    pub fn new(from: ProtectionFrom) -> Self {
        Self { from }
    }

    pub fn from_color(color: crate::color::Color) -> Self {
        Self::new(ProtectionFrom::Color(color.into()))
    }

    pub fn from_all_colors() -> Self {
        Self::new(ProtectionFrom::AllColors)
    }

    pub fn from_everything() -> Self {
        Self::new(ProtectionFrom::Everything)
    }

    pub fn from_card_type(card_type: crate::types::CardType) -> Self {
        Self::new(ProtectionFrom::CardType(card_type))
    }
}

fn describe_colors_reference(spec: &crate::target::ChooseSpec) -> String {
    use crate::target::ChooseSpec;
    match spec.base() {
        ChooseSpec::Target(inner) => match inner.base() {
            ChooseSpec::Object(filter) => format!("target {}", filter.description()),
            _ => "target permanent".to_string(),
        },
        ChooseSpec::Object(filter) => filter.description(),
        ChooseSpec::Source => "this permanent".to_string(),
        _ => "that permanent".to_string(),
    }
}

fn describe_color_set(colors: crate::color::ColorSet) -> String {
    let mut names = Vec::new();
    if colors.contains(Color::White) {
        names.push("white");
    }
    if colors.contains(Color::Blue) {
        names.push("blue");
    }
    if colors.contains(Color::Black) {
        names.push("black");
    }
    if colors.contains(Color::Red) {
        names.push("red");
    }
    if colors.contains(Color::Green) {
        names.push("green");
    }
    join_with_and(&names)
}

impl StaticAbilityKind for Protection {
    fn canonical_model(&self) -> Option<super::CompiledStaticAbility> {
        Some(super::CompiledStaticAbility::protection(self.from.clone()))
    }

    fn rewrite_text_words(&self, change: ironsmith_core::TextChange)
        -> Result<Option<super::StaticAbility>, crate::continuous::text_changes::TextChangeDomainError>
    {
        let from = crate::continuous::text_changes::rewrite_protection_words(&self.from, change)?;
        Ok((from != self.from).then(|| super::StaticAbility::new(Self { from })))
    }

    // Protection is queried directly by targeting, blocking, attachment and
    // damage prevention. It does not emit continuous effects.
    fn may_generate_continuous_effects(&self) -> bool {
        false
    }

    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Protection
    }

    fn display(&self) -> String {
        match &self.from {
            ProtectionFrom::Color(colors) => {
                let described = describe_color_set(*colors);
                if described.is_empty() {
                    "Protection from colorless".to_string()
                } else {
                    format!("Protection from {}", described)
                }
            }
            ProtectionFrom::OwnColors => "Protection from each of its colors".to_string(),
            ProtectionFrom::ColorsAmong { filter, .. } | ProtectionFrom::ColorsAmongAtResolution(filter) => format!(
                "Protection from each color among {}", describe_protection_mana_value_scope(filter)),
            ProtectionFrom::AllColors => "Protection from all colors".to_string(),
            ProtectionFrom::Colorless => "Protection from colorless".to_string(),
            ProtectionFrom::Everything => "Protection from everything".to_string(),
            ProtectionFrom::ChosenPlayer => "Protection from the chosen player".to_string(),
            ProtectionFrom::ChosenColor => "Protection from the chosen color".to_string(),
            ProtectionFrom::ColorsOutsideCommanderIdentity => {
                "Protection from each color that's not in your commander's color identity"
                    .to_string()
            }
            ProtectionFrom::ColorsOf(spec) => {
                format!("Protection from the colors of {}", describe_colors_reference(spec))
            }
            ProtectionFrom::CardType(ct) => format!("Protection from {}", ct.plural_name()),
            ProtectionFrom::Creatures => "Protection from creatures".to_string(),
            ProtectionFrom::Permanents(filter) => {
                let description = describe_protection_permanent_filter(filter);
                let description = if matches!(filter.zone, Some(crate::zone::Zone::Stack))
                    && filter.stack_kind == Some(crate::filter::StackObjectKind::Spell)
                    && !description.ends_with("spells")
                {
                    description
                        .strip_suffix(" spell")
                        .map(|prefix| format!("{prefix} spells"))
                        .unwrap_or(description)
                } else {
                    description
                };
                format!("Protection from {description}")
            }
            ProtectionFrom::EachManaValueAmong(filter) => format!(
                "Protection from each mana value among {}",
                describe_protection_mana_value_scope(filter)
            ),
            ProtectionFrom::ManaValuesOtherThanChosenNumber => {
                "Protection from each mana value other than the chosen number".to_string()
            }
        }
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn generate_replacement_effect(
        &self,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Option<crate::replacement::ReplacementEffect> {
        Some(crate::replacement::ReplacementEffect::with_matcher(
            source,
            controller,
            ProtectionDamageMatcher(self.from.clone()),
            crate::replacement::ReplacementAction::PreventDamage,
        ))
    }

    fn has_protection(&self) -> bool {
        true
    }

    fn protection_from(&self) -> Option<&ProtectionFrom> {
        Some(&self.from)
    }

    /// "Protection from the colors of target permanent" locks in the
    /// referenced object's colors as the granting instruction resolves; later
    /// color changes of that object don't change the protection.
    fn materialize_resolution_values(
        &self,
        game: &crate::game_state::GameState,
        ctx: &mut crate::effects::ExecutionContext<'_>,
    ) -> Result<Option<super::StaticAbility>, crate::effects::ExecutionError> {
        // "Planeswalkers you control gain protection from that player": the
        // player is the one this resolution named (CR 702.16k, 611.2c).
        if let ProtectionFrom::Permanents(filter) = &self.from {
            let bound = crate::effects::player_reference_binding::bind_filter_player_references(
                filter, game, ctx,
            );
            return Ok((bound != *filter)
                .then(|| super::StaticAbility::protection(ProtectionFrom::Permanents(bound))));
        }
        if let ProtectionFrom::ColorsAmongAtResolution(filter) = &self.from {
            let context = ctx.filter_context(game);
            let mut colors = crate::color::ColorSet::new();
            for &id in &game.battlefield {
                if game.is_phased_out(id) { continue; }
                let Some(chars) = game.try_current_characteristics(id)
                    .map_err(crate::effects::ExecutionError::ContinuousDiscovery)? else { continue; };
                if game.object(id).is_some_and(|object| {
                    use crate::filter::ObjectFilterExt as _;
                    filter.matches(object, &context, game)
                }) { colors = colors.union(chars.colors); }
            }
            return Ok(Some(super::StaticAbility::protection(ProtectionFrom::Color(colors))));
        }
        let ProtectionFrom::ColorsOf(spec) = &self.from else {
            return Ok(None);
        };
        let mut colors = crate::color::ColorSet::new();
        let objects =
            crate::effects::helpers::resolve_objects_from_spec(game, spec, ctx).unwrap_or_default();
        if objects.is_empty() {
            // A reference to an object that has left its zone reads its
            // last known colors.
            if let crate::target::ChooseSpec::Tagged(tag) = spec.base()
                && let Some(snapshots) = ctx.get_tagged_all(tag.as_str())
            {
                for snapshot in snapshots {
                    colors = colors.union(snapshot.colors);
                }
            }
        }
        for id in objects {
            if let Some(object_colors) = game.current_colors(id) {
                colors = colors.union(object_colors);
            } else if let Some(object) = game.object(id) {
                colors = colors.union(object.colors());
            }
        }
        Ok(Some(super::StaticAbility::protection(ProtectionFrom::Color(
            colors,
        ))))
    }
}

#[derive(Debug, Clone)]
struct ProtectionDamageMatcher(ProtectionFrom);

impl crate::events::traits::ReplacementMatcher for ProtectionDamageMatcher {
    fn may_match_event_kind(&self, kind: crate::events::EventKind) -> bool {
        kind == crate::events::EventKind::Damage
    }

    fn matches_prepared_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::context::PreparedEventContext,
    ) -> bool {
        let Some(damage) = crate::events::downcast_event::<crate::events::DamageEvent>(event)
        else {
            return false;
        };
        let crate::events::DamageTarget::Object(target) = damage.target else {
            return false;
        };
        if ctx.source != Some(target) || damage.amount == 0 {
            return false;
        }
        let subject = if let Some(object) = ctx.game.object(damage.source)
            && !ctx.game.is_phased_out(damage.source)
        {
            crate::filter::ObjectSubject::Live(object)
        } else if let Some(snapshot) = ctx
            .event_source_snapshot
            .filter(|snapshot| snapshot.object_id == damage.source)
        {
            crate::filter::ObjectSubject::Snapshot(snapshot)
        } else {
            // Every source-dependent quality needs actual characteristics.
            // An absent object with no retained snapshot is incomplete
            // evidence, never permission to damage through protection.
            let needs_evidence = match &self.0 {
                ProtectionFrom::Everything => return true,
                ProtectionFrom::Color(colors) => !colors.is_empty(),
                ProtectionFrom::ColorsOf(_) | ProtectionFrom::ColorsAmongAtResolution(_) => false,
                ProtectionFrom::ChosenColor => ctx.game.chosen_color(target).is_some(),
                ProtectionFrom::ChosenPlayer => ctx.game.chosen_player(target).is_some(),
                ProtectionFrom::Permanents(filter) if filter.mana_value_parity.is_some() => {
                    use crate::filter::ParityRequirementRuntimeExt as _;
                    filter.mana_value_parity.and_then(|parity| parity.resolve(ctx.game, Some(target))).is_some()
                }
                _ => true,
            };
            if needs_evidence {
                ctx.game.record_token_resource_failure(&crate::effects::ExecutionError::IncompleteEvidence(
                    "protection requires the exact damage source or its last-known snapshot".into(),
                ));
            }
            return false;
        };
        let view = crate::derived_view::DerivedGameView::new(ctx.game);
        crate::targeting::protection_from_subject_with_view(
            ctx.game, target, subject, &self.0, &view,
        )
    }
    fn display(&self) -> String {
        "Prevent damage from sources matching protection".into()
    }
}

pub(crate) fn describe_protection_mana_value_scope(filter: &ObjectFilter) -> String {
    let description = filter.description();
    pluralize_leading_subject(&description).unwrap_or(description)
}

fn pluralize_leading_subject(description: &str) -> Option<String> {
    let description = description
        .strip_prefix("an ")
        .or_else(|| description.strip_prefix("a "))
        .unwrap_or(description);
    for (singular, plural) in [
        ("artifact", "artifacts"),
        ("creature", "creatures"),
        ("enchantment", "enchantments"),
        ("land", "lands"),
        ("permanent", "permanents"),
        ("spell", "spells"),
    ] {
        if description == singular {
            return Some(plural.to_string());
        }
        if let Some(rest) = description.strip_prefix(&format!("{singular} ")) {
            return Some(format!("{plural} {rest}"));
        }
    }
    None
}

fn describe_protection_permanent_filter(filter: &ObjectFilter) -> String {
    if let Some(quality) = filter.protection_mana_value_parity_quality() {
        return quality.to_string();
    }
    if *filter == ObjectFilter::spell() {
        return "spells".to_string();
    }
    if let Some(chosen) = filter.protection_chosen_card_type_quality() {
        return chosen.to_string();
    }
    // "protection from each of the exiled card's card types" (Mirror Golem):
    // any source sharing a card type with the card this source exiled.
    if let [constraint] = filter.tagged_constraints.as_slice()
        && constraint.relation == crate::filter::TaggedOpbjectRelation::SharesCardType
        && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        && (ObjectFilter {
            tagged_constraints: Vec::new(),
            ..filter.clone()
        }) == ObjectFilter::default()
    {
        return "each of the exiled card's card types".to_string();
    }
    if *filter == ObjectFilter::default().monocolored() {
        return "monocolored".to_string();
    }
    if *filter == ObjectFilter::default().with_supertype(crate::types::Supertype::Snow) {
        return "snow".to_string();
    }
    if *filter == ObjectFilter::default().multicolored() {
        return "multicolored".to_string();
    }
    let mut cast_this_turn_permanent_filter = ObjectFilter::permanent();
    cast_this_turn_permanent_filter.cast_this_turn = true;
    if *filter == cast_this_turn_permanent_filter {
        return "permanents that were cast this turn".to_string();
    }
    if filter.mana_value.is_some() {
        let mut without_mana_value = filter.clone();
        without_mana_value.mana_value = None;
        if without_mana_value == ObjectFilter::default() {
            let description = filter.description();
            if let Some(stripped) = description.strip_prefix("permanent with ") {
                return stripped.to_string();
            }
        }
    }

    if filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.len() == 1
        && filter.excluded_card_types.is_empty()
        && filter.excluded_subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.excluded_supertypes.is_empty()
        && filter.colors.is_none()
        && filter.excluded_colors.is_empty()
        && filter.controller.is_none()
        && filter.owner.is_none()
        && filter.zone.is_none()
        && !filter.token
        && !filter.nontoken
    {
        return pluralize_subtype_for_protection(filter.subtypes[0]);
    }

    let description = filter.description();
    if let Some(pluralized) = pluralize_permanent_counter_phrase(description.as_str()) {
        return pluralized;
    }
    description
}

fn pluralize_permanent_counter_phrase(description: &str) -> Option<String> {
    if let Some(rest) = description.strip_prefix("permanent with a ")
        && let Some(counter_name) = rest.strip_suffix(" counter on it")
    {
        return Some(format!("permanents with {counter_name} counters on them"));
    }
    if let Some(rest) = description.strip_prefix("permanent with an ")
        && let Some(counter_name) = rest.strip_suffix(" counter on it")
    {
        return Some(format!("permanents with {counter_name} counters on them"));
    }
    if description == "permanent with counter on it" {
        return Some("permanents with counters on them".to_string());
    }
    None
}

fn pluralize_subtype_for_protection(subtype: crate::types::Subtype) -> String {
    let name = subtype.to_string();
    match name.as_str() {
        "Elf" => "Elves".to_string(),
        "Dwarf" => "Dwarves".to_string(),
        "Human" => "Humans".to_string(),
        "Wolf" => "Wolves".to_string(),
        "Zombie" => "Zombies".to_string(),
        _ if name.ends_with('s') => name,
        _ => format!("{name}s"),
    }
}

/// Hexproof from [quality].
///
/// A creature with hexproof from [quality] can't be the target of spells
/// or abilities controlled by opponents that have that quality.
#[derive(Debug, Clone, PartialEq)]
pub struct HexproofFrom {
    pub filter: ObjectFilter,
}

impl HexproofFrom {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

fn describe_hexproof_from_filter(filter: &ObjectFilter) -> String {
    if hexproof_from_activated_and_triggered_abilities(filter) {
        return "activated and triggered abilities".to_string();
    }
    if !filter.any_of.is_empty() {
        return filter
            .any_of
            .iter()
            .map(describe_hexproof_from_filter)
            .collect::<Vec<_>>()
            .join(" or ");
    }

    if is_exactly_all_magic_colors_filter(filter) {
        return "each color".to_string();
    }
    // "protection from the chosen card type" (Serra's Emissary).
    if filter.chosen_card_type && {
        let mut chosen_only = ObjectFilter::default();
        chosen_only.chosen_card_type = true;
        *filter == chosen_only
    } {
        return "the chosen card type".to_string();
    }
    // "hexproof from that color" (Skrelv, Defector Mite): the chosen color.
    if filter.chosen_color && {
        let mut chosen_only = ObjectFilter::default();
        chosen_only.chosen_color = true;
        *filter == chosen_only
    } {
        return "the chosen color".to_string();
    }

    let description = filter.description();
    let fragment = description
        .strip_suffix(" permanent")
        .or_else(|| description.strip_suffix(" spell"))
        .or_else(|| description.strip_suffix(" source"))
        .unwrap_or(description.as_str());
    // A bare type noun reads as a class: "hexproof from planeswalkers".
    if filter.card_types.len() == 1
        && filter.card_types[0]
            .to_string()
            .eq_ignore_ascii_case(fragment)
    {
        return format!("{fragment}s");
    }
    fragment.to_string()
}

fn hexproof_from_activated_and_triggered_abilities(filter: &ObjectFilter) -> bool {
    if filter.any_of.len() != 2 {
        return false;
    }

    let mut activated = ObjectFilter::default();
    activated.zone = Some(crate::zone::Zone::Stack);
    activated.stack_kind = Some(crate::filter::StackObjectKind::ActivatedAbility);
    let mut triggered = ObjectFilter::default();
    triggered.zone = Some(crate::zone::Zone::Stack);
    triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);

    filter.any_of.iter().any(|branch| branch == &activated)
        && filter.any_of.iter().any(|branch| branch == &triggered)
        && {
            let mut outer = filter.clone();
            outer.any_of.clear();
            outer == ObjectFilter::default()
        }
}

fn is_exactly_all_magic_colors_filter(filter: &ObjectFilter) -> bool {
    let mut expected = ObjectFilter::default();
    expected.colors = Some(all_magic_colors());
    filter == &expected
}

fn all_magic_colors() -> crate::color::ColorSet {
    crate::color::ColorSet::WHITE
        .union(crate::color::ColorSet::BLUE)
        .union(crate::color::ColorSet::BLACK)
        .union(crate::color::ColorSet::RED)
        .union(crate::color::ColorSet::GREEN)
}

impl StaticAbilityKind for HexproofFrom {
    fn canonical_model(&self) -> Option<super::CompiledStaticAbility> {
        Some(super::CompiledStaticAbility::hexproof_from(self.filter.clone()))
    }
    fn rewrite_text_words(&self, change: ironsmith_core::TextChange)
        -> Result<Option<super::StaticAbility>, crate::continuous::text_changes::TextChangeDomainError>
    {
        let filter = crate::continuous::text_change_predicates::rewrite_filter_words(&self.filter, change)?;
        Ok((filter != self.filter).then(|| super::StaticAbility::new(Self { filter })))
    }

    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::HexproofFrom
    }

    fn display(&self) -> String {
        format!(
            "Hexproof from {}",
            describe_hexproof_from_filter(&self.filter)
        )
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn has_hexproof(&self) -> bool {
        // HexproofFrom is NOT full hexproof - return false so that
        // the special HexproofFrom check handles it instead.
        false
    }

    fn hexproof_from_filter(&self) -> Option<&crate::target::ObjectFilter> {
        Some(&self.filter)
    }
}

/// Ward {cost}.
///
/// Whenever this permanent becomes the target of a spell or ability
/// an opponent controls, counter it unless that player pays {cost}.
#[derive(Debug, Clone, PartialEq)]
pub struct Ward {
    pub cost: TotalCost,
    retained_model: super::CompiledStaticAbility,
}

impl Ward {
    pub fn new(cost: TotalCost) -> Self {
        let retained_model = super::CompiledStaticAbility::ward(cost.clone());
        Self {
            cost,
            retained_model,
        }
    }

    fn same_payment_graph(left: &TotalCost, right: &TotalCost) -> bool {
        match (left.kind(), right.kind()) {
            (
                ironsmith_core::TotalCostKind::All(left),
                ironsmith_core::TotalCostKind::All(right),
            ) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| std::sync::Arc::ptr_eq(&left.0, &right.0))
            }
            (
                ironsmith_core::TotalCostKind::OneOf(left),
                ironsmith_core::TotalCostKind::OneOf(right),
            ) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| Self::same_payment_graph(left, right))
            }
            _ => false,
        }
    }
}


impl StaticAbilityKind for Ward {
    // Ward is handled when an object becomes targeted. It does not emit
    // characteristic-changing effects, including when its payment is nonmana.
    fn may_generate_continuous_effects(&self) -> bool {
        false
    }

    fn compiled_model(&self) -> Option<&super::CompiledStaticAbility> {
        let ironsmith_core::StaticAbilityPayload::Ward(retained) = &self.retained_model.payload
        else {
            return None;
        };
        Self::same_payment_graph(&self.cost, retained).then_some(&self.retained_model)
    }

    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Ward
    }

    fn display(&self) -> String {
        // A ward price can be a nested choice. This is a read-only display
        // predicate, not a request to select or concatenate payment branches.
        fn has_waterbend(cost: &TotalCost) -> bool {
            match cost.kind() {
                ironsmith_core::TotalCostKind::All(components) => components
                    .iter()
                    .filter_map(|component| component.mana_cost_ref())
                    .any(|mana| mana.has_waterbend_obligation()),
                ironsmith_core::TotalCostKind::OneOf(branches) => {
                    branches.iter().any(has_waterbend)
                }
            }
        }
        if has_waterbend(&self.cost) {
            return format!("Ward—{}.", self.cost.display());
        }
        // Mana-only ward uses a space ("Ward {2}"); ward with any non-mana cost
        // uses an em dash ("Ward—Discard a card").
        if self.cost.has_non_mana_costs() {
            format!("Ward—{}", self.cost.display())
        } else {
            format!("Ward {}", self.cost.display())
        }
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn ward_cost(&self) -> Option<&TotalCost> {
        Some(&self.cost)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::filter::CounterConstraint;
    use crate::object::CounterType;

    #[test]
    fn test_protection_from_color() {
        let prot = Protection::from_color(Color::Black);
        assert_eq!(prot.id(), StaticAbilityId::Protection);
        assert!(prot.has_protection());
        // from_color converts Color to ColorSet, so check it contains Black
        if let Some(ProtectionFrom::Color(colors)) = prot.protection_from() {
            assert!(colors.contains(Color::Black));
        } else {
            panic!("Expected ProtectionFrom::Color");
        }
    }

    #[test]
    fn test_protection_from_all_colors() {
        let prot = Protection::from_all_colors();
        assert!(prot.has_protection());
        assert!(matches!(
            prot.protection_from(),
            Some(ProtectionFrom::AllColors)
        ));
    }

    #[test]
    fn test_ward() {
        use crate::costs::Cost;
        let cost = TotalCost::from_cost(Cost::life(2));
        let ward = Ward::new(cost.clone());
        assert_eq!(ward.id(), StaticAbilityId::Ward);
        assert!(!ward.may_generate_continuous_effects());
        assert!(ward.ward_cost().is_some());
    }

    #[test]
    fn ward_display_visits_alternatives_without_selecting_or_flattening_them() {
        use crate::costs::Cost;
        use crate::mana::{ManaCost, ManaSymbol};
        let mana = || ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]);
        for cost in [
            TotalCost::mana(mana()),
            TotalCost::one_of(vec![TotalCost::from_cost(Cost::life(2)), TotalCost::mana(mana())]),
            TotalCost::one_of(vec![TotalCost::mana(mana()), TotalCost::one_of(vec![
                TotalCost::from_cost(Cost::life(2)), TotalCost::mana(mana().with_waterbend()),
            ])]),
        ] {
            let original = cost.clone();
            let ward = Ward::new(cost);
            let display = ward.display();
            assert!(display.starts_with("Ward"));
            assert_eq!(ward.cost, original, "rendering must retain the choice graph");
            if original.as_one_of().is_some() {
                assert!(display.contains(" or "));
                assert!(display.starts_with("Ward—"));
            } else {
                assert_eq!(display, "Ward {2}");
            }
            let restored = crate::static_abilities::StaticAbility::from_model(
                ward.compiled_model().unwrap().clone(),
            );
            assert_eq!(restored.display(), display);
            assert_eq!(restored.ward_cost(), Some(&original));
        }
    }

    #[test]
    fn test_protection_display_single_color() {
        let prot = Protection::from_color(Color::Black);
        assert_eq!(prot.display(), "Protection from black");
    }

    #[test]
    fn test_protection_display_multi_color() {
        let colors = crate::color::ColorSet::WHITE.union(crate::color::ColorSet::BLUE);
        let prot = Protection::new(ProtectionFrom::Color(colors));
        assert_eq!(prot.display(), "Protection from white and blue");
    }

    #[test]
    fn test_protection_display_permanents_filter() {
        let filter = ObjectFilter::artifact();
        let prot = Protection::new(ProtectionFrom::Permanents(filter));
        assert_eq!(prot.display(), "Protection from artifact");
    }

    #[test]
    fn test_protection_display_permanents_with_counter_filter_pluralizes() {
        let mut filter = ObjectFilter::permanent();
        filter.with_counter = Some(CounterConstraint::Typed(CounterType::Charge));
        let prot = Protection::new(ProtectionFrom::Permanents(filter));
        assert_eq!(
            prot.display(),
            "Protection from permanents with charge counters on them"
        );
    }

    #[test]
    fn test_hexproof_from() {
        let filter = ObjectFilter::default();
        let hexproof = HexproofFrom::new(filter.clone());
        assert_eq!(hexproof.id(), StaticAbilityId::HexproofFrom);
        // HexproofFrom is NOT full hexproof - it only blocks specific sources
        assert!(!hexproof.has_hexproof());
        // But it should report the filter
        assert!(hexproof.hexproof_from_filter().is_some());
        assert_eq!(hexproof.hexproof_from_filter(), Some(&filter));
    }

    #[test]
    fn hexproof_from_activated_and_triggered_abilities_compacts_typed_stack_union() {
        let mut activated = ObjectFilter::default();
        activated.zone = Some(crate::zone::Zone::Stack);
        activated.stack_kind = Some(crate::filter::StackObjectKind::ActivatedAbility);
        let mut triggered = ObjectFilter::default();
        triggered.zone = Some(crate::zone::Zone::Stack);
        triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
        let mut filter = ObjectFilter::default();
        filter.any_of = vec![activated, triggered];

        assert_eq!(
            HexproofFrom::new(filter).display(),
            "Hexproof from activated and triggered abilities"
        );
    }
}

/// Bind "the chosen card type" / "the chosen color" in a protection or
/// hexproof-from quality to the choice made for the granting object.
///
/// CR 702.16a / 702.11d: the quality is the one chosen for the object that
/// grants the ability (Serra's Emissary, Skrelv, Defector Mite). Left
/// unbound, the filter would be evaluated against the choice of the spell or
/// source being checked, which never has one. Returns `None` when nothing
/// needs binding (or the choice hasn't been made yet).
pub(crate) fn bind_chosen_protection_qualities(
    ability: &super::StaticAbility,
    game: &crate::game_state::GameState,
    chooser_source: crate::ids::ObjectId,
    static_grant: bool,
) -> Option<super::StaticAbility> {
    if static_grant
        && let Some(ProtectionFrom::ColorsAmong { filter, reference_source: None }) = ability.protection_from()
    {
        // Bind the exact granting object, not a controller snapshot or the
        // receiving creature. Later control and population changes are live.
        return Some(super::StaticAbility::protection(ProtectionFrom::ColorsAmong {
            filter: filter.clone(), reference_source: Some(chooser_source),
        }));
    }
    // "Protection from the chosen color" granted by a spell or another
    // permanent (Brave the Elements, Ward Sliver): the color is the one
    // chosen for the granting object, not for the protected creature.
    if ability.has_protection()
        && matches!(ability.protection_from(), Some(ProtectionFrom::ChosenColor))
    {
        let color = game.chosen_color(chooser_source)?;
        return Some(super::StaticAbility::protection(ProtectionFrom::Color(
            crate::color::ColorSet::from(color),
        )));
    }
    // Commander's Plate: "your commander's color identity" is the identity
    // of the granting object's controller (CR 903.4), which can differ from
    // the protected permanent's controller after a control change.
    if ability.has_protection()
        && matches!(
            ability.protection_from(),
            Some(ProtectionFrom::ColorsOutsideCommanderIdentity)
        )
    {
        let granter = game.object(chooser_source)?;
        let outside = crate::targeting::colors_outside_commander_identity(
            game,
            game.controller_of(granter),
        );
        return Some(super::StaticAbility::protection(ProtectionFrom::Color(
            outside,
        )));
    }
    if let Some(ProtectionFrom::Permanents(filter)) = ability.protection_from() {
        let bound = bind_chosen_filter_qualities(filter, game, chooser_source)?;
        return Some(super::StaticAbility::protection(ProtectionFrom::Permanents(
            bound,
        )));
    }
    if let Some(filter) = ability.hexproof_from_filter() {
        let bound = bind_chosen_filter_qualities(filter, game, chooser_source)?;
        return Some(super::StaticAbility::hexproof_from(bound));
    }
    None
}

pub(crate) fn bind_chosen_filter_qualities(
    filter: &ObjectFilter,
    game: &crate::game_state::GameState,
    chooser_source: crate::ids::ObjectId,
) -> Option<ObjectFilter> {
    let mut bound = filter.clone();
    let mut changed = false;
    // A granted choice-dependent quality belongs to the granting object.
    // Bind it before the recipient's own choice context could replace it.
    if let Some(parity @ (ironsmith_core::ParityRequirement::Chosen | ironsmith_core::ParityRequirement::NotChosen)) = bound.mana_value_parity {
        use crate::filter::ParityRequirementRuntimeExt as _;
        if let Some(resolved) = parity.resolve(game, Some(chooser_source)) {
            bound.mana_value_parity = Some(resolved);
        } else {
            // An unchosen grantor grants protection from no mana values. Do
            // not leave a relative choice for the recipient to supply later.
            bound.mana_value_parity = None;
            bound.mana_value = Some(crate::filter::Comparison::OneOf(Vec::new()));
        }
        changed = true;
    }
    if bound.chosen_card_type
        && let Some(card_type) = game.chosen_card_type(chooser_source)
    {
        bound.chosen_card_type = false;
        if !bound.all_card_types.contains(&card_type) {
            bound.all_card_types.push(card_type);
        }
        changed = true;
    }
    // "protection from creatures of the chosen type" (Riders of Gavony).
    if bound.chosen_creature_type
        && !bound.has_chosen_type_this_way_surface()
        && let Some(subtype) = game.chosen_creature_type(chooser_source)
    {
        bound.chosen_creature_type = false;
        if !bound.all_subtypes.contains(&subtype) {
            bound.all_subtypes.push(subtype);
        }
        changed = true;
    }
    if bound.chosen_color {
        match game.chosen_color(chooser_source) {
            Some(color) if bound.colors.is_none_or(|existing| existing.contains(color)) => {
                bound.colors = Some(crate::color::ColorSet::from(color));
            }
            Some(color) => {
                // `colors` requires any overlap, while `chosen_color`
                // additionally requires this particular color. Red plus a
                // blue choice matches red-blue objects. Preserve that
                // conjunction and any already required colors.
                bound.required_colors = Some(bound.required_colors
                    .unwrap_or(crate::color::ColorSet::COLORLESS).with(color));
            }
            // No color was chosen at this resolution. A later choice must
            // not retroactively supply this grant/restriction's meaning.
            None => bound.mana_value = Some(crate::filter::Comparison::OneOf(Vec::new())),
        }
        bound.chosen_color = false;
        changed = true;
    }
    for (index, branch) in filter.any_of.iter().enumerate() {
        if let Some(bound_branch) = bind_chosen_filter_qualities(branch, game, chooser_source) {
            bound.any_of[index] = bound_branch;
            changed = true;
        }
    }
    changed.then_some(bound)
}

#[cfg(test)]
mod retained_native_ward_model_tests {
    use super::*;
    #[test]
    fn retained_native_ward_model_tracks_exact_nested_payment_graph_and_rejects_stale_cost() {
        let mana =
            || crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Generic(2)]]);
        let branch = || TotalCost::mana(mana());
        let mut ward = Ward::new(TotalCost::one_of(vec![
            branch(),
            TotalCost::one_of(vec![branch(), branch()]),
        ]));
        let model = ward
            .compiled_model()
            .expect("native ward retains complete nested cost")
            .clone();
        let restored = crate::static_abilities::StaticAbility::from_model(model);
        assert_eq!(restored.display(), ward.display());
        assert!(restored.0.is_keyword());
        assert_eq!(restored.ward_cost(), Some(&ward.cost));
        assert!(ward.clone().compiled_model().is_some());
        let before = ward.cost.display();
        ward.cost = TotalCost::one_of(vec![branch(), TotalCost::one_of(vec![branch(), branch()])]);
        assert_eq!(
            ward.cost.display(),
            before,
            "same display is not the same executable payment graph"
        );
        assert!(
            ward.compiled_model().is_none(),
            "public cost replacement must invalidate captured native ward model"
        );
        let mut cost = crate::costs::Cost::mana(mana());
        assert!(matches!(
            cost.compiled_model(),
            Some(ironsmith_core::Cost::Mana(_))
        ));
        cost.0 = crate::costs::Cost::life(2).0;
        assert!(
            cost.compiled_model().is_none(),
            "native mana model cannot restore a replaced life payer"
        );
    }
}
