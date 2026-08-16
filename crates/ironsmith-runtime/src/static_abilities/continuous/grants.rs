use super::*;

fn normalize_symbol_case(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut in_symbol = false;
    for character in text.chars() {
        match character {
            '{' => {
                in_symbol = true;
                normalized.push(character);
            }
            '}' => {
                in_symbol = false;
                normalized.push(character);
            }
            _ if in_symbol => normalized.push(character.to_ascii_uppercase()),
            _ => normalized.push(character),
        }
    }
    normalized
}

/// Controller of source controls the permanent attached to source.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlAttachedPermanent {
    pub display: String,
}

impl ControlAttachedPermanent {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

impl StaticAbilityKind for ControlAttachedPermanent {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ControlAttachedPermanent
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::AttachedTo(source),
                Modification::ChangeController(controller),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// "Enchanted land is the chosen type."
#[derive(Debug, Clone, PartialEq)]
pub struct EnchantedLandIsChosenType {
    pub display: String,
}

impl EnchantedLandIsChosenType {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

impl StaticAbilityKind for EnchantedLandIsChosenType {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnchantedLandIsChosenType
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let Some(chosen_type) = game.chosen_basic_land_type(source) else {
            return Vec::new();
        };

        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::AttachedTo(source),
                Modification::SetSubtypes(vec![chosen_type]),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// "This creature is the chosen type in addition to its other types."
#[derive(Debug, Clone, PartialEq)]
pub struct AddChosenCreatureTypeForFilter {
    pub filter: ObjectFilter,
    pub display: String,
}

impl AddChosenCreatureTypeForFilter {
    pub fn new(filter: ObjectFilter, display: String) -> Self {
        Self { filter, display }
    }
}

impl StaticAbilityKind for AddChosenCreatureTypeForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddChosenCreatureType
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let Some(chosen_type) = game.chosen_creature_type(source) else {
            return Vec::new();
        };

