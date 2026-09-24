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

/// "This land is the chosen type." (Multiversal Passage)
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLandIsChosenType {
    pub display: String,
}

impl SourceLandIsChosenType {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

impl StaticAbilityKind for SourceLandIsChosenType {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SourceLandIsChosenType
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
                EffectTarget::Specific(source),
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
    /// Grant only to the source object, regardless of what `filter` says.
    pub source_only: bool,
    /// Render the granted ability's own text instead of the authored `display`.
    ///
    /// A keyword grant authors the surface it wants; a static-ability grant
    /// derives it from the ability. Carrying that as data rather than as a
    /// second type keeps one implementation of every other behavior.
    pub derived_ability_display: bool,
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
            source_only: false,
            derived_ability_display: false,
        }
    }

    /// Grant a static ability, rendering the granted ability's own text.
    pub fn from_static_ability(
        filter: ObjectFilter,
        ability: crate::static_abilities::StaticAbility,
    ) -> Self {
        Self {
            filter,
            ability: Ability::static_ability(ability),
            additional_abilities: Vec::new(),
            display: String::new(),
            condition: None,
            set_quantifier_surface: None,
            source_only: false,
            derived_ability_display: true,
        }
    }

    /// Grant a static ability to the source object only.
    pub fn source_static_ability(ability: crate::static_abilities::StaticAbility) -> Self {
        Self {
            source_only: true,
            ..Self::from_static_ability(ObjectFilter::creature(), ability)
        }
    }

    /// The static ability this grant carries, when it carries one.
    pub(crate) fn granted_static_ability(
        &self,
    ) -> Option<&crate::static_abilities::StaticAbility> {
        match &self.ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability),
            _ => None,
        }
    }

    pub(crate) fn applies_to_source(&self) -> bool {
        self.source_only || self.filter == ObjectFilter::source()
    }

    /// The surface a static-ability grant renders: derived from the granted
    /// ability rather than from an authored string. Moved verbatim from the
    /// former `GrantAbility` so the two forms cannot drift.
    fn derived_ability_display_text(
        &self,
        granted: &crate::static_abilities::StaticAbility,
    ) -> String {
        let applies_to_source = self.applies_to_source();
        let (subject, explicitly_singular_subject) = if applies_to_source {
            ("this creature".to_string(), true)
        } else if let Some(subject) =
            exact_one_condition_antecedent_subject(&self.filter, self.condition.as_ref())
        {
            (subject, false)
        } else {
            grant_subject_with_set_quantifier(&self.filter, self.set_quantifier_surface)
        };
        if applies_to_source
            && granted.id() == StaticAbilityId::CanAttackAsThoughHaste
            && let Some(crate::ConditionExpr::Not(inner)) = &self.condition
            && matches!(
                inner.as_ref(),
                crate::ConditionExpr::ObjectEnteredBattlefieldThisTurn(filter)
                    if *filter == ObjectFilter::source()
            )
        {
            return format!(
                "{subject} can attack as though it had haste unless it entered this turn"
            );
        }
        let raw_ability_text = granted.display();
        let mut ability_text = raw_ability_text.clone();
        if granted.is_keyword() {
            ability_text = lowercase_first_ascii(&ability_text);
        }
        let is_quoted_cost_modifier = matches!(
            granted.id(),
            StaticAbilityId::CostReduction
                | StaticAbilityId::CostReductionManaCost
                | StaticAbilityId::CostIncrease
                | StaticAbilityId::CostIncreaseManaCost
        );
        if is_quoted_cost_modifier {
            ability_text = capitalize_first(&ability_text);
        }
        if matches!(
            ability_text.split_whitespace().next(),
            Some("If" | "When" | "Whenever" | "At")
        ) || granted.id() == StaticAbilityId::DungeonRoomTriggerDuplication
            || granted.id() == StaticAbilityId::RuleRestriction
            || is_quoted_cost_modifier
        {
            ability_text = format!("\"{ability_text}\"");
        }
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
                "{affected} {verb} {ability_text} if mana from {mana_source} was spent to cast it"
            );
            if granted.id() == StaticAbilityId::SplitSecond
                && !rendered.to_ascii_lowercase().contains("as long as")
            {
                rendered.push_str(
                    ". (As long as it's on the stack, players can't cast spells or activate abilities that aren't mana abilities.)",
                );
            }
            return rendered;
        }
        let singular_subject = explicitly_singular_subject
            || subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ");
        let ability_text_lower = ability_text.to_ascii_lowercase();
        let mut text = match granted.id() {
            StaticAbilityId::CanAttackAsThoughNoDefender => format!(
                "{subject} can attack as though {} didn't have defender",
                if singular_subject { "it" } else { "they" }
            ),
            StaticAbilityId::Unblockable => format!("{subject} can't be blocked"),
            StaticAbilityId::CantAttack => format!("{subject} can't attack"),
            StaticAbilityId::CantBlock => format!("{subject} can't block"),
            // These restrictions are complete verb phrases, not abilities
            // introduced by "has"/"have". Keeping them structural here also
            // gives token-carried anthems the right surface (for example,
            // "Creatures you control attack each combat if able").
            StaticAbilityId::MustAttack => format!(
                "{subject} {} each combat if able",
                if singular_subject {
                    "attacks"
                } else {
                    "attack"
                }
            ),
            StaticAbilityId::MustBlock => format!(
                "{subject} {} each combat if able",
                if singular_subject { "blocks" } else { "block" }
            ),
            _ if ability_text_lower.starts_with("can't ") => {
                format!("{subject} {}", lowercase_first_ascii(&ability_text))
            }
            _ => {
                let verb = if singular_subject { "has" } else { "have" };
                // Oracle quantifies unscoped grants ("All creatures with an
                // odd mana value have haste"); scoped subjects ("Creatures
                // you control ...") stay bare.
                let lower_subject = subject.to_ascii_lowercase();
                let already_quantified = singular_subject
                    || lower_subject.starts_with("all ")
                    || lower_subject.starts_with("each ")
                    || lower_subject.starts_with("other ")
                    || lower_subject.starts_with("another ");
                let scoped = [
                    " you control",
                    " you don't control",
                    " your team controls",
                    " an opponent controls",
                    " your opponents control",
                    " that player controls",
                    " you own",
                    " they control",
                    " you cast",
                    " spells",
                    " spell",
                ]
                .iter()
                .any(|suffix| lower_subject.contains(suffix));
                // Subtype-qualified anthems stay bare in oracle ("Cleric
                // creatures have vigilance"); only generic nouns quantify.
                let generic_noun_subject = matches!(
                    lower_subject.split_whitespace().next(),
                    Some(
                        "creature"
                            | "creatures"
                            | "permanent"
                            | "permanents"
                            | "artifact"
                            | "artifacts"
                            | "enchantment"
                            | "enchantments"
                            | "land"
                            | "lands"
                            | "planeswalker"
                            | "planeswalkers"
                            | "card"
                            | "cards"
                            | "token"
                            | "tokens"
                            | "nonland"
                            | "nontoken"
                            | "nonbasic"
                    )
                );
                if !already_quantified && !scoped && generic_noun_subject {
                    format!(
                        "All {} {verb} {ability_text}",
                        lowercase_first_ascii(&subject)
                    )
                } else {
                    format!("{subject} {verb} {ability_text}")
                }
            }
        };
        if let Some(condition) = &self.condition {
            if matches!(condition, crate::ConditionExpr::SourceControllersEndStep) {
                return format!("During your end step, {text}");
            }
            if applies_to_source
                && granted.is_keyword()
                && leading_source_keyword_condition(condition)
            {
                let condition_text = normalize_source_counter_condition_text(
                    &describe_same_source_static_condition(condition),
                );
                if let Some(rest) = condition_text.strip_prefix("as long as ") {
                    return format!("as long as {rest}, {subject} has {ability_text}");
                }
            }
            let condition_text = if applies_to_source {
                describe_same_source_static_condition(condition)
            } else {
                describe_static_condition(condition)
            };
            if static_condition_is_during_your_turn(condition) {
                return format!("During your turn, {text}");
            }
            if let Some(rest) = condition_text.strip_prefix("as long as ") {
                if applies_to_source {
                    return format!("{text} as long as {rest}");
                }
                return format!("as long as {rest}, {text}");
            }
            text.push(' ');
            text.push_str(&condition_text);
        }
        text
    }

    fn effect_target(&self, source: ObjectId) -> EffectTarget {
        if self.applies_to_source() {
            EffectTarget::Source
        } else {
            effect_target_for_filter(source, &self.filter)
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
    /// One kind, two reported identities.
    ///
    /// The two grant forms were separate types; code that keys on the id —
    /// oracle-text reconstruction most of all — still needs to tell an
    /// authored surface from a derived one. The display mode is what that
    /// distinction always was, so the id now reads off it.
    fn id(&self) -> StaticAbilityId {
        if self.derived_ability_display {
            StaticAbilityId::GrantAbility
        } else {
            StaticAbilityId::GrantObjectAbilityForFilter
        }
    }

    fn grants_abilities(&self) -> bool {
        true
    }

    /// Gate the grant on its own condition.
    ///
    /// Restriction collection filters candidates through `is_active`, so
    /// without this a conditional grant ("... have split second as long as
    /// you control a Wizard") would impose its restrictions even while the
    /// condition is false.
    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source_obj) = game.object(source) else {
            return false;
        };
        super::static_condition_is_active(condition, game, source, game.controller_of(source_obj))
    }

    /// Register the restrictions of a granted static ability for the stack
    /// objects this grant reaches.
    ///
    /// Battlefield objects are deliberately not scanned here. Restriction
    /// tracking already reads the fully layered characteristics for the
    /// battlefield, so a permanent's granted abilities reach the cant tracker
    /// through that path — and only that path honors a later effect removing
    /// the ability again. Outside the battlefield it reads printed abilities
    /// instead, so a grant that lands on a spell (split second, CR 702.61b, is
    /// the printed case) is invisible to it: the object carries the ability,
    /// renders it, and answers `current_has_static_ability_id`, while no player
    /// is ever actually prohibited from anything. This closes that gap without
    /// duplicating the battlefield work or second-guessing the layer system.
    fn apply_restrictions(
        &self,
        game: &mut crate::game_state::GameState,
        _source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) {
        let granted = self
            .additional_abilities
            .iter()
            .chain(std::iter::once(&self.ability))
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.clone()),
                _ => None,
            })
            // Keywords like flying or vigilance never populate the cant
            // tracker, so a grant of one must not pay for an object scan.
            .filter(crate::game_state::GameState::static_ability_requires_cant_update)
            .collect::<Vec<_>>();
        if granted.is_empty() {
            return;
        }

        let filter_ctx = game.filter_context_for(controller, None);
        let matching: Vec<crate::ids::ObjectId> = game
            .stack
            .iter()
            .map(|entry| entry.object_id)
            .filter(|&id| {
                game.object(id)
                    .map(|obj| self.filter.matches(obj, &filter_ctx, game))
                    .unwrap_or(false)
            })
            .collect();

        for object_id in matching {
            for static_ability in &granted {
                static_ability.apply_restrictions(game, object_id, controller);
            }
        }
    }

    fn display(&self) -> String {
        if self.derived_ability_display
            && let Some(granted) = self.granted_static_ability()
        {
            return self.derived_ability_display_text(granted);
        }
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
                // A quoted ability is a whole ability, so it opens with a
                // capital even when the authored surface was lowercased.
                format!("\"{}\"", capitalize_first(&ability_text))
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

    fn granted_inline_condition(&self) -> Option<&crate::ConditionExpr> {
        self.condition.as_ref()
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
                self.effect_target(source),
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
                    self.effect_target(source),
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