        vec![
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::AddSubtypes(vec![chosen_type]),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// "Objects are the chosen basic land type in addition to their other types."
#[derive(Debug, Clone, PartialEq)]
pub struct AddChosenBasicLandTypeForFilter {
    pub filter: ObjectFilter,
    pub display: String,
}

impl AddChosenBasicLandTypeForFilter {
    pub fn new(filter: ObjectFilter, display: String) -> Self {
        Self { filter, display }
    }
}

impl StaticAbilityKind for AddChosenBasicLandTypeForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddChosenBasicLandType
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let Some(chosen_type) = game.chosen_basic_land_type(source) else {
            return Vec::new();
        };

        vec![
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::AddSubtypes(vec![chosen_type]),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// "Objects are the chosen color in addition to their other colors."
#[derive(Debug, Clone, PartialEq)]
pub struct AddChosenColorForFilter {
    pub filter: ObjectFilter,
    pub display: String,
}

impl AddChosenColorForFilter {
    pub fn new(filter: ObjectFilter, display: String) -> Self {
        Self { filter, display }
    }
}

impl StaticAbilityKind for AddChosenColorForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddChosenColor
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let Some(chosen_color) = game.chosen_color(source) else {
            return Vec::new();
        };

        vec![
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::AddColors(crate::color::ColorSet::from(chosen_color)),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// "This permanent is the chosen color."
#[derive(Debug, Clone, PartialEq)]
pub struct SetChosenColorForFilter {
    pub filter: ObjectFilter,
    pub display: String,
}

impl SetChosenColorForFilter {
    pub fn new(filter: ObjectFilter, display: String) -> Self {
        Self { filter, display }
    }
}

impl StaticAbilityKind for SetChosenColorForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetChosenColor
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let Some(chosen_color) = game.chosen_color(source) else {
            return Vec::new();
        };

        vec![
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::SetColors(crate::color::ColorSet::from(chosen_color)),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Permanents matching a filter have an activated or triggered ability.
#[derive(Clone, PartialEq)]
pub struct GrantObjectAbilityForFilter {
    pub filter: ObjectFilter,
    pub ability: Ability,
    pub additional_abilities: Vec<Ability>,
    pub display: String,
    pub condition: Option<crate::ConditionExpr>,
    pub set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
}

impl std::fmt::Debug for GrantObjectAbilityForFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrantObjectAbilityForFilter")
            .field("filter", &self.filter)
            .field("ability", &self.ability)
            .field("additional_abilities", &self.additional_abilities)
            .field(
                "generated_modification",
                &Modification::AddAbilityGeneric(self.ability.clone()),
            )
            .field("display", &self.display)
            .field("condition", &self.condition)
            .field("set_quantifier_surface", &self.set_quantifier_surface)
            .finish()
    }
}

impl GrantObjectAbilityForFilter {
    pub fn new(filter: ObjectFilter, ability: Ability, display: String) -> Self {
        Self {
            filter,
            ability,
            additional_abilities: Vec::new(),
            display,
            condition: None,
            set_quantifier_surface: None,
        }
    }

    pub fn with_additional_abilities(mut self, abilities: Vec<Ability>) -> Self {
        self.additional_abilities = abilities;
        self
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn with_set_quantifier_surface(
        mut self,
        surface: Option<ironsmith_core::SetQuantifierSurface>,
    ) -> Self {
        self.set_quantifier_surface = surface;
        self
    }
}

impl StaticAbilityKind for GrantObjectAbilityForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::GrantObjectAbilityForFilter
    }

    fn display(&self) -> String {
        let mut ability_text = normalize_symbol_case(&self.display);
        if let AbilityKind::Activated(activated) = &self.ability.kind
            && activated.is_loyalty_ability
            && let Some(rendered) = loyalty_activated_ability_display(activated, &ability_text)
        {
            ability_text = rendered;
        }
        if let Some((head, tail)) = ability_text.split_once(": ")
            && let Some(first) = tail.chars().next()
            && first.is_ascii_lowercase()
        {
            ability_text = format!("{head}: {}", capitalize_first(tail));
        }

        let filter_desc = self.filter.description();
        let keyword_label = object_ability_keyword_label(&self.ability)
            .or_else(|| explicit_granted_keyword_label(&ability_text));
        if object_ability_is_static_keyword(&self.ability) {
            ability_text = lowercase_first_ascii(&ability_text);
        } else if let Some(label) = keyword_label.as_deref() {
            ability_text = lowercase_first_ascii(label.trim());
        }
        let rendered_ability = match (&self.ability.kind, keyword_label.as_deref()) {
            (_, Some(_)) => ability_text,
            (AbilityKind::Activated(_) | AbilityKind::Triggered(_), _) => {
                if !ability_text.ends_with('.') {
                    ability_text.push('.');
                }
                format!("\"{ability_text}\"")
            }
            _ => ability_text,
        };
        if self.condition.is_none()
            && self.filter.has_mana_source_spent_trailing_if_surface()
            && let Some(source_filter) = &self.filter.mana_from_source_spent_to_cast
        {
            let mut affected_filter = self.filter.clone();
            affected_filter.mana_from_source_spent_to_cast = None;
            affected_filter.set_mana_source_spent_trailing_if_surface(false);
            let (affected, singular) =
                grant_subject_with_set_quantifier(&affected_filter, self.set_quantifier_surface);
            let verb = if singular { "has" } else { "have" };
            let mana_source = with_indefinite_article_unless_present(source_filter.description());
            let mut rendered = format!(
                "{affected} {verb} {rendered_ability} if mana from {mana_source} was spent to cast it"
            );
            if matches!(
                &self.ability.kind,
                AbilityKind::Static(ability)
                    if ability.id() == StaticAbilityId::SplitSecond
            ) && !rendered.to_ascii_lowercase().contains("as long as")
            {
                rendered.push_str(
                    ". (As long as it's on the stack, players can't cast spells or activate abilities that aren't mana abilities.)",
                );
            }
            return rendered;
        }
        let (mut subject, explicitly_singular_subject) = if let Some(subject) =
            exact_one_condition_antecedent_subject(&self.filter, self.condition.as_ref())
        {
            (subject, true)
        } else if filter_desc == "Sliver" {
            ("All Slivers".to_string(), false)
        } else {
            grant_subject_with_set_quantifier(&self.filter, self.set_quantifier_surface)
        };
        let verb = if explicitly_singular_subject {
            "has"
        } else if grant_subject_is_plural(&subject) {
            "have"
        } else {
            "has"
        };
        let renders_unblockable_restriction = matches!(
            &self.ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::Unblockable
        );
        // A source-only unblockable grant necessarily applies to a creature.
        // Keep the generic filter's internal `source` noun out of Oracle text
        // when the parser did not preserve a more specific source surface.
        if renders_unblockable_restriction && self.filter.source && subject == "this source" {
            subject = "this creature".to_string();
        }
        let mut text = if renders_unblockable_restriction {
            format!("{subject} can't be blocked")
        } else {
            format!("{subject} {verb} {rendered_ability}")
        };
        if let Some(condition) = &self.condition {
            if self.filter.controller.is_none()
                && grant_subject_is_plural(&subject)
                && let crate::ConditionExpr::CountComparison {
                    count: AnthemCountExpression::MatchingFilter(counted_filter),
                    display: Some(display),
                    ..
                } = condition
                && counted_filter.controller == Some(PlayerFilter::IteratedPlayer)
                && display.starts_with("that player ")
                && let Some(predicate) = text.strip_prefix(&subject)
            {
                return format!("{subject} each player controls{predicate} as long as {display}");
            }
            if (subject.starts_with("equipped ") || subject.starts_with("enchanted "))
                && let Some(condition_text) =
                    describe_attached_subject_static_condition(condition, &subject)
            {
                let predicate = if renders_unblockable_restriction {
                    "it can't be blocked".to_string()
                } else {
                    format!("it has {rendered_ability}")
                };
                return format!("{condition_text}, {predicate}");
            }
            if renders_unblockable_restriction
                && self.filter.source
                && (matches!(
                    condition,
                    crate::ConditionExpr::SourceIsEquipped
                        | crate::ConditionExpr::SourceIsEnchanted
                        | crate::ConditionExpr::SourceIsMonstrous
                        | crate::ConditionExpr::SourceIsAttacking
                        | crate::ConditionExpr::SourceIsUntapped
                ) || source_is_attacking_alone_condition(condition))
            {
                text.push(' ');
                text.push_str(&describe_same_source_static_condition(condition));
                return text;
            }
            let condition_text = describe_static_condition(condition);
            if static_condition_is_during_your_turn(condition) {
                return format!("During your turn, {text}");
            }
            if let Some(rest) = condition_text.strip_prefix("as long as ") {
                return format!("as long as {rest}, {text}");
            }
            text.push(' ');
            text.push_str(&condition_text);
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn granted_inline_ability(&self) -> Option<&crate::ability::Ability> {
        Some(&self.ability)
    }

    fn source_granted_inline_abilities(&self) -> Vec<&crate::ability::Ability> {
        if !self.filter.source {
            return Vec::new();
        }
        std::iter::once(&self.ability)
            .chain(self.additional_abilities.iter())
            .collect()
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let mut effects = Vec::with_capacity(1 + self.additional_abilities.len());
        effects.push(effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::AddAbilityGeneric(self.ability.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        ));
        effects.extend(self.additional_abilities.iter().cloned().map(|ability| {
            effect_with_optional_static_condition(
                ContinuousEffect::new(
                    source,
                    controller,
                    effect_target_for_filter(source, &self.filter),
                    Modification::AddAbilityGeneric(ability),
                )
                .with_source_type(EffectSourceType::StaticAbility),
                &self.condition,
            )
        }));
        effects
    }
}

fn loyalty_activated_ability_display(
    activated: &crate::ability::ActivatedAbility,
    fallback: &str,
) -> Option<String> {
    let tail = fallback
        .split_once(": ")
        .map(|(_, tail)| tail)
        .unwrap_or(fallback);
    let cost = if activated.mana_cost.is_free() {
        "0".to_string()
    } else {
        let [cost] = activated.mana_cost.as_all()? else {
            return None;
        };
        let effect = cost.effect_ref()?;
        if let Some(remove) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>()
            && remove.counter_type == CounterType::Loyalty
            && let Value::Fixed(amount) = remove.count
        {
            format!("−{amount}")
        } else if let Some(put) = effect.downcast_ref::<crate::effects::PutCountersEffect>()
            && put.counter_type == CounterType::Loyalty
            && matches!(put.target, crate::target::ChooseSpec::Source)
            && let Value::Fixed(amount) = put.amount
        {
            format!("+{amount}")
        } else {
            return None;
        }
    };
    Some(format!("[{cost}]: {tail}"))
}

fn grant_subject_is_plural(subject: &str) -> bool {
    let lower = subject.trim().to_ascii_lowercase();
    if lower.starts_with("enchanted ")
        || lower.starts_with("equipped ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
    {
        return false;
    }

    // Filter-backed grant subjects are pluralized before they reach this
    // helper. Inferring number from a trailing `s` loses scoped subtype
    // subjects such as "Elves you control" as well as invariant plurals such
    // as "Merfolk you control".
    true
}
