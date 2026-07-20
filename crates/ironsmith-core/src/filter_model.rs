use crate::{
    CardType, ChoiceCount, ChooseSpec, Color, ColorSet, CounterType, EffectMetric,
    KeywordActionKind, ObjectId, PlayerId, PriorEffectAction, SourceReferenceSurface,
    StaticAbilityId, Subtype, Supertype, TagKey, Value, Zone, effect_model::EventValueSpec,
};

fn ensure_indefinite_article(text: String) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "a permanent".to_string();
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("another ")
        || lower.starts_with("each ")
        || lower.starts_with("all ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
        || lower.starts_with("those ")
        || lower.starts_with("target ")
        || lower.starts_with("any ")
        || lower.starts_with("up to ")
        || lower.starts_with("at least ")
        || matches!(lower.as_str(), "him" | "her" | "it" | "them" | "you")
        || lower.chars().next().is_some_and(|ch| ch.is_ascii_digit())
    {
        return trimmed.to_string();
    }
    let first = trimmed.chars().next().unwrap_or('a').to_ascii_lowercase();
    let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u') {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

fn describe_filter_union_list(
    mut parts: Vec<String>,
    connective: ObjectFilterUnionConnective,
    serial_or: bool,
) -> String {
    match parts.as_slice() {
        [] => return String::new(),
        [single] => return single.clone(),
        [first, second] => {
            let joiner = match connective {
                ObjectFilterUnionConnective::Or => "or",
                ObjectFilterUnionConnective::AndOr => "and/or",
            };
            return format!("{first} {joiner} {second}");
        }
        _ => {}
    }

    if connective == ObjectFilterUnionConnective::Or && !serial_or {
        return parts.join(" or ");
    }
    let last = parts.pop().expect("union list has at least three parts");
    let joiner = match connective {
        ObjectFilterUnionConnective::Or => "or",
        ObjectFilterUnionConnective::AndOr => "and/or",
    };
    format!("{}, {joiner} {last}", parts.join(", "))
}

/// A reference to an object for use in filters and effects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ObjectRef {
    #[default]
    Target,
    Specific(ObjectId),
    Tagged(TagKey),
}

impl ObjectRef {
    pub fn tagged(tag: impl Into<TagKey>) -> Self {
        Self::Tagged(tag.into())
    }

    pub fn specific(id: ObjectId) -> Self {
        Self::Specific(id)
    }
}

/// Constraint requiring the candidate object to be a legal target for a
/// referenced stack object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetabilityConstraint {
    pub stack_object: ObjectRef,
}

impl TargetabilityConstraint {
    pub fn by_stack_object(stack_object: ObjectRef) -> Self {
        Self { stack_object }
    }
}

/// The connective Oracle used for a set-union inside an object filter.
///
/// Both variants have the same inclusive-union runtime meaning. Keeping the
/// distinction lets compiled text preserve Oracle's `and/or` surface without
/// changing which objects match the filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObjectFilterUnionConnective {
    #[default]
    Or,
    AndOr,
}

/// Oracle-facing noun retained for a same-name tagged-object relationship.
///
/// The tag and [`TaggedOpbjectRelation::SameNameAsTagged`] remain the runtime
/// source of truth. This value only prevents a later renderer from collapsing
/// an authored antecedent such as "that spell" or "that creature" to the
/// ambiguous pronoun "it".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameNameAntecedentSurface {
    Card,
    Spell,
    Permanent,
    Creature,
    Object,
}

impl SameNameAntecedentSurface {
    pub fn from_noun(noun: &str) -> Option<Self> {
        match noun {
            "card" | "cards" => Some(Self::Card),
            "spell" | "spells" => Some(Self::Spell),
            "permanent" | "permanents" => Some(Self::Permanent),
            "creature" | "creatures" => Some(Self::Creature),
            "object" | "objects" => Some(Self::Object),
            _ => None,
        }
    }

    pub const fn phrase(self) -> &'static str {
        match self {
            Self::Card => "that card",
            Self::Spell => "that spell",
            Self::Permanent => "that permanent",
            Self::Creature => "that creature",
            Self::Object => "that object",
        }
    }
}

/// Oracle-facing action used when a filter refers back to the object paid as
/// an additional cost. Object identity remains a tagged runtime relation;
/// this value only preserves whether the authored noun was sacrificed or
/// exiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdditionalCostObjectAction {
    Sacrificed,
    Exiled,
}

impl AdditionalCostObjectAction {
    pub const fn past_participle(self) -> &'static str {
        match self {
            Self::Sacrificed => "sacrificed",
            Self::Exiled => "exiled",
        }
    }
}

/// Presentation-only description of an explicit additional-cost object
/// reference such as "the sacrificed creature" or "the exiled permanent."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdditionalCostObjectSurface {
    pub action: AdditionalCostObjectAction,
    pub kind: crate::target_model::SacrificedObjectKind,
}

impl AdditionalCostObjectSurface {
    pub const fn new(
        action: AdditionalCostObjectAction,
        kind: crate::target_model::SacrificedObjectKind,
    ) -> Self {
        Self { action, kind }
    }

    pub fn description(self) -> String {
        format!("the {} {}", self.action.past_participle(), self.kind.noun())
    }
}

/// Presentation metadata for an [`ObjectFilter`].
///
/// `PartialEq` is intentionally semantic-transparent: `ObjectFilter` derives
/// equality and is used throughout lowering, deduplication, and runtime shape
/// checks. Oracle-only spelling choices must therefore compare equal to the
/// same runtime filter rendered with a canonical surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectFilterUnionSurface {
    connective: ObjectFilterUnionConnective,
    one_or_more: bool,
    /// Oracle repeated an indefinite article for every arm of an explicit
    /// object-filter union (for example, "a Doctor card, a card with ..., or
    /// a Vehicle card"). This controls punctuation and articles only.
    explicit_branch_articles: bool,
    /// Oracle selected one member of a previously established tagged set with
    /// wording such as "one of them". Lowering owns the actual choice; this
    /// flag only preserves that authored reference in compiled text.
    one_of_tagged_set: bool,
    /// Oracle explicitly quantified this set with `all` or distributive
    /// `each`. This is presentation-only and does not change the matched set.
    set_quantifier: Option<crate::effect::SetQuantifierSurface>,
    /// Oracle explicitly used `card`/`cards` for this filter's noun.
    ///
    /// Lowering may intentionally clear a nonbattlefield zone after it has
    /// encoded the actual movement or event elsewhere. Keep the noun surface
    /// independently so that the same semantic filter does not render as a
    /// battlefield `permanent` merely because its contextual zone moved.
    explicit_card_noun: bool,
    counter_requirement_one_or_more: bool,
    counter_requirement_plural_noun: bool,
    counter_requirement_plural_subject: bool,
    counter_exclusion_plural_noun: bool,
    counter_exclusion_plural_subject: bool,
    /// Oracle expressed an intrinsic attachment state postpositively, as in
    /// "creatures you control that are enchanted", instead of the canonical
    /// adjective form "enchanted creatures you control".
    relative_attachment_state: bool,
    /// Oracle expressed a coordinated subtype list as a relative clause, for
    /// example "creature that's an Insect, Rat, Spider, or Squirrel".
    relative_characteristic_list: bool,
    /// Oracle explicitly related this object set to a prior producer with
    /// `... this way`. The generated runtime tag retains identity; this field
    /// preserves the authored action without deriving semantics from tag text.
    prior_effect_action: Option<PriorEffectAction>,
    /// Explicit noun/action for a reference to an additional-cost object.
    /// This is deliberately equality-transparent with the rest of this
    /// presentation metadata.
    additional_cost_object: Option<AdditionalCostObjectSurface>,
    /// Authored noun for a `SameNameAsTagged` antecedent. The tagged relation
    /// carries identity; this field only preserves the unambiguous noun.
    same_name_antecedent: Option<SameNameAntecedentSurface>,
}

impl ObjectFilterUnionSurface {
    pub const fn new(connective: ObjectFilterUnionConnective) -> Self {
        Self {
            connective,
            one_or_more: false,
            explicit_branch_articles: false,
            one_of_tagged_set: false,
            set_quantifier: None,
            explicit_card_noun: false,
            counter_requirement_one_or_more: false,
            counter_requirement_plural_noun: false,
            counter_requirement_plural_subject: false,
            counter_exclusion_plural_noun: false,
            counter_exclusion_plural_subject: false,
            relative_attachment_state: false,
            relative_characteristic_list: false,
            prior_effect_action: None,
            additional_cost_object: None,
            same_name_antecedent: None,
        }
    }

    pub const fn connective(self) -> ObjectFilterUnionConnective {
        self.connective
    }

    pub const fn with_connective(mut self, connective: ObjectFilterUnionConnective) -> Self {
        self.connective = connective;
        self
    }

    pub const fn one_or_more(self) -> bool {
        self.one_or_more
    }

    pub const fn with_one_or_more(mut self, one_or_more: bool) -> Self {
        self.one_or_more = one_or_more;
        self
    }

    pub const fn with_explicit_branch_articles(mut self, explicit: bool) -> Self {
        self.explicit_branch_articles = explicit;
        self
    }

    pub const fn explicit_branch_articles(self) -> bool {
        self.explicit_branch_articles
    }

    pub const fn with_one_of_tagged_set(mut self, one_of_tagged_set: bool) -> Self {
        self.one_of_tagged_set = one_of_tagged_set;
        self
    }

    pub const fn one_of_tagged_set(self) -> bool {
        self.one_of_tagged_set
    }

    pub const fn with_set_quantifier(
        mut self,
        set_quantifier: Option<crate::effect::SetQuantifierSurface>,
    ) -> Self {
        self.set_quantifier = set_quantifier;
        self
    }

    pub const fn set_quantifier(self) -> Option<crate::effect::SetQuantifierSurface> {
        self.set_quantifier
    }

    pub const fn explicit_card_noun(self) -> bool {
        self.explicit_card_noun
    }

    pub const fn with_explicit_card_noun(mut self, explicit_card_noun: bool) -> Self {
        self.explicit_card_noun = explicit_card_noun;
        self
    }

    pub const fn with_counter_requirement_surface(
        mut self,
        one_or_more: bool,
        plural_noun: bool,
        plural_subject: bool,
    ) -> Self {
        self.counter_requirement_one_or_more = one_or_more;
        self.counter_requirement_plural_noun = plural_noun;
        self.counter_requirement_plural_subject = plural_subject;
        self
    }

    pub const fn counter_requirement_surface(self) -> (bool, bool, bool) {
        (
            self.counter_requirement_one_or_more,
            self.counter_requirement_plural_noun,
            self.counter_requirement_plural_subject,
        )
    }

    pub const fn with_counter_exclusion_surface(
        mut self,
        plural_noun: bool,
        plural_subject: bool,
    ) -> Self {
        self.counter_exclusion_plural_noun = plural_noun;
        self.counter_exclusion_plural_subject = plural_subject;
        self
    }

    pub const fn counter_exclusion_surface(self) -> (bool, bool) {
        (
            self.counter_exclusion_plural_noun,
            self.counter_exclusion_plural_subject,
        )
    }

    pub const fn with_relative_attachment_state(mut self, relative: bool) -> Self {
        self.relative_attachment_state = relative;
        self
    }

    pub const fn relative_attachment_state(self) -> bool {
        self.relative_attachment_state
    }

    pub const fn with_relative_characteristic_list(mut self, relative: bool) -> Self {
        self.relative_characteristic_list = relative;
        self
    }

    pub const fn relative_characteristic_list(self) -> bool {
        self.relative_characteristic_list
    }

    pub const fn with_prior_effect_action(mut self, action: Option<PriorEffectAction>) -> Self {
        self.prior_effect_action = action;
        self
    }

    pub const fn prior_effect_action(self) -> Option<PriorEffectAction> {
        self.prior_effect_action
    }

    pub const fn with_additional_cost_object(
        mut self,
        surface: Option<AdditionalCostObjectSurface>,
    ) -> Self {
        self.additional_cost_object = surface;
        self
    }

    pub const fn additional_cost_object(self) -> Option<AdditionalCostObjectSurface> {
        self.additional_cost_object
    }

    pub const fn with_same_name_antecedent(
        mut self,
        surface: Option<SameNameAntecedentSurface>,
    ) -> Self {
        self.same_name_antecedent = surface;
        self
    }

    pub const fn same_name_antecedent(self) -> Option<SameNameAntecedentSurface> {
        self.same_name_antecedent
    }
}

impl PartialEq for ObjectFilterUnionSurface {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for ObjectFilterUnionSurface {}

/// Which power/toughness reference a filter should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PtReference {
    #[default]
    Effective,
    Base,
}

/// Relationship between a candidate object's own power and toughness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerToughnessRelation {
    PowerGreaterThanToughness,
    ToughnessGreaterThanPower,
}

/// Relationship an object may have with a tagged object set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaggedOpbjectRelation {
    IsTaggedObject,
    /// Identity membership retained from an entry-time sacrifice choice. This
    /// is runtime-equivalent to `IsTaggedObject`, while preserving the authored
    /// characteristic surface "sacrificed as it entered."
    IsTaggedObjectSacrificedAsSourceEntered,
    SharesCardType,
    /// Runtime-equivalent to `SharesCardType`, while preserving the rare
    /// Oracle surface that explicitly says "permanent type."
    SharesPermanentType,
    SharesSubtypeWithTagged,
    /// The candidate must share at least one subtype with every object in the
    /// tagged set. The shared subtype may differ between tagged objects.
    SharesSubtypeWithEachTagged,
    SharesColorWithTagged,
    SharesMostCommonPermanentColor,
    SameStableId,
    SameNameAsTagged,
    DifferentNameFromTagged,
    SameControllerAsTagged,
    SameManaValueAsTagged,
    ManaValueLteTagged,
    ManaValueLtTagged,
    AttachedToTaggedObject,
    SoulbondPartnerOfTagged,
    IsNotTaggedObject,
}

/// Alternative casting capability qualifier for card filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlternativeCastKind {
    Blitz,
    Dash,
    Flashback,
    JumpStart,
    Escape,
    Madness,
    Miracle,
    Suspend,
}

/// Counter-state qualifier for object filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterConstraint {
    Any,
    Typed(CounterType),
}

/// A parity requirement for numeric object properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParityRequirement {
    Odd,
    Even,
    Chosen,
}

impl ParityRequirement {
    pub fn explicit_label(self) -> Option<&'static str> {
        match self {
            Self::Odd => Some("odd"),
            Self::Even => Some("even"),
            Self::Chosen => None,
        }
    }

    pub fn describe_axis(self, axis: &str) -> String {
        match self {
            Self::Odd | Self::Even => {
                format!("with {} {axis}", self.explicit_label().unwrap_or(""))
            }
            Self::Chosen => format!("with {axis} of the chosen quality"),
        }
    }
}

/// Power relationship against the source object in filter context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePowerRelation {
    LessThanSource,
}

/// Stack object kind constraint for stack-targeting filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackObjectKind {
    Spell,
    Ability,
    ActivatedAbility,
    TriggeredAbility,
    SpellOrAbility,
}

/// A tagged-object constraint used by object filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedObjectConstraint {
    pub tag: TagKey,
    pub relation: TaggedOpbjectRelation,
}

/// Filter for selecting players.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PlayerFilter {
    #[default]
    Any,
    You,
    NotYou,
    Opponent,
    Teammate,
    Active,
    Defending,
    Attacking,
    DamagedPlayer,
    EffectController,
    Specific(PlayerId),
    MostLifeTied,
    LowestLifeTied,
    MostCardsInHand,
    CastCardTypeThisTurn(CardType),
    CardsInHandAtLeastMoreThanYou {
        base: Box<PlayerFilter>,
        count: u32,
    },
    HasMoreLifeThanYou {
        base: Box<PlayerFilter>,
    },
    /// An opponent of `player` who controls strictly more objects matching
    /// `filter` than that player controls.
    OpponentWithMoreControlledObjectsThan {
        player: Box<PlayerFilter>,
        filter: Box<ObjectFilter>,
    },
    MaxSpeed {
        base: Box<PlayerFilter>,
        has_max_speed: bool,
    },
    ChosenPlayer,
    TaggedPlayer(TagKey),
    IteratedPlayer,
    TargetPlayerOrControllerOfTarget,
    Target(Box<PlayerFilter>),
    /// A player selected by an earlier target declaration. Runtime resolution
    /// is identical to `Target`, but compiled text uses anaphoric
    /// "that player" / "their" wording and does not imply a new target.
    AliasedTarget(Box<PlayerFilter>),
    Excluding {
        base: Box<PlayerFilter>,
        excluded: Box<PlayerFilter>,
    },
    ControllerOf(ObjectRef),
    OwnerOf(ObjectRef),
    AliasedOwnerOf(ObjectRef),
    AliasedControllerOf(ObjectRef),
}

impl PlayerFilter {
    pub fn target_player() -> Self {
        Self::Target(Box::new(Self::Any))
    }

    pub fn target_opponent() -> Self {
        Self::Target(Box::new(Self::Opponent))
    }

    pub fn excluding(base: PlayerFilter, excluded: PlayerFilter) -> Self {
        Self::Excluding {
            base: Box::new(base),
            excluded: Box::new(excluded),
        }
    }

    pub fn with_max_speed(base: PlayerFilter) -> Self {
        Self::MaxSpeed {
            base: Box::new(base),
            has_max_speed: true,
        }
    }

    pub fn without_max_speed(base: PlayerFilter) -> Self {
        Self::MaxSpeed {
            base: Box::new(base),
            has_max_speed: false,
        }
    }

    pub fn mentions_iterated_player(&self) -> bool {
        match self {
            Self::IteratedPlayer => true,
            Self::Target(inner) | Self::AliasedTarget(inner) => inner.mentions_iterated_player(),
            Self::CardsInHandAtLeastMoreThanYou { base, .. } => base.mentions_iterated_player(),
            Self::HasMoreLifeThanYou { base } => base.mentions_iterated_player(),
            Self::OpponentWithMoreControlledObjectsThan { player, filter } => {
                player.mentions_iterated_player() || filter.mentions_iterated_player()
            }
            Self::MaxSpeed { base, .. } => base.mentions_iterated_player(),
            Self::Excluding { base, excluded } => {
                base.mentions_iterated_player() || excluded.mentions_iterated_player()
            }
            Self::Any
            | Self::You
            | Self::NotYou
            | Self::Opponent
            | Self::Teammate
            | Self::Active
            | Self::Defending
            | Self::Attacking
            | Self::DamagedPlayer
            | Self::EffectController
            | Self::Specific(_)
            | Self::MostLifeTied
            | Self::LowestLifeTied
            | Self::MostCardsInHand
            | Self::CastCardTypeThisTurn(_)
            | Self::ChosenPlayer
            | Self::TaggedPlayer(_)
            | Self::TargetPlayerOrControllerOfTarget
            | Self::ControllerOf(_)
            | Self::OwnerOf(_)
            | Self::AliasedOwnerOf(_)
            | Self::AliasedControllerOf(_) => false,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Any => "a player".to_string(),
            Self::You => "you".to_string(),
            Self::NotYou => "a player other than you".to_string(),
            Self::Opponent => "an opponent".to_string(),
            Self::Teammate => "a teammate".to_string(),
            Self::Active => "the active player".to_string(),
            Self::Defending => "the defending player".to_string(),
            Self::Attacking => "the attacking player".to_string(),
            Self::DamagedPlayer => "that player".to_string(),
            Self::EffectController => "the player who cast this spell".to_string(),
            Self::Specific(_) => "that player".to_string(),
            Self::MostLifeTied => "a player with the most life or tied for most life".to_string(),
            Self::LowestLifeTied => {
                "a player with the lowest life or tied for lowest life".to_string()
            }
            Self::MostCardsInHand => "the player who has the most cards in hand".to_string(),
            Self::CastCardTypeThisTurn(card_type) => format!(
                "a player who cast one or more {} spells this turn",
                card_type.to_string().to_ascii_lowercase()
            ),
            Self::CardsInHandAtLeastMoreThanYou { base, count } => {
                let count_text = crate::cardinal_word(*count).unwrap_or_else(|| count.to_string());
                format!(
                    "{} who has at least {} more cards in hand than you do as you activate this ability",
                    base.description(),
                    count_text
                )
            }
            Self::HasMoreLifeThanYou { base } => {
                format!(
                    "{} who has more life than you do as you activate this ability",
                    base.description()
                )
            }
            Self::OpponentWithMoreControlledObjectsThan { player, filter } => format!(
                "an opponent of {} who controls more {} than they do",
                player.description(),
                pluralize_count_terminal_word(&filter.description())
            ),
            Self::MaxSpeed {
                base,
                has_max_speed,
            } => {
                let verb = if *has_max_speed {
                    "has max speed"
                } else {
                    "doesn't have max speed"
                };
                format!("{} who {verb}", base.description())
            }
            Self::ChosenPlayer => "the chosen player".to_string(),
            Self::TaggedPlayer(tag) if tag.as_str() == "enchanted" => {
                "enchanted player".to_string()
            }
            Self::TaggedPlayer(_) => "that player".to_string(),
            Self::IteratedPlayer => "that player".to_string(),
            Self::TargetPlayerOrControllerOfTarget => {
                "that player or that object's controller".to_string()
            }
            Self::Target(inner) => format!("target {}", inner.description()),
            Self::AliasedTarget(_) => "that player".to_string(),
            Self::Excluding { base, excluded } => {
                format!(
                    "{} other than {}",
                    base.description(),
                    excluded.description()
                )
            }
            Self::ControllerOf(ObjectRef::Tagged(_) | ObjectRef::Target) => {
                "its controller".to_string()
            }
            Self::OwnerOf(ObjectRef::Tagged(_) | ObjectRef::Target) => "its owner".to_string(),
            Self::ControllerOf(_) => "that object's controller".to_string(),
            Self::OwnerOf(_) => "that object's owner".to_string(),
            Self::AliasedOwnerOf(_) | Self::AliasedControllerOf(_) => "that player".to_string(),
        }
    }
}

/// A numeric comparison for filtering.
#[derive(Debug, Clone, PartialEq)]
pub enum Comparison {
    Equal(i32),
    OneOf(Vec<i32>),
    NotEqual(i32),
    LessThan(i32),
    LessThanOrEqual(i32),
    GreaterThan(i32),
    GreaterThanOrEqual(i32),
    EqualExpr(Box<Value>),
    NotEqualExpr(Box<Value>),
    LessThanExpr(Box<Value>),
    LessThanOrEqualExpr(Box<Value>),
    GreaterThanExpr(Box<Value>),
    GreaterThanOrEqualExpr(Box<Value>),
}

impl Comparison {
    pub fn satisfies(&self, value: i32) -> bool {
        match self {
            Self::Equal(n) => value == *n,
            Self::OneOf(values) => values.contains(&value),
            Self::NotEqual(n) => value != *n,
            Self::LessThan(n) => value < *n,
            Self::LessThanOrEqual(n) => value <= *n,
            Self::GreaterThan(n) => value > *n,
            Self::GreaterThanOrEqual(n) => value >= *n,
            Self::EqualExpr(_)
            | Self::NotEqualExpr(_)
            | Self::LessThanExpr(_)
            | Self::LessThanOrEqualExpr(_)
            | Self::GreaterThanExpr(_)
            | Self::GreaterThanOrEqualExpr(_) => false,
        }
    }
}

/// Filter for selecting objects (permanents, spells, cards).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectFilter {
    pub zone: Option<Zone>,
    pub controller: Option<PlayerFilter>,
    pub cast_by: Option<PlayerFilter>,
    pub cast_this_turn: bool,
    pub first_spell_cast_each_turn: bool,
    pub owner: Option<PlayerFilter>,
    pub single_graveyard: bool,
    pub targets_player: Option<PlayerFilter>,
    pub targets_object: Option<Box<ObjectFilter>>,
    pub targets_any_of: bool,
    pub stack_kind: Option<StackObjectKind>,
    pub target_count: Option<ChoiceCount>,
    pub target_set_same_controller: bool,
    pub target_set_different_controllers: bool,
    pub targets_only_player: Option<PlayerFilter>,
    pub targets_only_object: Option<Box<ObjectFilter>>,
    pub targets_only_any_of: bool,
    pub could_be_targeted_by: Option<TargetabilityConstraint>,
    pub card_types: Vec<CardType>,
    pub all_card_types: Vec<CardType>,
    pub excluded_card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub type_or_subtype_union: bool,
    pub union_surface: ObjectFilterUnionSurface,
    pub excluded_subtypes: Vec<Subtype>,
    pub supertypes: Vec<Supertype>,
    pub excluded_supertypes: Vec<Supertype>,
    pub colors: Option<ColorSet>,
    pub required_colors: Option<ColorSet>,
    pub chosen_color: bool,
    pub chosen_land_type: bool,
    /// Requires at least one of the five basic land subtypes.  This is not
    /// the same as requiring the Basic supertype: nonbasic dual lands can
    /// have basic land types, while a Basic land need not be constrained by
    /// how its supertypes are printed.
    pub has_basic_land_type: bool,
    pub has_nonbasic_land_type: bool,
    pub chosen_creature_type: bool,
    pub chosen_card_type: bool,
    pub excluded_chosen_creature_type: bool,
    pub excluded_colors: ColorSet,
    pub colorless: bool,
    pub multicolored: bool,
    pub monocolored: bool,
    pub all_colors: Option<bool>,
    pub exactly_two_colors: Option<bool>,
    pub color_count: Option<Comparison>,
    pub historic: bool,
    pub nonhistoric: bool,
    pub modified: bool,
    pub sticker: Option<KeywordActionKind>,
    pub token: bool,
    pub nontoken: bool,
    pub face_down: Option<bool>,
    pub other: bool,
    pub tapped: bool,
    pub untapped: bool,
    pub attacking: bool,
    pub attacked_this_turn: bool,
    pub attacking_player_or_planeswalker_controlled_by: Option<PlayerFilter>,
    /// When set with `attacking_player_or_planeswalker_controlled_by`, require
    /// the attack target itself to be that player rather than a planeswalker
    /// they control.
    pub attacking_player_only: bool,
    /// The battlefield object this object is attached to must match this
    /// filter. Unlike a tagged-object relation, this is an intrinsic selector
    /// and is valid without a prior effect establishing a tag.
    pub attached_to_object: Option<Box<ObjectFilter>>,
    pub attached_to_player: Option<PlayerFilter>,
    pub nonattacking: bool,
    pub enlist_eligible: bool,
    pub blocking: bool,
    pub nonblocking: bool,
    pub blocked: bool,
    pub blocked_by: Option<ObjectRef>,
    pub blocked_by_source: bool,
    pub unblocked: bool,
    pub in_combat_with_source: bool,
    pub entered_since_your_last_turn_ended: bool,
    pub entered_battlefield_this_turn: bool,
    pub entered_battlefield_controller: Option<PlayerFilter>,
    /// The object was put onto the battlefield by an effect of the current
    /// source object (for example, "the creature put onto the battlefield
    /// with this enchantment").
    pub put_onto_battlefield_with_source: bool,
    /// Oracle-facing source noun for the relation above. Runtime identity is
    /// carried by the relation and the game-state link, not this surface hint.
    pub put_onto_battlefield_with_source_surface: Option<SourceReferenceSurface>,
    /// This object is a token created by the resolving ability's source
    /// instance. Creation provenance is tracked by stable source and token
    /// identities so a source-leaves trigger can still match correctly.
    pub created_with_source: bool,
    /// Authored source noun used by `created_with_source`, for example
    /// "this enchantment".
    pub created_with_source_surface: Option<SourceReferenceSurface>,
    pub entered_graveyard_this_turn: bool,
    pub entered_graveyard_from_battlefield_this_turn: bool,
    pub surveilled_this_turn: bool,
    pub discarded_or_cycled_this_turn_by: Option<PlayerFilter>,
    pub was_dealt_damage_this_turn: bool,
    pub dealt_damage_by_source_this_turn: Option<crate::DamagedBySource>,
    pub dealt_damage_to_player_this_turn: Option<PlayerFilter>,
    pub drawn_this_turn: bool,
    pub power: Option<Comparison>,
    pub power_parity: Option<ParityRequirement>,
    pub power_reference: PtReference,
    pub power_relative_to_source: Option<SourcePowerRelation>,
    pub power_greater_than_base_power: bool,
    pub power_toughness_relation: Option<PowerToughnessRelation>,
    pub toughness: Option<Comparison>,
    pub toughness_reference: PtReference,
    pub total_power_toughness: Option<Comparison>,
    pub mana_value: Option<Comparison>,
    pub mana_value_parity: Option<ParityRequirement>,
    pub mana_value_eq_counters_on_source: Option<CounterType>,
    pub has_mana_cost: bool,
    pub has_tap_activated_ability: bool,
    pub no_abilities: bool,
    pub no_x_in_cost: bool,
    pub has_x_in_cost: bool,
    pub with_counter: Option<CounterConstraint>,
    pub without_counter: Option<CounterConstraint>,
    pub total_counters_parity: Option<ParityRequirement>,
    pub name: Option<String>,
    pub excluded_name: Option<String>,
    /// The candidate's current name must belong to an oracle identity whose
    /// earliest eligible paper printing was in this expansion.
    pub name_originally_printed_in_set: Option<String>,
    pub distinct_names: bool,
    pub distinct_powers: bool,
    pub distinct_creature_types: bool,
    pub alternative_cast: Option<AlternativeCastKind>,
    pub static_abilities: Vec<StaticAbilityId>,
    pub excluded_static_abilities: Vec<StaticAbilityId>,
    pub ability_markers: Vec<String>,
    pub excluded_ability_markers: Vec<String>,
    pub no_shared_creature_types_with: Vec<ObjectFilter>,
    pub shares_creature_type_with_source: bool,
    pub is_commander: bool,
    pub noncommander: bool,
    pub tagged_constraints: Vec<TaggedObjectConstraint>,
    pub specific: Option<ObjectId>,
    pub any_of: Vec<ObjectFilter>,
    pub source: bool,
    pub source_surface: Option<SourceReferenceSurface>,
}

impl ObjectFilter {
    pub fn mentions_iterated_player(&self) -> bool {
        [
            self.controller.as_ref(),
            self.cast_by.as_ref(),
            self.owner.as_ref(),
            self.targets_player.as_ref(),
            self.targets_only_player.as_ref(),
            self.attacking_player_or_planeswalker_controlled_by.as_ref(),
            self.attached_to_player.as_ref(),
            self.entered_battlefield_controller.as_ref(),
            self.discarded_or_cycled_this_turn_by.as_ref(),
            self.dealt_damage_to_player_this_turn.as_ref(),
        ]
        .into_iter()
        .flatten()
        .any(PlayerFilter::mentions_iterated_player)
            || self
                .targets_object
                .as_deref()
                .is_some_and(ObjectFilter::mentions_iterated_player)
            || self
                .targets_only_object
                .as_deref()
                .is_some_and(ObjectFilter::mentions_iterated_player)
            || self
                .attached_to_object
                .as_deref()
                .is_some_and(ObjectFilter::mentions_iterated_player)
            || self
                .no_shared_creature_types_with
                .iter()
                .any(ObjectFilter::mentions_iterated_player)
            || self
                .any_of
                .iter()
                .any(ObjectFilter::mentions_iterated_player)
    }

    /// Preserve the Oracle connective used for this filter's inclusive union.
    /// This does not affect runtime matching or semantic equality.
    pub fn with_union_connective(mut self, connective: ObjectFilterUnionConnective) -> Self {
        self.union_surface = self.union_surface.with_connective(connective);
        self
    }

    pub fn set_union_connective(&mut self, connective: ObjectFilterUnionConnective) {
        self.union_surface = self.union_surface.with_connective(connective);
    }

    pub const fn union_connective(&self) -> ObjectFilterUnionConnective {
        self.union_surface.connective()
    }

    pub fn set_union_one_or_more(&mut self, one_or_more: bool) {
        self.union_surface = self.union_surface.with_one_or_more(one_or_more);
    }

    pub const fn union_is_one_or_more(&self) -> bool {
        self.union_surface.one_or_more()
    }

    pub fn set_explicit_union_branch_articles(&mut self, explicit: bool) {
        self.union_surface = self.union_surface.with_explicit_branch_articles(explicit);
    }

    pub const fn has_explicit_union_branch_articles(&self) -> bool {
        self.union_surface.explicit_branch_articles()
    }

    pub fn set_one_of_tagged_set_surface(&mut self, one_of_tagged_set: bool) {
        self.union_surface = self.union_surface.with_one_of_tagged_set(one_of_tagged_set);
    }

    pub const fn has_one_of_tagged_set_surface(&self) -> bool {
        self.union_surface.one_of_tagged_set()
    }

    /// Preserve an authored leading `all`/`each` quantifier without changing
    /// filter equality or runtime matching.
    pub fn set_set_quantifier_surface(
        &mut self,
        surface: Option<crate::effect::SetQuantifierSurface>,
    ) {
        self.union_surface = self.union_surface.with_set_quantifier(surface);
    }

    pub const fn set_quantifier_surface(&self) -> Option<crate::effect::SetQuantifierSurface> {
        self.union_surface.set_quantifier()
    }

    /// Preserve an explicit Oracle `card`/`cards` noun without changing the
    /// zones or object characteristics used for matching.
    pub fn set_explicit_card_noun(&mut self, explicit_card_noun: bool) {
        self.union_surface = self
            .union_surface
            .with_explicit_card_noun(explicit_card_noun);
    }

    pub const fn has_explicit_card_noun(&self) -> bool {
        self.union_surface.explicit_card_noun()
    }

    /// Preserve whether Oracle used `one or more`, `counter`/`counters`, and
    /// `it`/`them` for a positive counter requirement. This changes rendering
    /// only.
    pub fn set_counter_requirement_surface(
        &mut self,
        one_or_more: bool,
        plural_noun: bool,
        plural_subject: bool,
    ) {
        self.union_surface = self.union_surface.with_counter_requirement_surface(
            one_or_more,
            plural_noun,
            plural_subject,
        );
    }

    pub const fn counter_requirement_surface(&self) -> (bool, bool, bool) {
        self.union_surface.counter_requirement_surface()
    }

    /// Preserve whether Oracle used `counter`/`counters` and `it`/`them` for
    /// a negative counter requirement. This changes rendering only.
    pub fn set_counter_exclusion_surface(&mut self, plural_noun: bool, plural_subject: bool) {
        self.union_surface = self
            .union_surface
            .with_counter_exclusion_surface(plural_noun, plural_subject);
    }

    pub const fn counter_exclusion_surface(&self) -> (bool, bool) {
        self.union_surface.counter_exclusion_surface()
    }

    /// Preserve postpositive attachment wording without changing matching.
    pub fn set_relative_attachment_state_surface(&mut self, relative: bool) {
        self.union_surface = self.union_surface.with_relative_attachment_state(relative);
    }

    pub const fn has_relative_attachment_state_surface(&self) -> bool {
        self.union_surface.relative_attachment_state()
    }

    /// Preserve relative-clause wording for a coordinated characteristic list.
    pub fn set_relative_characteristic_list_surface(&mut self, relative: bool) {
        self.union_surface = self
            .union_surface
            .with_relative_characteristic_list(relative);
    }

    pub const fn has_relative_characteristic_list_surface(&self) -> bool {
        self.union_surface.relative_characteristic_list()
    }

    /// Preserve an explicit `... this way` object-reference action without
    /// changing the set of objects matched at runtime.
    pub fn set_prior_effect_action_surface(&mut self, action: Option<PriorEffectAction>) {
        self.union_surface = self.union_surface.with_prior_effect_action(action);
    }

    pub const fn prior_effect_action_surface(&self) -> Option<PriorEffectAction> {
        self.union_surface.prior_effect_action()
    }

    /// Preserve the authored additional-cost noun without changing runtime
    /// filter matching.
    pub fn set_additional_cost_object_surface(
        &mut self,
        surface: Option<AdditionalCostObjectSurface>,
    ) {
        self.union_surface = self.union_surface.with_additional_cost_object(surface);
    }

    /// Preserve the authored noun of a same-name antecedent without changing
    /// the tagged runtime relationship.
    pub fn set_same_name_antecedent_surface(&mut self, surface: Option<SameNameAntecedentSurface>) {
        self.union_surface = self.union_surface.with_same_name_antecedent(surface);
    }

    pub const fn same_name_antecedent_surface(&self) -> Option<SameNameAntecedentSurface> {
        self.union_surface.same_name_antecedent()
    }

    pub const fn additional_cost_object_surface(&self) -> Option<AdditionalCostObjectSurface> {
        self.union_surface.additional_cost_object()
    }

    pub fn uses_power_or_toughness_characteristics(&self) -> bool {
        self.power.is_some()
            || self.power_parity.is_some()
            || self.power_relative_to_source.is_some()
            || self.power_greater_than_base_power
            || self.power_toughness_relation.is_some()
            || self.distinct_powers
            || self.distinct_creature_types
            || self.toughness.is_some()
            || self.total_power_toughness.is_some()
            || self
                .attached_to_object
                .as_deref()
                .is_some_and(Self::uses_power_or_toughness_characteristics)
            || self
                .any_of
                .iter()
                .any(Self::uses_power_or_toughness_characteristics)
    }

    pub fn uses_non_pt_battlefield_characteristics(&self) -> bool {
        self.controller.is_some()
            || !self.card_types.is_empty()
            || !self.all_card_types.is_empty()
            || !self.excluded_card_types.is_empty()
            || !self.subtypes.is_empty()
            || self.type_or_subtype_union
            || !self.excluded_subtypes.is_empty()
            || !self.supertypes.is_empty()
            || !self.excluded_supertypes.is_empty()
            || self.colors.is_some()
            || self.required_colors.is_some()
            || self.chosen_color
            || self.chosen_land_type
            || self.has_basic_land_type
            || self.has_nonbasic_land_type
            || self.chosen_creature_type
            || self.chosen_card_type
            || self.excluded_chosen_creature_type
            || !self.excluded_colors.is_empty()
            || self.colorless
            || self.multicolored
            || self.monocolored
            || self.all_colors.is_some()
            || self.exactly_two_colors.is_some()
            || self.color_count.is_some()
            || self.historic
            || self.nonhistoric
            || self.modified
            || self.sticker.is_some()
            || self.enlist_eligible
            || self.attached_to_object.is_some()
            || self.attached_to_player.is_some()
            || self.surveilled_this_turn
            || self.discarded_or_cycled_this_turn_by.is_some()
            || self.drawn_this_turn
            || self.mana_value.is_some()
            || self.mana_value_parity.is_some()
            || self.mana_value_eq_counters_on_source.is_some()
            || self.has_mana_cost
            || self.has_tap_activated_ability
            || self.no_abilities
            || self.no_x_in_cost
            || self.has_x_in_cost
            || self.name.is_some()
            || self.excluded_name.is_some()
            || self.name_originally_printed_in_set.is_some()
            || self.alternative_cast.is_some()
            || !self.static_abilities.is_empty()
            || !self.excluded_static_abilities.is_empty()
            || !self.ability_markers.is_empty()
            || !self.excluded_ability_markers.is_empty()
            || !self.no_shared_creature_types_with.is_empty()
            || self.shares_creature_type_with_source
            || !self.tagged_constraints.is_empty()
            || self
                .attached_to_object
                .as_deref()
                .is_some_and(Self::uses_non_pt_battlefield_characteristics)
            || self
                .any_of
                .iter()
                .any(Self::uses_non_pt_battlefield_characteristics)
    }

    pub fn permanent() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            ..Default::default()
        }
    }

    pub fn permanent_card() -> Self {
        Self {
            card_types: vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ],
            ..Default::default()
        }
    }

    pub fn specific(id: ObjectId) -> Self {
        Self {
            specific: Some(id),
            ..Default::default()
        }
    }

    pub fn creature() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Creature],
            ..Default::default()
        }
    }

    pub fn artifact() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Artifact],
            ..Default::default()
        }
    }

    pub fn enchantment() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Enchantment],
            ..Default::default()
        }
    }

    pub fn land() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Land],
            ..Default::default()
        }
    }

    pub fn planeswalker() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Planeswalker],
            ..Default::default()
        }
    }

    pub fn spell() -> Self {
        Self {
            zone: Some(Zone::Stack),
            stack_kind: Some(StackObjectKind::Spell),
            ..Default::default()
        }
    }

    pub fn spell_or_ability() -> Self {
        Self {
            zone: Some(Zone::Stack),
            stack_kind: Some(StackObjectKind::SpellOrAbility),
            ..Default::default()
        }
    }

    pub fn ability() -> Self {
        Self {
            zone: Some(Zone::Stack),
            stack_kind: Some(StackObjectKind::Ability),
            ..Default::default()
        }
    }

    pub fn activated_ability() -> Self {
        Self {
            zone: Some(Zone::Stack),
            stack_kind: Some(StackObjectKind::ActivatedAbility),
            ..Default::default()
        }
    }

    pub fn instant_or_sorcery() -> Self {
        Self {
            zone: Some(Zone::Stack),
            card_types: vec![CardType::Instant, CardType::Sorcery],
            stack_kind: Some(StackObjectKind::Spell),
            ..Default::default()
        }
    }

    pub fn noncreature_spell() -> Self {
        Self {
            excluded_card_types: vec![CardType::Creature, CardType::Land],
            ..Default::default()
        }
    }

    pub fn nonland_permanent() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            excluded_card_types: vec![CardType::Land],
            ..Default::default()
        }
    }

    pub fn noncreature_permanent() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            excluded_card_types: vec![CardType::Creature],
            ..Default::default()
        }
    }

    pub fn nonland() -> Self {
        Self {
            excluded_card_types: vec![CardType::Land],
            ..Default::default()
        }
    }

    pub fn in_zone(mut self, zone: Zone) -> Self {
        self.zone = Some(zone);
        self
    }

    pub fn with_default_zone(mut self, zone: Zone) -> Self {
        self.zone.get_or_insert(zone);
        self
    }

    pub fn ensure_zone(&mut self, zone: Zone) -> Zone {
        *self.zone.get_or_insert(zone)
    }

    pub fn has_search_stated_quality(&self) -> bool {
        let mut generic = Self::default();
        generic.zone = self.zone;
        generic.owner = self.owner.clone();
        generic.controller = self.controller.clone();
        generic.cast_by = self.cast_by.clone();
        generic.first_spell_cast_each_turn = self.first_spell_cast_each_turn;
        generic.single_graveyard = self.single_graveyard;
        self != &generic
    }

    pub fn targeting(mut self, player: Option<PlayerFilter>, object: Option<ObjectFilter>) -> Self {
        self.zone = Some(Zone::Stack);
        self.targets_player = player;
        self.targets_object = object.map(Box::new);
        self
    }

    pub fn targeting_only(
        mut self,
        player: Option<PlayerFilter>,
        object: Option<ObjectFilter>,
    ) -> Self {
        self.zone = Some(Zone::Stack);
        self.targets_only_player = player;
        self.targets_only_object = object.map(Box::new);
        if self.targets_only_player.is_some() && self.targets_only_object.is_some() {
            self.targets_only_any_of = true;
        }
        self
    }

    pub fn targeting_only_player(self, player: PlayerFilter) -> Self {
        self.targeting_only(Some(player), None)
    }

    pub fn targeting_only_object(self, object: ObjectFilter) -> Self {
        self.targeting_only(None, Some(object))
    }

    pub fn with_target_count(mut self, count: ChoiceCount) -> Self {
        self.target_count = Some(count);
        self
    }

    pub fn target_count_exact(self, count: usize) -> Self {
        self.with_target_count(ChoiceCount::exactly(count))
    }

    pub fn could_be_targeted_by(mut self, stack_object: ObjectRef) -> Self {
        self.could_be_targeted_by = Some(TargetabilityConstraint::by_stack_object(stack_object));
        self
    }

    pub fn targeting_player(self, player: PlayerFilter) -> Self {
        self.targeting(Some(player), None)
    }

    pub fn targeting_object(self, object: ObjectFilter) -> Self {
        self.targeting(None, Some(object))
    }

    pub fn controlled_by(mut self, controller: PlayerFilter) -> Self {
        self.controller = Some(controller);
        self
    }

    pub fn attacking_player_or_planeswalker_controlled_by(mut self, player: PlayerFilter) -> Self {
        self.attacking_player_or_planeswalker_controlled_by = Some(player);
        self
    }

    pub fn attacking_player(mut self, player: PlayerFilter) -> Self {
        self.attacking_player_or_planeswalker_controlled_by = Some(player);
        self.attacking_player_only = true;
        self
    }

    pub fn cast_by(mut self, caster: PlayerFilter) -> Self {
        self.cast_by = Some(caster);
        self
    }

    pub fn cast_by_you(self) -> Self {
        self.cast_by(PlayerFilter::You)
    }

    pub fn first_spell_cast_each_turn(mut self) -> Self {
        self.first_spell_cast_each_turn = true;
        self
    }

    pub fn you_control(self) -> Self {
        self.controlled_by(PlayerFilter::You)
    }

    pub fn opponent_controls(self) -> Self {
        self.controlled_by(PlayerFilter::Opponent)
    }

    pub fn owned_by(mut self, owner: PlayerFilter) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn single_graveyard(mut self) -> Self {
        self.single_graveyard = true;
        self
    }

    pub fn other(mut self) -> Self {
        self.other = true;
        self
    }

    pub fn with_type(mut self, card_type: CardType) -> Self {
        self.card_types.push(card_type);
        self
    }

    pub fn with_all_type(mut self, card_type: CardType) -> Self {
        self.all_card_types.push(card_type);
        self
    }

    pub fn without_type(mut self, card_type: CardType) -> Self {
        self.excluded_card_types.push(card_type);
        self
    }

    pub fn with_subtype(mut self, subtype: Subtype) -> Self {
        self.subtypes.push(subtype);
        self
    }

    pub fn without_subtype(mut self, subtype: Subtype) -> Self {
        self.excluded_subtypes.push(subtype);
        self
    }

    pub fn exclude_subtype(self, subtype: Subtype) -> Self {
        self.without_subtype(subtype)
    }

    pub fn with_supertype(mut self, supertype: Supertype) -> Self {
        self.supertypes.push(supertype);
        self
    }

    pub fn without_supertype(mut self, supertype: Supertype) -> Self {
        self.excluded_supertypes.push(supertype);
        self
    }

    pub fn nonbasic(self) -> Self {
        self.without_supertype(Supertype::Basic)
    }

    pub fn token(mut self) -> Self {
        self.token = true;
        self
    }

    pub fn nontoken(mut self) -> Self {
        self.nontoken = true;
        self
    }

    pub fn face_down(mut self) -> Self {
        self.face_down = Some(true);
        self
    }

    pub fn face_up(mut self) -> Self {
        self.face_down = Some(false);
        self
    }

    pub fn tapped(mut self) -> Self {
        self.tapped = true;
        self
    }

    pub fn untapped(mut self) -> Self {
        self.untapped = true;
        self
    }

    pub fn enlist_eligible(mut self) -> Self {
        self.enlist_eligible = true;
        self
    }

    pub fn with_power(mut self, cmp: Comparison) -> Self {
        self.power = Some(cmp);
        self.power_reference = PtReference::Effective;
        self
    }

    pub fn with_power_parity(mut self, parity: ParityRequirement) -> Self {
        self.power_parity = Some(parity);
        self
    }

    pub fn with_base_power(mut self, cmp: Comparison) -> Self {
        self.power = Some(cmp);
        self.power_reference = PtReference::Base;
        self
    }

    pub fn with_power_less_than_source(mut self) -> Self {
        self.power_relative_to_source = Some(SourcePowerRelation::LessThanSource);
        self
    }

    pub fn with_power_toughness_relation(mut self, relation: PowerToughnessRelation) -> Self {
        self.power_toughness_relation = Some(relation);
        self
    }

    pub fn with_toughness(mut self, cmp: Comparison) -> Self {
        self.toughness = Some(cmp);
        self.toughness_reference = PtReference::Effective;
        self
    }

    pub fn with_total_power_toughness(mut self, cmp: Comparison) -> Self {
        self.total_power_toughness = Some(cmp);
        self
    }

    pub fn with_base_toughness(mut self, cmp: Comparison) -> Self {
        self.toughness = Some(cmp);
        self.toughness_reference = PtReference::Base;
        self
    }

    pub fn with_mana_value(mut self, cmp: Comparison) -> Self {
        self.mana_value = Some(cmp);
        self
    }

    pub fn with_color_count(mut self, cmp: Comparison) -> Self {
        self.color_count = Some(cmp);
        self
    }

    pub fn with_mana_value_parity(mut self, parity: ParityRequirement) -> Self {
        self.mana_value_parity = Some(parity);
        self
    }

    pub fn with_total_counters_parity(mut self, parity: ParityRequirement) -> Self {
        self.total_counters_parity = Some(parity);
        self
    }

    pub fn with_colors(mut self, colors: ColorSet) -> Self {
        self.colors = Some(colors);
        self
    }

    pub fn of_chosen_color(mut self) -> Self {
        self.chosen_color = true;
        self
    }

    pub fn of_chosen_land_type(mut self) -> Self {
        self.chosen_land_type = true;
        self
    }

    pub fn of_chosen_creature_type(mut self) -> Self {
        self.chosen_creature_type = true;
        self
    }

    pub fn discarded_or_cycled_this_turn_by(mut self, player: PlayerFilter) -> Self {
        self.discarded_or_cycled_this_turn_by = Some(player);
        self
    }

    pub fn of_chosen_card_type(mut self) -> Self {
        self.chosen_card_type = true;
        self
    }

    pub fn not_of_chosen_creature_type(mut self) -> Self {
        self.excluded_chosen_creature_type = true;
        self
    }

    pub fn without_colors(mut self, colors: ColorSet) -> Self {
        self.excluded_colors = self.excluded_colors.union(colors);
        self
    }

    pub fn colorless(mut self) -> Self {
        self.colorless = true;
        self
    }

    pub fn multicolored(mut self) -> Self {
        self.multicolored = true;
        self
    }

    pub fn monocolored(mut self) -> Self {
        self.monocolored = true;
        self
    }

    pub fn historic(mut self) -> Self {
        self.historic = true;
        self
    }

    pub fn nonhistoric(mut self) -> Self {
        self.nonhistoric = true;
        self
    }

    pub fn modified(mut self) -> Self {
        self.modified = true;
        self
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn not_named(mut self, name: impl Into<String>) -> Self {
        self.excluded_name = Some(name.into());
        self
    }

    pub fn commander(mut self) -> Self {
        self.is_commander = true;
        self
    }

    pub fn noncommander(mut self) -> Self {
        self.noncommander = true;
        self
    }

    pub fn with_alternative_cast(mut self, kind: AlternativeCastKind) -> Self {
        self.alternative_cast = Some(kind);
        self
    }

    pub fn with_any_counter(mut self) -> Self {
        self.with_counter = Some(CounterConstraint::Any);
        self
    }

    pub fn with_counter_type(mut self, counter_type: CounterType) -> Self {
        self.with_counter = Some(CounterConstraint::Typed(counter_type));
        self
    }

    pub fn without_any_counter(mut self) -> Self {
        self.without_counter = Some(CounterConstraint::Any);
        self
    }

    pub fn without_counter_type(mut self, counter_type: CounterType) -> Self {
        self.without_counter = Some(CounterConstraint::Typed(counter_type));
        self
    }

    pub fn with_static_ability(mut self, ability_id: StaticAbilityId) -> Self {
        if !self.static_abilities.contains(&ability_id) {
            self.static_abilities.push(ability_id);
        }
        self
    }

    pub fn without_static_ability(mut self, ability_id: StaticAbilityId) -> Self {
        if !self.excluded_static_abilities.contains(&ability_id) {
            self.excluded_static_abilities.push(ability_id);
        }
        self
    }

    pub fn with_ability_marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !self
            .ability_markers
            .iter()
            .any(|m| m.eq_ignore_ascii_case(&marker))
        {
            self.ability_markers.push(marker);
        }
        self
    }

    pub fn without_ability_marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !self
            .excluded_ability_markers
            .iter()
            .any(|m| m.eq_ignore_ascii_case(&marker))
        {
            self.excluded_ability_markers.push(marker);
        }
        self
    }

    pub fn sharing_no_creature_types_with(mut self, filter: ObjectFilter) -> Self {
        self.no_shared_creature_types_with.push(filter);
        self
    }

    pub fn with_tap_activated_ability(mut self) -> Self {
        self.has_tap_activated_ability = true;
        self
    }

    pub fn match_tagged(mut self, tag: impl Into<TagKey>, relation: TaggedOpbjectRelation) -> Self {
        self.tagged_constraints.push(TaggedObjectConstraint {
            tag: tag.into(),
            relation,
        });
        self
    }

    pub fn shares_card_type_with_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SharesCardType)
    }

    pub fn shares_color_with_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SharesColorWithTagged)
    }

    pub fn shares_most_common_permanent_color(self) -> Self {
        self.match_tagged(
            TagKey::from("most_common_permanent_color"),
            TaggedOpbjectRelation::SharesMostCommonPermanentColor,
        )
    }

    pub fn shares_subtype_with_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SharesSubtypeWithTagged)
    }

    pub fn shares_subtype_with_each_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SharesSubtypeWithEachTagged)
    }

    pub fn same_stable_id_as_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SameStableId)
    }

    pub fn tagged(tag: impl Into<TagKey>) -> Self {
        Self::default().match_tagged(tag, TaggedOpbjectRelation::IsTaggedObject)
    }

    pub fn not_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::IsNotTaggedObject)
    }

    pub fn any_of_types(types: &[CardType]) -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: types.to_vec(),
            ..Default::default()
        }
    }

    pub fn source() -> Self {
        Self {
            source: true,
            ..Default::default()
        }
    }

    pub fn source_with_surface(surface: SourceReferenceSurface) -> Self {
        Self::source().with_source_surface(surface)
    }

    pub fn with_source_surface(mut self, surface: SourceReferenceSurface) -> Self {
        self.source = true;
        self.source_surface = Some(surface);
        self
    }

    pub fn description(&self) -> String {
        let any_of_keyword_clause =
            describe_simple_any_of_keyword_clause(&self.any_of, self.union_connective());
        if let Some(description) = describe_branch_scoped_card_type_union(self) {
            return description;
        }
        if let Some(description) = describe_owner_scoped_zone_union(self) {
            return description;
        }
        if any_of_keyword_clause.is_none() && !self.any_of.is_empty() {
            let explicit_branch_articles = self.has_explicit_union_branch_articles();
            let mut description = describe_filter_union_list(
                self.any_of
                    .iter()
                    .map(ObjectFilter::description)
                    .map(|description| {
                        if explicit_branch_articles {
                            ensure_indefinite_article(description)
                        } else {
                            description
                        }
                    })
                    .collect(),
                self.union_connective(),
                explicit_branch_articles,
            );
            if let Some(attached_to) = &self.attached_to_object {
                description.push_str(&format!(
                    " attached to {}",
                    ensure_indefinite_article(attached_to.description())
                ));
            }
            if let Some(attached_to_player) = &self.attached_to_player {
                description.push_str(&format!(
                    " attached to {}",
                    attached_to_player.description()
                ));
            }
            return description;
        }

        let mut parts = Vec::new();
        let mut post_noun_qualifiers: Vec<String> = Vec::new();
        let append_token_after_type = self.token;
        let mut controller_suffix: Option<String> = None;
        let mut owner_suffix: Option<String> = None;
        let source_surface_text = if self.source {
            self.source_surface
                .as_ref()
                .map(source_reference_surface_text)
        } else {
            None
        };
        if self.other {
            // `other` is semantically an exclusion of the source, but Oracle
            // expresses that relation adjectivally as "another".  The source
            // surface still matters when `source` itself is described; it
            // should not expand an ordinary trigger subject into the internal
            // phrase "... other than this permanent".
            parts.push("another".to_string());
        }
        let has_target_tag = self.tagged_constraints.iter().any(|constraint| {
            matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                && constraint.tag.as_str().starts_with("targeted")
        });
        if has_target_tag {
            parts.push("target".to_string());
        }
        let has_chosen_tag = self.tagged_constraints.iter().any(|constraint| {
            matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                && constraint.tag.as_str() == "__chosen_objects__"
        });
        if has_chosen_tag {
            parts.push("the chosen".to_string());
        }
        if self.source {
            parts.push(
                source_surface_text
                    .clone()
                    .unwrap_or_else(|| "this".to_string()),
            );
        }
        if self.modified {
            parts.push("modified".to_string());
        }

        let has_leading_determiner = self.other || has_target_tag || has_chosen_tag || self.source;

        if let Some(ref ctrl) = self.controller {
            match ctrl {
                PlayerFilter::You => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("you control".to_string());
                }
                PlayerFilter::NotYou => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("you don't control".to_string());
                }
                PlayerFilter::Opponent => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("an opponent controls".to_string());
                }
                PlayerFilter::Any => {}
                PlayerFilter::Active => parts.push("the active player's".to_string()),
                PlayerFilter::EffectController => {
                    parts.push("the player who cast this spell's".to_string())
                }
                PlayerFilter::Specific(_) => parts.push("a specific player's".to_string()),
                PlayerFilter::MostLifeTied => {
                    parts.push("the player with the most life's".to_string())
                }
                PlayerFilter::LowestLifeTied => {
                    parts.push("the player with the lowest life's".to_string())
                }
                PlayerFilter::MostCardsInHand => {
                    parts.push("the player with the most cards in hand's".to_string())
                }
                PlayerFilter::CastCardTypeThisTurn(card_type) => parts.push(format!(
                    "a player who cast one or more {} spells this turn's",
                    card_type.to_string().to_ascii_lowercase()
                )),
                PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::HasMoreLifeThanYou { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::MaxSpeed { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::ChosenPlayer => parts.push("the chosen player's".to_string()),
                PlayerFilter::TaggedPlayer(_) => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("that player controls".to_string());
                }
                PlayerFilter::Teammate => parts.push("a teammate's".to_string()),
                PlayerFilter::Defending => parts.push("the defending player's".to_string()),
                PlayerFilter::Attacking => parts.push("an attacking player's".to_string()),
                PlayerFilter::DamagedPlayer => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("that player controls".to_string());
                }
                PlayerFilter::IteratedPlayer => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("that player controls".to_string())
                }
                PlayerFilter::TargetPlayerOrControllerOfTarget => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix =
                        Some("that player or that object's controller controls".to_string())
                }
                PlayerFilter::Excluding { base, excluded }
                    if matches!(base.as_ref(), PlayerFilter::Any)
                        && matches!(
                            excluded.as_ref(),
                            PlayerFilter::ControllerOf(ObjectRef::Tagged(_) | ObjectRef::Target)
                        ) =>
                {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("another player controls".to_string());
                }
                PlayerFilter::Excluding { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::Target(inner) => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    let inner_desc = describe_player_filter(inner.as_ref());
                    controller_suffix = Some(format!("target {inner_desc} controls"));
                }
                PlayerFilter::AliasedTarget(_) => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("that player controls".to_string());
                }
                PlayerFilter::ControllerOf(_) => parts.push("a controller's".to_string()),
                PlayerFilter::OwnerOf(_) => parts.push("an owner's".to_string()),
                PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
                    parts.push("that player's".to_string())
                }
            }
        }

        if let Some(cast_by) = &self.cast_by {
            post_noun_qualifiers.push(format!("cast by {}", describe_player_filter(cast_by)));
        }
        if self.cast_this_turn {
            post_noun_qualifiers.push("cast this turn".to_string());
        }
        if self.first_spell_cast_each_turn {
            post_noun_qualifiers.push("first spell cast each turn".to_string());
        }

        let owner_conveyed_by_zone = matches!(
            self.zone,
            Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
        );
        if !owner_conveyed_by_zone && let Some(ref owner) = self.owner {
            owner_suffix = Some(match owner {
                PlayerFilter::You => "you own".to_string(),
                PlayerFilter::NotYou => "you don't own".to_string(),
                PlayerFilter::Opponent => "an opponent owns".to_string(),
                PlayerFilter::Any => "a player owns".to_string(),
                PlayerFilter::Active => "the active player owns".to_string(),
                PlayerFilter::EffectController => "the player who cast this spell owns".to_string(),
                PlayerFilter::Specific(_) => "that player owns".to_string(),
                PlayerFilter::MostLifeTied => {
                    "the player with the most life or tied for most life owns".to_string()
                }
                PlayerFilter::LowestLifeTied => {
                    "the player with the lowest life or tied for lowest life owns".to_string()
                }
                PlayerFilter::MostCardsInHand => {
                    "the player who has the most cards in hand owns".to_string()
                }
                PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
                    "a player who cast one or more {} spells this turn owns",
                    card_type.to_string().to_ascii_lowercase()
                ),
                PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::HasMoreLifeThanYou { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::MaxSpeed { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::ChosenPlayer => "the chosen player owns".to_string(),
                PlayerFilter::TaggedPlayer(_) => "that player owns".to_string(),
                PlayerFilter::Teammate => "a teammate owns".to_string(),
                PlayerFilter::Defending => "the defending player owns".to_string(),
                PlayerFilter::Attacking => "an attacking player owns".to_string(),
                PlayerFilter::DamagedPlayer => "that player owns".to_string(),
                PlayerFilter::IteratedPlayer => "that player owns".to_string(),
                PlayerFilter::TargetPlayerOrControllerOfTarget => {
                    "that player or that object's controller owns".to_string()
                }
                PlayerFilter::Excluding { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::Target(inner) => {
                    format!("target {} owns", describe_player_filter(inner.as_ref()))
                }
                PlayerFilter::AliasedTarget(_) => "that player owns".to_string(),
                PlayerFilter::ControllerOf(_) => "that object's controller owns".to_string(),
                PlayerFilter::OwnerOf(_) => "that object's owner owns".to_string(),
                PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
                    "that player owns".to_string()
                }
            });
        }

        if self.nontoken && !self.has_explicit_card_noun() {
            parts.push("nontoken".to_string());
        }
        if let Some(face_down) = self.face_down {
            parts.push(if face_down {
                "face-down".to_string()
            } else {
                "face-up".to_string()
            });
        }
        if let Some(colors) = self.required_colors {
            let mut color_words = Vec::new();
            if colors.contains(Color::White) {
                color_words.push("white");
            }
            if colors.contains(Color::Blue) {
                color_words.push("blue");
            }
            if colors.contains(Color::Black) {
                color_words.push("black");
            }
            if colors.contains(Color::Red) {
                color_words.push("red");
            }
            if colors.contains(Color::Green) {
                color_words.push("green");
            }
            if !color_words.is_empty() {
                parts.push(format!("both {}", color_words.join(" and ")));
            }
        } else if let Some(colors) = self.colors {
            if colors.contains_all(Color::ALL.into_iter().collect::<ColorSet>()) {
                parts.push("colored".to_string());
            } else {
                let mut color_words = Vec::new();
                if colors.contains(Color::White) {
                    color_words.push("white");
                }
                if colors.contains(Color::Blue) {
                    color_words.push("blue");
                }
                if colors.contains(Color::Black) {
                    color_words.push("black");
                }
                if colors.contains(Color::Red) {
                    color_words.push("red");
                }
                if colors.contains(Color::Green) {
                    color_words.push("green");
                }
                if !color_words.is_empty() {
                    parts.push(describe_filter_union_list(
                        color_words.into_iter().map(str::to_string).collect(),
                        self.union_connective(),
                        false,
                    ));
                }
            }
        }
        if self.chosen_color {
            post_noun_qualifiers.push("of the chosen color".to_string());
        }
        if let Some(sticker) = self.sticker {
            let sticker = match sticker {
                KeywordActionKind::ArtSticker => "an art sticker",
                KeywordActionKind::AbilitySticker => "an ability sticker",
                KeywordActionKind::PowerToughnessSticker => "a power and toughness sticker",
                KeywordActionKind::NameSticker => "a name sticker",
                _ => "a sticker",
            };
            post_noun_qualifiers.push(format!("with {sticker} on it"));
        }
        if self.chosen_land_type {
            post_noun_qualifiers.push("of the chosen land type".to_string());
        }
        if self.has_basic_land_type {
            post_noun_qualifiers.push("with a basic land type".to_string());
        }
        if self.has_nonbasic_land_type {
            post_noun_qualifiers.push("with a nonbasic land type".to_string());
        }
        if self.chosen_creature_type {
            post_noun_qualifiers.push("of the chosen type".to_string());
        }
        if self.chosen_card_type {
            post_noun_qualifiers.push("of the chosen type".to_string());
        }
        if let Some(set_name) = &self.name_originally_printed_in_set {
            post_noun_qualifiers.push(format!(
                "with a name originally printed in the {set_name} expansion"
            ));
        }
        if self.excluded_chosen_creature_type {
            post_noun_qualifiers.push("that aren't of the chosen type".to_string());
        }
        if !self.no_shared_creature_types_with.is_empty() {
            let comparison = self
                .no_shared_creature_types_with
                .iter()
                .map(ObjectFilter::description)
                .collect::<Vec<_>>()
                .join(" or ");
            post_noun_qualifiers.push(format!(
                "that doesn't share a creature type with {comparison}"
            ));
        }
        if self.shares_creature_type_with_source {
            post_noun_qualifiers.push("that shares a creature type with this creature".to_string());
        }
        for constraint in &self.tagged_constraints {
            match constraint.relation {
                TaggedOpbjectRelation::IsTaggedObject
                | TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered => {
                    match constraint.tag.as_str() {
                        "it" => parts.push("that".to_string()),
                        "enchanted" => parts.push("enchanted".to_string()),
                        "equipped" => parts.push("equipped".to_string()),
                        "convoked_this_spell" => {
                            post_noun_qualifiers.push("that convoked this spell".to_string());
                        }
                        "improvised_this_spell" => {
                            post_noun_qualifiers.push("that improvised this spell".to_string());
                        }
                        "crewed_it_this_turn" => {
                            post_noun_qualifiers.push("that crewed it this turn".to_string());
                        }
                        "saddled_it_this_turn" => {
                            post_noun_qualifiers.push("that saddled it this turn".to_string());
                        }
                        tag if tag == crate::SOURCE_EXILED_TAG => {
                            post_noun_qualifiers.push("exiled with this permanent".to_string());
                        }
                        _ => {}
                    }
                }
                TaggedOpbjectRelation::IsNotTaggedObject => {
                    parts.push("other".to_string());
                }
                TaggedOpbjectRelation::SameNameAsTagged => {
                    if constraint.tag.as_str() == crate::SOURCE_EXILED_TAG {
                        post_noun_qualifiers.push(
                            "with the same name as a card exiled with this permanent".to_string(),
                        );
                    } else {
                        let antecedent = self
                            .same_name_antecedent_surface()
                            .map(SameNameAntecedentSurface::phrase)
                            .unwrap_or_else(|| match constraint.tag.as_str() {
                                // The implicit object tag is established by choosing or revealing
                                // a card. Keeping the lexical kind avoids an ambiguous pronoun
                                // across a following search through multiple zones.
                                "__it__" => "that card",
                                // A triggering spell is commonly followed by a same-name search or
                                // graveyard count. Battlefield trigger subjects remain permanents.
                                "triggering" if self.zone != Some(Zone::Battlefield) => {
                                    "that spell"
                                }
                                _ => "it",
                            });
                        post_noun_qualifiers.push(format!("with the same name as {antecedent}"));
                    }
                }
                TaggedOpbjectRelation::DifferentNameFromTagged => {
                    post_noun_qualifiers
                        .push("with a different name from those objects".to_string());
                }
                TaggedOpbjectRelation::SameControllerAsTagged => {
                    post_noun_qualifiers.push("controlled by its controller".to_string());
                }
                TaggedOpbjectRelation::SameManaValueAsTagged => {
                    if let Some(surface) = self.additional_cost_object_surface() {
                        post_noun_qualifiers.push(format!(
                            "with the same mana value as {}",
                            surface.description()
                        ));
                    } else if constraint.tag.as_str().starts_with("sacrifice_cost_") {
                        post_noun_qualifiers.push(
                            "with the same mana value as the sacrificed creature".to_string(),
                        );
                    } else {
                        post_noun_qualifiers.push("with the same mana value as it".to_string());
                    }
                }
                TaggedOpbjectRelation::ManaValueLteTagged => {
                    if constraint.tag.as_str() == "triggering" {
                        post_noun_qualifiers
                            .push("with equal or lesser mana value than that spell".to_string());
                    } else {
                        post_noun_qualifiers.push(
                            "with mana value less than or equal to its mana value".to_string(),
                        );
                    }
                }
                TaggedOpbjectRelation::ManaValueLtTagged => {
                    post_noun_qualifiers.push("with lesser mana value than it".to_string());
                }
                TaggedOpbjectRelation::SharesColorWithTagged => {
                    if let Some(surface) = self.additional_cost_object_surface() {
                        post_noun_qualifiers.push(format!(
                            "that shares a color with {}",
                            surface.description()
                        ));
                    } else {
                        post_noun_qualifiers.push("that shares a color with it".to_string());
                    }
                }
                TaggedOpbjectRelation::SharesMostCommonPermanentColor => {
                    post_noun_qualifiers.push(
                        "that shares a color with the most common color among all permanents or a color tied for most common"
                            .to_string(),
                    );
                }
                TaggedOpbjectRelation::SharesSubtypeWithTagged => {
                    if let Some(surface) = self.additional_cost_object_surface() {
                        post_noun_qualifiers.push(format!(
                            "that shares a creature type with {}",
                            surface.description()
                        ));
                    } else {
                        post_noun_qualifiers
                            .push("that shares a creature type with it".to_string());
                    }
                }
                TaggedOpbjectRelation::SharesSubtypeWithEachTagged => {
                    post_noun_qualifiers.push(
                        "that shares a creature type with each creature tapped this way"
                            .to_string(),
                    );
                }
                TaggedOpbjectRelation::SharesCardType => {
                    if constraint.tag.as_str() == crate::SOURCE_EXILED_TAG {
                        post_noun_qualifiers.push(
                            "that shares a card type with a card exiled with this permanent"
                                .to_string(),
                        );
                        continue;
                    }
                    if let Some(surface) = self.additional_cost_object_surface() {
                        post_noun_qualifiers.push(format!(
                            "that shares a card type with {}",
                            surface.description()
                        ));
                        continue;
                    }
                    if constraint.tag.as_str().starts_with("sacrificed_")
                        || constraint.tag.as_str().starts_with("sacrifice_cost_")
                    {
                        post_noun_qualifiers.push(
                            "that shares a card type with the sacrificed permanent".to_string(),
                        );
                        continue;
                    }
                    post_noun_qualifiers.push("that shares a card type with it".to_string());
                }
                TaggedOpbjectRelation::SharesPermanentType => {
                    if let Some(surface) = self.additional_cost_object_surface() {
                        post_noun_qualifiers.push(format!(
                            "that shares a permanent type with {}",
                            surface.description()
                        ));
                    } else {
                        post_noun_qualifiers
                            .push("that shares a permanent type with it".to_string());
                    }
                }
                TaggedOpbjectRelation::AttachedToTaggedObject => {
                    post_noun_qualifiers.push("attached to it".to_string());
                }
                TaggedOpbjectRelation::SoulbondPartnerOfTagged => {
                    post_noun_qualifiers.push("paired with it".to_string());
                }
                TaggedOpbjectRelation::SameStableId => {}
            }
        }
        if !self.supertypes.is_empty() {
            for supertype in &self.supertypes {
                parts.push(supertype.name().to_string());
            }
        }
        if !self.excluded_card_types.is_empty() {
            for card_type in &self.excluded_card_types {
                parts.push(format!("non{}", describe_card_type_word(*card_type)));
            }
        }
        if !self.excluded_supertypes.is_empty() {
            for supertype in &self.excluded_supertypes {
                parts.push(format!("non{}", supertype.name()));
            }
        }
        if !self.excluded_subtypes.is_empty() {
            let mut remaining = self.excluded_subtypes.clone();
            let outlaw_pack = [
                Subtype::Assassin,
                Subtype::Mercenary,
                Subtype::Pirate,
                Subtype::Rogue,
                Subtype::Warlock,
            ];
            if outlaw_pack
                .iter()
                .all(|subtype| remaining.contains(subtype))
            {
                parts.push("non-outlaw".to_string());
                remaining.retain(|subtype| !outlaw_pack.contains(subtype));
            }
            for subtype in &remaining {
                parts.push(format!("non-{}", subtype.to_string().to_ascii_lowercase()));
            }
        }
        if !self.excluded_colors.is_empty() {
            if self.excluded_colors.contains(Color::White) {
                parts.push("nonwhite".to_string());
            }
            if self.excluded_colors.contains(Color::Blue) {
                parts.push("nonblue".to_string());
            }
            if self.excluded_colors.contains(Color::Black) {
                parts.push("nonblack".to_string());
            }
            if self.excluded_colors.contains(Color::Red) {
                parts.push("nonred".to_string());
            }
            if self.excluded_colors.contains(Color::Green) {
                parts.push("nongreen".to_string());
            }
        }
        if self.colorless {
            parts.push("colorless".to_string());
        }
        if self.multicolored {
            parts.push("multicolored".to_string());
        }
        if self.monocolored {
            parts.push("monocolored".to_string());
        }
        if let Some(all_colors) = self.all_colors {
            if all_colors {
                post_noun_qualifiers.push("that are all colors".to_string());
            } else {
                post_noun_qualifiers.push("that are not all colors".to_string());
            }
        }
        if let Some(exactly_two_colors) = self.exactly_two_colors {
            if exactly_two_colors {
                post_noun_qualifiers.push("that are exactly two colors".to_string());
            } else {
                post_noun_qualifiers.push("that are not exactly two colors".to_string());
            }
        }
        if self.historic {
            parts.push("historic".to_string());
        }
        if self.nonhistoric {
            post_noun_qualifiers.push("that's not historic".to_string());
        }
        if self.is_commander {
            parts.push("commander".to_string());
        }
        if self.noncommander {
            parts.push("noncommander".to_string());
        }
        if self.blocked && self.unblocked {
            parts.push("blocked/unblocked".to_string());
        } else {
            if self.blocked {
                parts.push("blocked".to_string());
            }
            if self.unblocked {
                parts.push("unblocked".to_string());
            }
        }
        if let Some(blocker) = &self.blocked_by {
            let blocker_text = match blocker {
                ObjectRef::Target => "target creature",
                ObjectRef::Specific(_) => "that creature",
                ObjectRef::Tagged(tag) if tag.as_str() == "blocking" => "the blocking creature",
                ObjectRef::Tagged(_) => "one of those creatures",
            };
            post_noun_qualifiers.push(format!("blocked by {blocker_text} this turn"));
        }
        if self.blocked_by_source {
            post_noun_qualifiers.push("blocked by this creature this turn".to_string());
        }
        if self.tapped && self.untapped {
            parts.push("tapped/untapped".to_string());
        } else if self.tapped {
            parts.push("tapped".to_string());
        } else if self.untapped {
            parts.push("untapped".to_string());
        }
        if self.attacking && self.blocking {
            parts.push("attacking/blocking".to_string());
        } else {
            if self.attacking {
                parts.push("attacking".to_string());
            }
            if self.blocking {
                parts.push("blocking".to_string());
            }
        }
        if self.attacked_this_turn {
            post_noun_qualifiers.push("that attacked this turn".to_string());
        }
        if let Some(player_filter) = &self.attacking_player_or_planeswalker_controlled_by {
            let player_text = player_filter.description();
            if self.attacking_player_only {
                post_noun_qualifiers.push(format!("attacking {player_text}"));
            } else {
                post_noun_qualifiers.push(format!(
                    "attacking {player_text} or a planeswalker controlled by {player_text}"
                ));
            }
        }
        if self.in_combat_with_source {
            post_noun_qualifiers.push("blocking or blocked by this creature".to_string());
        }
        if self.nonattacking && self.nonblocking {
            parts.push("nonattacking, nonblocking".to_string());
        } else {
            if self.nonattacking {
                parts.push("nonattacking".to_string());
            }
            if self.enlist_eligible {
                parts.push("enlist-eligible".to_string());
            }
            if self.nonblocking {
                parts.push("nonblocking".to_string());
            }
        }
        if self.entered_since_your_last_turn_ended {
            post_noun_qualifiers.push("that entered since your last turn ended".to_string());
        }
        if self.no_abilities {
            post_noun_qualifiers.push("with no abilities".to_string());
        }

        let subtype_implies_type = !self.subtypes.is_empty()
            && matches!(self.zone, None | Some(Zone::Battlefield))
            && self.all_card_types.is_empty()
            && self.card_types.is_empty();
        let has_all_permanent_types = {
            let required = [
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ];
            self.card_types.len() == required.len()
                && required
                    .iter()
                    .all(|card_type| self.card_types.contains(card_type))
        };
        let stack_source_ability = matches!(self.zone, Some(Zone::Stack))
            && matches!(
                self.stack_kind,
                Some(
                    StackObjectKind::Ability
                        | StackObjectKind::ActivatedAbility
                        | StackObjectKind::TriggeredAbility
                )
            );

        let mut type_phrase = if !self.all_card_types.is_empty() {
            Some((
                true,
                self.all_card_types
                    .iter()
                    .map(|t| t.name().to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            ))
        } else if !self.card_types.is_empty() {
            if stack_source_ability {
                let kind = self.stack_kind.unwrap_or(StackObjectKind::Ability);
                post_noun_qualifiers.push(format!(
                    "from {} source",
                    describe_card_type_source_phrase(&self.card_types, self.union_connective())
                ));
                Some((false, describe_stack_object_kind(kind).to_string()))
            } else if has_all_permanent_types {
                Some((true, "permanent".to_string()))
            } else {
                Some((
                    true,
                    describe_card_type_list(&self.card_types, self.union_connective()),
                ))
            }
        } else if !self.token && !subtype_implies_type {
            let default_noun = if self.source {
                match self.zone {
                    Some(Zone::Graveyard)
                    | Some(Zone::Hand)
                    | Some(Zone::Library)
                    | Some(Zone::Exile)
                    | Some(Zone::Command)
                    | Some(Zone::Ante)
                    | Some(Zone::OutsideGame) => "card",
                    _ => "source",
                }
            } else {
                match self.zone {
                    Some(Zone::Battlefield) | None => "permanent",
                    Some(Zone::Stack) => {
                        let kind = self.stack_kind.unwrap_or_else(|| {
                            if self.has_mana_cost {
                                StackObjectKind::Spell
                            } else {
                                StackObjectKind::SpellOrAbility
                            }
                        });
                        describe_stack_object_kind(kind)
                    }
                    Some(Zone::Graveyard)
                    | Some(Zone::Hand)
                    | Some(Zone::Library)
                    | Some(Zone::Exile)
                    | Some(Zone::Command)
                    | Some(Zone::Ante)
                    | Some(Zone::OutsideGame) => "card",
                }
            };
            Some((false, default_noun.to_string()))
        } else {
            None
        };

        let subtype_phrase = if !self.subtypes.is_empty() {
            let mut parts = Vec::new();
            let mut remaining = self.subtypes.clone();
            let outlaw_pack = [
                Subtype::Assassin,
                Subtype::Mercenary,
                Subtype::Pirate,
                Subtype::Rogue,
                Subtype::Warlock,
            ];
            if outlaw_pack
                .iter()
                .all(|subtype| remaining.contains(subtype))
            {
                parts.push("outlaw".to_string());
                remaining.retain(|subtype| !outlaw_pack.contains(subtype));
            }
            parts.extend(remaining.iter().map(std::string::ToString::to_string));
            Some(describe_filter_union_list(
                parts,
                self.union_connective(),
                false,
            ))
        } else {
            None
        };

        if self.has_explicit_card_noun() {
            match type_phrase.as_mut() {
                Some((true, phrase)) if !phrase.ends_with(" card") => {
                    phrase.push_str(" card");
                }
                Some((false, phrase)) if phrase == "permanent" => {
                    *phrase = "card".to_string();
                }
                None => {
                    type_phrase = Some((false, "card".to_string()));
                }
                _ => {}
            }
        }

        if let Some((type_is_card_type, phrase)) = type_phrase.as_mut()
            && *type_is_card_type
            && matches!(
                self.zone,
                Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
            )
            && !phrase.ends_with(" card")
        {
            phrase.push_str(" card");
        }
        if let Some((type_is_card_type, phrase)) = type_phrase.as_mut()
            && *type_is_card_type
            && matches!(self.zone, Some(Zone::Stack))
            && !phrase.ends_with(" spell")
        {
            phrase.push_str(" spell");
        }

        let creature_only = self.all_card_types.is_empty()
            && self.card_types.len() == 1
            && self.card_types[0] == CardType::Creature;
        let planeswalker_only = self.all_card_types.is_empty()
            && self.card_types.len() == 1
            && self.card_types[0] == CardType::Planeswalker;
        let land_only = self.all_card_types.is_empty()
            && self.card_types.len() == 1
            && self.card_types[0] == CardType::Land
            && !matches!(self.zone, Some(Zone::Stack));
        let source_surface_replaces_noun = self.source && source_surface_text.is_some();
        if source_surface_replaces_noun {
            // The parsed oracle surface already contains the self-reference noun
            // ("this Equipment", "this creature", a short card name, etc.).
        } else if self.type_or_subtype_union {
            match (type_phrase, subtype_phrase) {
                (Some((_, type_phrase)), Some(subtype_phrase)) => {
                    parts.push(describe_filter_union_list(
                        vec![type_phrase, subtype_phrase],
                        self.union_connective(),
                        false,
                    ));
                }
                (Some((_, type_phrase)), None) => parts.push(type_phrase),
                (None, Some(subtype_phrase)) => parts.push(subtype_phrase),
                (None, None) => {}
            }
        } else {
            match (type_phrase, subtype_phrase) {
                (Some((_, type_phrase)), Some(subtype_phrase))
                    if creature_only || planeswalker_only =>
                {
                    parts.push(subtype_phrase);
                    parts.push(type_phrase);
                }
                (Some((_, _type_phrase)), Some(subtype_phrase)) if land_only => {
                    parts.push(subtype_phrase);
                    if self.has_explicit_card_noun()
                        || matches!(
                            self.zone,
                            Some(
                                Zone::Graveyard
                                    | Zone::Hand
                                    | Zone::Library
                                    | Zone::Exile
                                    | Zone::Command
                                    | Zone::OutsideGame
                            )
                        )
                    {
                        parts.push("card".to_string());
                    }
                }
                (Some((type_is_card_type, type_phrase)), Some(subtype_phrase))
                    if !type_is_card_type && type_phrase == "card" =>
                {
                    parts.push(subtype_phrase);
                    parts.push(type_phrase);
                }
                (Some((_, type_phrase)), Some(subtype_phrase)) => {
                    parts.push(type_phrase);
                    parts.push(subtype_phrase);
                }
                (Some((_, type_phrase)), None) => parts.push(type_phrase),
                (None, Some(subtype_phrase)) => parts.push(subtype_phrase),
                (None, None) => {}
            }
        }
        if append_token_after_type {
            parts.push("token".to_string());
        }
        if !post_noun_qualifiers.is_empty() {
            parts.extend(post_noun_qualifiers);
        }
        if self.distinct_names {
            parts.push("with different names".to_string());
        }
        if self.distinct_powers {
            parts.push("with different powers".to_string());
        }
        if self.distinct_creature_types {
            parts.push("that share no creature types".to_string());
        }

        if let Some(ref name) = self.name {
            match (&controller_suffix, &owner_suffix) {
                (Some(controller), Some(owner)) => {
                    if controller == "you control" && owner == "you own" {
                        parts.push("you both own and control".to_string());
                    } else if controller == "you control" && owner == "you don't own" {
                        parts.push("you control but don't own".to_string());
                    } else if controller == "that player controls" && owner == "that player owns" {
                        parts.push("that player both owns and controls".to_string());
                    } else {
                        parts.push(controller.clone());
                        parts.push(owner.clone());
                    }
                }
                (Some(controller), None) => parts.push(controller.clone()),
                (None, Some(owner)) => parts.push(owner.clone()),
                (None, None) => {}
            }
            if name == "{chosen name}" {
                // The name is a runtime back-reference to a previously chosen
                // card name, not a literal card name.
                return format!("a {} with that name", parts.join(" "));
            }
            return format!("a {} named {}", parts.join(" "), name);
        }
        if let Some(ref name) = self.excluded_name {
            return format!("{} not named {}", parts.join(" "), name);
        }

        let has_power_or_toughness_qualifier = self.power.is_some()
            || self.toughness.is_some()
            || self.power_parity.is_some()
            || self.power_greater_than_base_power
            || self.power_toughness_relation.is_some()
            || self.power_relative_to_source.is_some()
            || self.total_power_toughness.is_some();
        let has_counter_qualifier = self.with_counter.is_some()
            || self.without_counter.is_some()
            || self.total_counters_parity.is_some();
        if let Some(scope) = explicit_extremum_scope(self) {
            if self.controller.is_some() && self.controller == scope.controller {
                controller_suffix = None;
            }
            if self.owner.is_some() && self.owner == scope.owner {
                owner_suffix = None;
            }
        }
        if (has_power_or_toughness_qualifier || has_counter_qualifier)
            && owner_suffix.is_none()
            && let Some(controller) = controller_suffix.take()
        {
            parts.push(controller);
        }

        if let (Some(power), Some(toughness)) = (&self.power, &self.toughness)
            && let (Comparison::Equal(power_value), Comparison::Equal(toughness_value)) =
                (power, toughness)
            && self.power_reference == self.toughness_reference
        {
            let label = match self.power_reference {
                PtReference::Effective => "power and toughness",
                PtReference::Base => "base power and toughness",
            };
            parts.push(format!("with {label} {power_value}/{toughness_value}"));
        } else {
            if let Some(ref power) = self.power {
                let label = match self.power_reference {
                    PtReference::Effective => "power",
                    PtReference::Base => "base power",
                };
                parts.push(
                    describe_extremum_filter_comparison(power, label)
                        .unwrap_or_else(|| format!("with {label} {}", describe_comparison(power))),
                );
            }
            if let Some(power_parity) = self.power_parity {
                let axis = match self.power_reference {
                    PtReference::Effective => "power",
                    PtReference::Base => "base power",
                };
                parts.push(power_parity.describe_axis(axis));
            }
            if self.power_greater_than_base_power {
                parts.push("with power greater than its base power".to_string());
            }
            if let Some(relation) = self.power_toughness_relation {
                match relation {
                    PowerToughnessRelation::PowerGreaterThanToughness => {
                        parts.push("with power greater than its toughness".to_string());
                    }
                    PowerToughnessRelation::ToughnessGreaterThanPower => {
                        parts.push("with toughness greater than its power".to_string());
                    }
                }
            }
            if let Some(relation) = self.power_relative_to_source {
                match relation {
                    SourcePowerRelation::LessThanSource => {
                        parts.push("with power less than this creature's power".to_string());
                    }
                }
            }
            if let Some(ref toughness) = self.toughness {
                let label = match self.toughness_reference {
                    PtReference::Effective => "toughness",
                    PtReference::Base => "base toughness",
                };
                parts.push(
                    describe_extremum_filter_comparison(toughness, label).unwrap_or_else(|| {
                        format!("with {label} {}", describe_comparison(toughness))
                    }),
                );
            }
        }
        if let Some(ref total_power_toughness) = self.total_power_toughness {
            parts.push(format!(
                "with total power and toughness {}",
                describe_comparison(total_power_toughness)
            ));
        }
        if let Some(ref mana_value) = self.mana_value {
            parts.push(
                describe_extremum_filter_comparison(mana_value, "mana value").unwrap_or_else(
                    || {
                        format!(
                            "with mana value {}",
                            describe_mana_value_comparison(mana_value)
                        )
                    },
                ),
            );
        }
        if let Some(ref color_count) = self.color_count {
            parts.push(format!(
                "with color count {}",
                describe_comparison(color_count)
            ));
        }
        if let Some(mana_value_parity) = self.mana_value_parity {
            parts.push(mana_value_parity.describe_axis("mana value"));
        }
        if let Some(counter_type) = self.mana_value_eq_counters_on_source {
            parts.push(format!(
                "with mana value equal to the number of {} counters on this artifact",
                counter_type.description()
            ));
        }
        if let Some(clause) = any_of_keyword_clause {
            parts.push(format!("with {clause}"));
        }
        for ability in &self.static_abilities {
            if let Some(label) = describe_filter_static_ability(*ability) {
                parts.push(format!("with {}", label));
            }
        }
        for marker in &self.ability_markers {
            parts.push(format!("with {}", marker.to_ascii_lowercase()));
        }
        for ability in &self.excluded_static_abilities {
            if let Some(label) = describe_filter_static_ability(*ability) {
                parts.push(format!("without {}", label));
            }
        }
        for marker in &self.excluded_ability_markers {
            parts.push(format!("without {}", marker.to_ascii_lowercase()));
        }
        if let Some(counter_requirement) = self.with_counter {
            let (one_or_more, plural_noun, plural_subject) = self.counter_requirement_surface();
            parts.push(format!(
                "with {}{} on {}",
                if one_or_more { "one or more " } else { "" },
                describe_counter_constraint(counter_requirement, plural_noun),
                if plural_subject { "them" } else { "it" }
            ));
        }
        if let Some(counter_exclusion) = self.without_counter {
            let (plural_noun, plural_subject) = self.counter_exclusion_surface();
            parts.push(format!(
                "without {} on {}",
                describe_counter_constraint(counter_exclusion, plural_noun),
                if plural_subject { "them" } else { "it" }
            ));
        }
        if let Some(total_counters_parity) = self.total_counters_parity {
            match total_counters_parity {
                ParityRequirement::Odd | ParityRequirement::Even => parts.push(format!(
                    "with an {} number of counters on it",
                    total_counters_parity.explicit_label().unwrap_or("")
                )),
                ParityRequirement::Chosen => {
                    parts.push("with a number of counters on it of the chosen quality".to_string())
                }
            }
        }
        if let Some(kind) = self.alternative_cast {
            parts.push(format!("with {}", describe_alternative_cast_kind(kind)));
        }
        if self.has_tap_activated_ability {
            parts.push("that has an activated ability with {T} in its cost".to_string());
        }

        let has_source_exiled_constraint = self.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == crate::SOURCE_EXILED_TAG
        });
        if let Some(zone) = self.zone {
            let zone_name = match zone {
                Zone::Battlefield => None,
                Zone::Graveyard => Some("graveyard"),
                Zone::Hand => Some("hand"),
                Zone::Library => Some("library"),
                Zone::Exile => Some("exile"),
                Zone::Stack => None,
                Zone::Command => Some("command zone"),
                Zone::Ante => Some("ante"),
                Zone::OutsideGame => Some("outside the game"),
            };
            if zone == Zone::Exile && has_source_exiled_constraint {
            } else if let Some(zone_name) = zone_name {
                if let Some(owner) = &self.owner {
                    parts.push(format!(
                        "in {} {}",
                        describe_possessive_player_filter(owner),
                        zone_name
                    ));
                } else if zone == Zone::Graveyard && self.single_graveyard {
                    parts.push("in single graveyard".to_string());
                } else if zone == Zone::Graveyard {
                    parts.push("in a graveyard".to_string());
                } else {
                    parts.push(format!("in {}", zone_name));
                }
            }
        }

        let has_entered_battlefield_this_turn_clause = (self.entered_battlefield_this_turn
            || self.entered_battlefield_controller.is_some())
            && self.zone == Some(Zone::Battlefield);
        if has_entered_battlefield_this_turn_clause
            && self.entered_battlefield_controller.is_none()
            && owner_suffix.is_none()
            && let Some(controller) = controller_suffix.take()
        {
            parts.push(controller);
        }

        if has_entered_battlefield_this_turn_clause {
            let clause = if let Some(controller) = &self.entered_battlefield_controller {
                match controller {
                    PlayerFilter::You => {
                        "that entered the battlefield under your control this turn".to_string()
                    }
                    PlayerFilter::Opponent => {
                        "that entered the battlefield under an opponent's control this turn"
                            .to_string()
                    }
                    PlayerFilter::Any => "that entered this turn".to_string(),
                    other => format!(
                        "that entered the battlefield under {} control this turn",
                        describe_possessive_player_filter(other)
                    ),
                }
            } else {
                "that entered this turn".to_string()
            };
            parts.push(clause);
        }

        if self.put_onto_battlefield_with_source {
            let source = self
                .put_onto_battlefield_with_source_surface
                .as_ref()
                .map(SourceReferenceSurface::display_text)
                .unwrap_or_else(|| "this permanent".to_string());
            parts.push(format!("put onto the battlefield with {source}"));
        }

        if self.created_with_source {
            let source = self
                .created_with_source_surface
                .as_ref()
                .map(SourceReferenceSurface::display_text)
                .unwrap_or_else(|| "this permanent".to_string());
            parts.push(format!("created with {source}"));
        }

        if self.entered_graveyard_from_battlefield_this_turn && self.zone == Some(Zone::Graveyard) {
            parts.push("that was put there from the battlefield this turn".to_string());
        } else if self.entered_graveyard_this_turn && self.zone == Some(Zone::Graveyard) {
            parts.push("that was put there from anywhere this turn".to_string());
        }
        if self.surveilled_this_turn {
            parts.push("you've surveilled this turn".to_string());
        }
        if let Some(player) = &self.discarded_or_cycled_this_turn_by {
            let actor = describe_player_filter(player);
            parts.push(format!("{actor} cycled or discarded this turn"));
        }

        if self.was_dealt_damage_this_turn {
            parts.push("that was dealt damage this turn".to_string());
        }
        if let Some(damager) = &self.dealt_damage_by_source_this_turn {
            let source = match damager {
                crate::DamagedBySource::ThisCreature => "this creature",
                crate::DamagedBySource::EquippedCreature => "equipped creature",
                crate::DamagedBySource::EnchantedCreature => "enchanted creature",
            };
            parts.push(format!("that was dealt damage by {source} this turn"));
        }
        if let Some(player) = &self.dealt_damage_to_player_this_turn {
            parts.push(format!(
                "that dealt damage to {} this turn",
                describe_player_filter(player)
            ));
        }
        if self.drawn_this_turn {
            parts.push("drawn this turn".to_string());
        }

        match (controller_suffix, owner_suffix) {
            (Some(controller), Some(owner))
                if controller == "you control" && owner == "you own" =>
            {
                parts.push("you both own and control".to_string());
            }
            (Some(controller), Some(owner))
                if controller == "you control" && owner == "you don't own" =>
            {
                parts.push("you control but don't own".to_string());
            }
            (Some(controller), Some(owner))
                if controller == "that player controls" && owner == "that player owns" =>
            {
                parts.push("that player both owns and controls".to_string());
            }
            (Some(controller), Some(owner)) => {
                parts.push(format!("{owner} but {controller}"));
            }
            (Some(controller), None) => parts.push(controller),
            (None, Some(owner)) => parts.push(owner),
            (None, None) => {}
        }

        if let Some(attached_to) = &self.attached_to_object {
            parts.push(format!(
                "attached to {}",
                ensure_indefinite_article(attached_to.description())
            ));
        }
        if let Some(attached_to_player) = &self.attached_to_player {
            parts.push(format!("attached to {}", attached_to_player.description()));
        }

        let mut appended_targeting_only = false;
        if self.targets_only_player.is_some() || self.targets_only_object.is_some() {
            let mut target_fragments = Vec::new();
            if let Some(player_filter) = &self.targets_only_player {
                let mut text = describe_player_filter(player_filter);
                if text != "you" {
                    text = ensure_indefinite_article(text);
                }
                target_fragments.push(text);
            }
            if let Some(object_filter) = &self.targets_only_object {
                let mut text = ensure_indefinite_article(object_filter.description());
                if let Some(count) = self.target_count
                    && count.is_single()
                    && (text.starts_with("a ") || text.starts_with("an "))
                {
                    if let Some(rest) = text.strip_prefix("a ") {
                        text = format!("a single {rest}");
                    } else if let Some(rest) = text.strip_prefix("an ") {
                        text = format!("a single {rest}");
                    }
                }
                target_fragments.push(text);
            }
            if !target_fragments.is_empty() {
                let target_text = if target_fragments.len() == 2 {
                    let joiner = if self.targets_only_any_of {
                        match self.union_connective() {
                            ObjectFilterUnionConnective::Or => "or",
                            ObjectFilterUnionConnective::AndOr => "and/or",
                        }
                    } else {
                        "and"
                    };
                    format!("{} {} {}", target_fragments[0], joiner, target_fragments[1])
                } else {
                    target_fragments[0].clone()
                };
                parts.push(format!("that targets only {target_text}"));
                appended_targeting_only = true;
            }
        }

        if let Some(count) = self.target_count
            && !appended_targeting_only
        {
            let phrase = if count.is_single() {
                Some("with a single target".to_string())
            } else if let Some(max) = count.max {
                if count.min == max {
                    Some(format!("with {} targets", max))
                } else if count.min == 0 {
                    Some(format!("with up to {} targets", max))
                } else {
                    Some(format!("with between {} and {} targets", count.min, max))
                }
            } else if count.min == 0 {
                Some("with any number of targets".to_string())
            } else {
                Some(format!("with at least {} targets", count.min))
            };
            if let Some(phrase) = phrase {
                parts.push(phrase);
            }
        }

        if !appended_targeting_only {
            let mut target_fragments = Vec::new();
            if let Some(player_filter) = &self.targets_player {
                let mut text = describe_player_filter(player_filter);
                if text != "you" {
                    text = ensure_indefinite_article(text);
                }
                target_fragments.push(text);
            }
            if let Some(object_filter) = &self.targets_object {
                target_fragments.push(ensure_indefinite_article(object_filter.description()));
            }
            if !target_fragments.is_empty() {
                let target_text = if target_fragments.len() == 2 {
                    let joiner = if self.targets_any_of {
                        match self.union_connective() {
                            ObjectFilterUnionConnective::Or => "or",
                            ObjectFilterUnionConnective::AndOr => "and/or",
                        }
                    } else {
                        "and"
                    };
                    format!("{} {} {}", target_fragments[0], joiner, target_fragments[1])
                } else {
                    target_fragments[0].clone()
                };
                parts.push(format!("that targets {target_text}"));
            }
        }

        if let Some(targetability) = &self.could_be_targeted_by {
            let stack_text = match &targetability.stack_object {
                ObjectRef::Target => "that spell",
                ObjectRef::Tagged(tag)
                    if matches!(tag.as_str(), "triggering" | "__it__" | "it") =>
                {
                    "that spell"
                }
                ObjectRef::Tagged(tag) if tag.as_str().contains("copied") => "the copy",
                ObjectRef::Tagged(_) | ObjectRef::Specific(_) => "that object",
            };
            parts.push(format!("{stack_text} could target"));
        }

        parts.join(" ")
    }
}

fn source_reference_surface_text(surface: &SourceReferenceSurface) -> String {
    surface.display_text()
}

/// An owner-scoped union of bare zones ("all cards from all opponents'
/// hands and graveyards") — the generic branch join would lose the owner
/// and mangle pluralization.
pub fn describe_owner_scoped_zone_union(filter: &ObjectFilter) -> Option<String> {
    if filter.any_of.len() != 2 || !matches!(filter.owner, Some(PlayerFilter::Opponent)) {
        return None;
    }
    let mut zones = Vec::with_capacity(filter.any_of.len());
    for branch in &filter.any_of {
        let zone = branch.zone?;
        let probe = ObjectFilter {
            zone: Some(zone),
            ..ObjectFilter::default()
        };
        if branch != &probe {
            return None;
        }
        zones.push(match zone {
            Zone::Hand => "hands",
            Zone::Graveyard => "graveyards",
            Zone::Library => "libraries",
            _ => return None,
        });
    }
    let outer_probe = ObjectFilter {
        owner: filter.owner.clone(),
        any_of: filter.any_of.clone(),
        union_surface: filter.union_surface.clone(),
        ..ObjectFilter::default()
    };
    if filter != &outer_probe {
        return None;
    }
    Some(format!(
        "cards from all opponents' {} and {}",
        zones[0], zones[1]
    ))
}

fn describe_branch_scoped_card_type_union(filter: &ObjectFilter) -> Option<String> {
    if filter.any_of.len() < 2 {
        return None;
    }

    let mut selectors = Vec::new();
    for branch in &filter.any_of {
        if !branch_scoped_union_arm_is_card_type_selector(branch) {
            return None;
        }
        selectors.push(branch.description());
    }

    let mut outer = filter.clone();
    outer.any_of.clear();
    if !outer.card_types.is_empty()
        || !outer.all_card_types.is_empty()
        || !outer.excluded_card_types.is_empty()
        || !outer.subtypes.is_empty()
        || outer.type_or_subtype_union
        || !outer.excluded_subtypes.is_empty()
        || !outer.supertypes.is_empty()
        || !outer.excluded_supertypes.is_empty()
        || outer.colors.is_some()
        || outer.required_colors.is_some()
        || !outer.excluded_colors.is_empty()
        || outer.colorless
        || outer.multicolored
        || outer.monocolored
    {
        return None;
    }

    let selector = describe_filter_union_list(selectors, filter.union_connective(), true);
    let placeholder = if outer.has_explicit_card_noun()
        || matches!(
            outer.zone,
            Some(
                Zone::Graveyard
                    | Zone::Hand
                    | Zone::Library
                    | Zone::Exile
                    | Zone::Command
                    | Zone::OutsideGame
            )
        ) {
        "card"
    } else if outer.zone == Some(Zone::Stack) {
        "spell"
    } else {
        "permanent"
    };
    let replacement = if placeholder == "permanent" || selector.ends_with(placeholder) {
        selector
    } else {
        format!("{selector} {placeholder}")
    };
    replace_first_description_word(&outer.description(), placeholder, &replacement)
}

fn branch_scoped_union_arm_is_card_type_selector(filter: &ObjectFilter) -> bool {
    if !filter.any_of.is_empty()
        || filter.card_types.len() != 1
        || !filter.all_card_types.is_empty()
    {
        return false;
    }

    let mut remainder = filter.clone();
    remainder.card_types.clear();
    remainder.excluded_card_types.clear();
    remainder.excluded_subtypes.clear();
    remainder.excluded_supertypes.clear();
    remainder.excluded_colors = ColorSet::new();
    remainder == ObjectFilter::default()
}

fn replace_first_description_word(text: &str, word: &str, replacement: &str) -> Option<String> {
    let start = text.match_indices(word).find_map(|(start, matched)| {
        let before_is_boundary =
            start == 0 || !text.as_bytes()[start.saturating_sub(1)].is_ascii_alphanumeric();
        let end = start + matched.len();
        let after_is_boundary = end == text.len() || !text.as_bytes()[end].is_ascii_alphanumeric();
        (before_is_boundary && after_is_boundary).then_some(start)
    })?;
    let end = start + word.len();
    Some(format!("{}{}{}", &text[..start], replacement, &text[end..]))
}

fn describe_simple_any_of_keyword_clause(
    any_of: &[ObjectFilter],
    connective: ObjectFilterUnionConnective,
) -> Option<String> {
    if any_of.len() < 2 {
        return None;
    }

    let mut labels = Vec::new();
    for filter in any_of {
        if !filter.any_of.is_empty() {
            return None;
        }

        let mut stripped = filter.clone();
        stripped.static_abilities.clear();
        stripped.excluded_static_abilities.clear();
        stripped.ability_markers.clear();
        stripped.excluded_ability_markers.clear();
        if stripped != ObjectFilter::default() {
            return None;
        }

        if filter.static_abilities.len() == 1 && filter.ability_markers.is_empty() {
            let label = describe_filter_static_ability(filter.static_abilities[0])?;
            labels.push(label.to_string());
            continue;
        }
        if filter.ability_markers.len() == 1 && filter.static_abilities.is_empty() {
            labels.push(filter.ability_markers[0].to_ascii_lowercase());
            continue;
        }

        return None;
    }

    Some(describe_filter_union_list(labels, connective, false))
}

fn describe_possessive_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Any => "a player's".to_string(),
        PlayerFilter::You => "your".to_string(),
        PlayerFilter::NotYou => "a non-you player's".to_string(),
        PlayerFilter::Opponent => "an opponent's".to_string(),
        PlayerFilter::Teammate => "a teammate's".to_string(),
        PlayerFilter::Active => "the active player's".to_string(),
        PlayerFilter::Defending => "the defending player's".to_string(),
        PlayerFilter::Attacking => "an attacking player's".to_string(),
        PlayerFilter::DamagedPlayer => "that player's".to_string(),
        PlayerFilter::EffectController => "the player who cast this spell's".to_string(),
        PlayerFilter::Specific(_) => "that player's".to_string(),
        PlayerFilter::MostLifeTied => "the chosen player's".to_string(),
        PlayerFilter::LowestLifeTied => "the chosen player's".to_string(),
        PlayerFilter::MostCardsInHand => "the player with the most cards in hand's".to_string(),
        PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
            "a player who cast one or more {} spells this turn's",
            card_type.to_string().to_ascii_lowercase()
        ),
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            let count_text = crate::cardinal_word(*count).unwrap_or_else(|| count.to_string());
            format!(
                "{} who has at least {count_text} more cards in hand than you do as you activate this ability's",
                describe_player_filter(base)
            )
        }
        PlayerFilter::HasMoreLifeThanYou { base } => {
            format!(
                "{} who has more life than you do as you activate this ability's",
                describe_player_filter(base)
            )
        }
        PlayerFilter::OpponentWithMoreControlledObjectsThan { player, filter } => format!(
            "an opponent of {} who controls more {} than they do's",
            describe_player_filter(player),
            pluralize_count_terminal_word(&filter.description())
        ),
        PlayerFilter::MaxSpeed {
            base,
            has_max_speed,
        } => {
            let verb = if *has_max_speed {
                "has max speed"
            } else {
                "doesn't have max speed"
            };
            format!("{} who {verb}'s", describe_player_filter(base))
        }
        PlayerFilter::ChosenPlayer => "the chosen player's".to_string(),
        PlayerFilter::TaggedPlayer(_) => "that player's".to_string(),
        PlayerFilter::IteratedPlayer => "that player's".to_string(),
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            "that player or that object's controller's".to_string()
        }
        PlayerFilter::Excluding { base, excluded } => format!(
            "{} other than {}",
            describe_possessive_player_filter(base),
            describe_possessive_player_filter(excluded)
        ),
        PlayerFilter::Target(inner) => {
            let base = match inner.as_ref() {
                PlayerFilter::Any => "target player".to_string(),
                other => format!("target {}", describe_player_filter(other)),
            };
            format!("{base}'s")
        }
        PlayerFilter::AliasedTarget(_) => "that player's".to_string(),
        PlayerFilter::ControllerOf(ObjectRef::Tagged(_) | ObjectRef::Target) => {
            "its controller's".to_string()
        }
        PlayerFilter::OwnerOf(ObjectRef::Tagged(_) | ObjectRef::Target) => {
            "its owner's".to_string()
        }
        PlayerFilter::ControllerOf(_) => "that object's controller's".to_string(),
        PlayerFilter::OwnerOf(_) => "that object's owner's".to_string(),
        PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
            "that player's".to_string()
        }
    }
}

pub(crate) fn describe_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Any => "player".to_string(),
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::NotYou => "player other than you".to_string(),
        PlayerFilter::Opponent => "opponent".to_string(),
        PlayerFilter::Teammate => "teammate".to_string(),
        PlayerFilter::Active => "active player".to_string(),
        PlayerFilter::Defending => "defending player".to_string(),
        PlayerFilter::Attacking => "attacking player".to_string(),
        PlayerFilter::DamagedPlayer => "that player".to_string(),
        PlayerFilter::EffectController => "the player who cast this spell".to_string(),
        PlayerFilter::Specific(_) => "player".to_string(),
        PlayerFilter::MostLifeTied => "player with the most life or tied for most life".to_string(),
        PlayerFilter::LowestLifeTied => {
            "player with the lowest life or tied for lowest life".to_string()
        }
        PlayerFilter::MostCardsInHand => "the player who has the most cards in hand".to_string(),
        PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
            "player who cast one or more {} spells this turn",
            card_type.to_string().to_ascii_lowercase()
        ),
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            let count_text = crate::cardinal_word(*count).unwrap_or_else(|| count.to_string());
            format!(
                "{} who has at least {count_text} more cards in hand than you do as you activate this ability",
                describe_player_filter(base)
            )
        }
        PlayerFilter::HasMoreLifeThanYou { base } => {
            format!(
                "{} who has more life than you do as you activate this ability",
                describe_player_filter(base)
            )
        }
        PlayerFilter::OpponentWithMoreControlledObjectsThan { player, filter } => format!(
            "opponent of {} who controls more {} than they do",
            describe_player_filter(player),
            pluralize_count_terminal_word(&filter.description())
        ),
        PlayerFilter::MaxSpeed {
            base,
            has_max_speed,
        } => {
            let verb = if *has_max_speed {
                "has max speed"
            } else {
                "doesn't have max speed"
            };
            format!("{} who {verb}", describe_player_filter(base))
        }
        PlayerFilter::ChosenPlayer => "chosen player".to_string(),
        PlayerFilter::TaggedPlayer(tag) if tag.as_str() == "enchanted" => {
            "enchanted player".to_string()
        }
        PlayerFilter::TaggedPlayer(_) => "that player".to_string(),
        PlayerFilter::IteratedPlayer => "that player".to_string(),
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            "that player or that object's controller".to_string()
        }
        PlayerFilter::Excluding { base, excluded } => format!(
            "{} other than {}",
            describe_player_filter(base),
            describe_player_filter(excluded)
        ),
        PlayerFilter::Target(inner) => format!("target {}", describe_player_filter(inner)),
        PlayerFilter::AliasedTarget(_) => "that player".to_string(),
        PlayerFilter::ControllerOf(_) => "controller".to_string(),
        PlayerFilter::OwnerOf(_) => "owner".to_string(),
        PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
            "that player".to_string()
        }
    }
}

fn describe_card_type_word(card_type: CardType) -> &'static str {
    card_type.name()
}

fn describe_card_type_list(
    card_types: &[CardType],
    connective: ObjectFilterUnionConnective,
) -> String {
    describe_filter_union_list(
        card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect(),
        connective,
        true,
    )
}

fn describe_card_type_source_phrase(
    card_types: &[CardType],
    connective: ObjectFilterUnionConnective,
) -> String {
    let types = describe_card_type_list(card_types, connective);
    if types.is_empty() {
        return "a source".to_string();
    }
    let article = if types
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {types}")
}

fn describe_stack_object_kind(kind: StackObjectKind) -> &'static str {
    match kind {
        StackObjectKind::Spell => "spell",
        StackObjectKind::Ability => "ability",
        StackObjectKind::ActivatedAbility => "activated ability",
        StackObjectKind::TriggeredAbility => "triggered ability",
        StackObjectKind::SpellOrAbility => "spell or ability",
    }
}

fn describe_counter_constraint(constraint: CounterConstraint, plural: bool) -> String {
    match constraint {
        CounterConstraint::Any if plural => "counters".to_string(),
        CounterConstraint::Any => "a counter".to_string(),
        CounterConstraint::Typed(counter_type) if plural => {
            format!("{} counters", counter_type.description())
        }
        CounterConstraint::Typed(counter_type) => {
            format!("a {} counter", counter_type.description())
        }
    }
}

fn describe_alternative_cast_kind(kind: AlternativeCastKind) -> &'static str {
    match kind {
        AlternativeCastKind::Blitz => "blitz",
        AlternativeCastKind::Dash => "dash",
        AlternativeCastKind::Flashback => "flashback",
        AlternativeCastKind::JumpStart => "jump-start",
        AlternativeCastKind::Escape => "escape",
        AlternativeCastKind::Madness => "madness",
        AlternativeCastKind::Miracle => "miracle",
        AlternativeCastKind::Suspend => "suspend",
    }
}

fn describe_filter_static_ability(ability_id: StaticAbilityId) -> Option<&'static str> {
    use StaticAbilityId::*;
    match ability_id {
        Flying => Some("flying"),
        FirstStrike => Some("first strike"),
        DoubleStrike => Some("double strike"),
        Deathtouch => Some("deathtouch"),
        Defender => Some("defender"),
        Flash => Some("flash"),
        Haste => Some("haste"),
        Hexproof => Some("hexproof"),
        Indestructible => Some("indestructible"),
        Intimidate => Some("intimidate"),
        Lifelink => Some("lifelink"),
        Menace => Some("menace"),
        Reach => Some("reach"),
        Skulk => Some("skulk"),
        Shroud => Some("shroud"),
        Trample => Some("trample"),
        Vigilance => Some("vigilance"),
        Fear => Some("fear"),
        Flanking => Some("flanking"),
        Landwalk => Some("landwalk"),
        Bloodthirst => Some("bloodthirst"),
        Morph => Some("morph"),
        Disguise => Some("disguise"),
        Megamorph => Some("megamorph"),
        Shadow => Some("shadow"),
        Horsemanship => Some("horsemanship"),
        Wither => Some("wither"),
        Infect => Some("infect"),
        Changeling => Some("changeling"),
        Cascade => Some("cascade"),
        _ => None,
    }
}

fn pluralize_count_terminal_word(phrase: &str) -> String {
    let (prefix, word) = phrase
        .rsplit_once(' ')
        .map_or(("", phrase), |(prefix, word)| (prefix, word));
    let lower = word.to_ascii_lowercase();
    let plural = match lower.as_str() {
        "plains" | "urzas" | "myr" | "merfolk" | "equipment" => word.to_string(),
        "elf" => "elves".to_string(),
        "dwarf" => "dwarves".to_string(),
        "wolf" => "wolves".to_string(),
        "werewolf" => "werewolves".to_string(),
        "mouse" => "mice".to_string(),
        _ if lower.ends_with('y')
            && lower.len() > 1
            && !matches!(
                lower.as_bytes().get(lower.len() - 2).copied(),
                Some(b'a' | b'e' | b'i' | b'o' | b'u')
            ) =>
        {
            format!("{}ies", &word[..word.len() - 1])
        }
        _ if lower.ends_with('s')
            || lower.ends_with('x')
            || lower.ends_with('z')
            || lower.ends_with("ch")
            || lower.ends_with("sh") =>
        {
            format!("{word}es")
        }
        _ => format!("{word}s"),
    };
    let plural = if word.chars().next().is_some_and(char::is_uppercase) {
        let mut chars = plural.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    } else {
        plural
    };
    if prefix.is_empty() {
        plural
    } else {
        format!("{prefix} {plural}")
    }
}

fn describe_count_filter_subject(filter: &ObjectFilter) -> String {
    let description = filter.description();
    let bare = description
        .strip_prefix("a ")
        .or_else(|| description.strip_prefix("an "))
        .unwrap_or(&description);
    let suffix_start = [
        " you control",
        " you don't control",
        " you own",
        " you don't own",
        " an opponent controls",
        " an opponent owns",
        " a player controls",
        " a player owns",
        " that player controls",
        " that player owns",
        " they control",
        " they own",
        " in ",
        " on ",
        " with ",
        " without ",
        " that ",
        " named ",
        " not named ",
        " attached to ",
    ]
    .iter()
    .filter_map(|marker| bare.find(marker))
    .min()
    .unwrap_or(bare.len());
    let (noun, suffix) = bare.split_at(suffix_start);
    format!("{}{}", pluralize_count_terminal_word(noun.trim()), suffix)
}

fn extremum_value_scope(value: &Value) -> Option<&ObjectFilter> {
    match value.unhinted() {
        Value::GreatestPower(scope)
        | Value::GreatestToughness(scope)
        | Value::GreatestManaValue(scope)
        | Value::LeastPower(scope)
        | Value::LeastToughness(scope)
        | Value::LeastManaValue(scope) => Some(scope),
        _ => None,
    }
}

fn explicit_extremum_scope(filter: &ObjectFilter) -> Option<&ObjectFilter> {
    [&filter.power, &filter.toughness, &filter.mana_value]
        .into_iter()
        .flatten()
        .find_map(|comparison| {
            let Comparison::EqualExpr(value) = comparison else {
                return None;
            };
            if value.has_surface_hint(crate::ValueSurfaceHint::ExtremumImplicitScope) {
                None
            } else {
                extremum_value_scope(value)
            }
        })
}

fn describe_extremum_scope(filter: &ObjectFilter) -> String {
    let is_tagged_set = filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject);
    if is_tagged_set && filter.has_explicit_card_noun() {
        "those cards".to_string()
    } else {
        describe_count_filter_subject(filter)
    }
}

fn describe_extremum_filter_comparison(
    comparison: &Comparison,
    characteristic: &str,
) -> Option<String> {
    let Comparison::EqualExpr(value) = comparison else {
        return None;
    };
    let (direction, value_characteristic, scope) = match value.unhinted() {
        Value::GreatestPower(scope) => ("greatest", "power", scope),
        Value::GreatestToughness(scope) => ("greatest", "toughness", scope),
        Value::GreatestManaValue(scope) => ("greatest", "mana value", scope),
        Value::LeastPower(scope) => ("least", "power", scope),
        Value::LeastToughness(scope) => ("lowest", "toughness", scope),
        Value::LeastManaValue(scope) => ("lowest", "mana value", scope),
        _ => return None,
    };
    if characteristic != value_characteristic {
        return None;
    }

    let mut description = format!("with the {direction} {characteristic}");
    if !value.has_surface_hint(crate::ValueSurfaceHint::ExtremumImplicitScope) {
        description.push_str(" among ");
        description.push_str(&describe_extremum_scope(scope));
    }
    if value.has_surface_hint(crate::ValueSurfaceHint::ExtremumTiedShort) {
        description.push_str(&format!(" or tied for {direction}"));
    } else if value.has_surface_hint(crate::ValueSurfaceHint::ExtremumTiedForCharacteristic) {
        description.push_str(&format!(" or tied for the {direction} {characteristic}"));
    }
    Some(description)
}

fn describe_counter_holder(spec: &ChooseSpec) -> String {
    if let Some(surface) = spec.source_reference_surface() {
        return surface.display_text();
    }
    match spec.base() {
        ChooseSpec::Source => "this creature".to_string(),
        ChooseSpec::Target(_) => "that creature".to_string(),
        ChooseSpec::Tagged(tag) if tag.as_str() == "triggering" => "that spell".to_string(),
        ChooseSpec::Tagged(_) | ChooseSpec::Iterated => "it".to_string(),
        ChooseSpec::All(filter) => describe_count_filter_subject(filter),
        _ => "that object".to_string(),
    }
}

fn describe_comparison(cmp: &Comparison) -> String {
    fn describe_value_expr(value: &Value) -> String {
        match value {
            Value::SurfaceHinted { value: _, hints }
                if hints.contains(&crate::ValueSurfaceHint::WhereXIs) =>
            {
                "X".to_string()
            }
            Value::SurfaceHinted { value, hints } => {
                if hints.contains(&crate::ValueSurfaceHint::RevealedCardReference)
                    && matches!(value.unhinted(), Value::ManaValueOf(_))
                {
                    return "the revealed card's mana value".to_string();
                }
                if hints.contains(&crate::ValueSurfaceHint::CountersAmong)
                    && let Value::CountersOn(spec, counter_type) = value.unhinted()
                    && let ChooseSpec::All(filter) = spec.unhinted()
                {
                    let subject = describe_count_filter_subject(filter);
                    return match counter_type {
                        Some(counter_type) => format!(
                            "the number of {} counters among {subject}",
                            counter_type.description()
                        ),
                        None => format!("the number of counters among {subject}"),
                    };
                }
                if let Some(kind) = hints.iter().find_map(|hint| match hint {
                    crate::ValueSurfaceHint::SacrificedObject(kind) => Some(*kind),
                    _ => None,
                }) {
                    let characteristic = match value.unhinted() {
                        Value::PowerOf(_) => Some("power"),
                        Value::ToughnessOf(_) => Some("toughness"),
                        Value::ManaValueOf(_) => Some("mana value"),
                        _ => None,
                    };
                    if let Some(characteristic) = characteristic {
                        return format!("the sacrificed {}'s {characteristic}", kind.noun());
                    }
                }
                describe_value_expr(value)
            }
            Value::Fixed(v) => v.to_string(),
            Value::X => "X".to_string(),
            Value::Count(filter) => {
                format!("the number of {}", describe_count_filter_subject(filter))
            }
            Value::CountScaled(filter, factor) => {
                format!(
                    "{factor} times the number of {}",
                    describe_count_filter_subject(filter)
                )
            }
            Value::DividedRoundedDown(value, divisor) => {
                format!(
                    "{} divided by {divisor}, rounded down",
                    describe_value_expr(value)
                )
            }
            Value::LandsEnteredBattlefieldThisTurn(player) => {
                format!(
                    "the number of lands that entered the battlefield under {} control this turn",
                    describe_possessive_player_filter(player)
                )
            }
            Value::ColorsAmong(filter) => {
                format!("the number of colors among {}", filter.description())
            }
            Value::CardTypesAmong(filter) => {
                format!("the number of card types among {}", filter.description())
            }
            Value::GreatestPower(filter) => {
                format!(
                    "the greatest power among {}",
                    describe_count_filter_subject(filter)
                )
            }
            Value::GreatestToughness(filter) => format!(
                "the greatest toughness among {}",
                describe_count_filter_subject(filter)
            ),
            Value::GreatestManaValue(filter) => format!(
                "the greatest mana value among {}",
                describe_count_filter_subject(filter)
            ),
            Value::LeastPower(filter) => {
                format!(
                    "the least power among {}",
                    describe_count_filter_subject(filter)
                )
            }
            Value::LeastToughness(filter) => format!(
                "the lowest toughness among {}",
                describe_count_filter_subject(filter)
            ),
            Value::LeastManaValue(filter) => format!(
                "the lowest mana value among {}",
                describe_count_filter_subject(filter)
            ),
            Value::StaticAbilitiesAmong { filter, abilities } => {
                let names = abilities
                    .iter()
                    .map(|ability| format!("{ability:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "the number of abilities from among {names} among {}",
                    filter.description()
                )
            }
            Value::CreatureTypesAmong(filter) => {
                format!(
                    "the number of creature types among {}",
                    filter.description()
                )
            }
            Value::LifeTotal(player) => {
                format!("{} life total", describe_possessive_player_filter(player))
            }
            Value::LifeTotalAsTurnBegan(player) => {
                format!(
                    "{} life total as the turn began",
                    describe_possessive_player_filter(player)
                )
            }
            Value::LifeTotalDifference(player) => {
                format!("difference between {player:?} players' life totals")
            }
            Value::Speed(player) => format!("{player:?}'s speed"),
            Value::StartingLifeTotal(player) => format!("{player:?}'s starting life total"),
            Value::LastNotedLifeTotal => "last noted life total".to_string(),
            Value::ThisAbilityResolvedThisTurnCount => {
                "the number of times this ability has resolved this turn".to_string()
            }
            Value::CountersOnSource(counter_type) => {
                format!(
                    "the number of {} counters on it",
                    counter_type.description()
                )
            }
            Value::CountersOn(spec, Some(counter_type)) => {
                format!(
                    "the number of {} counters on {}",
                    counter_type.description(),
                    describe_counter_holder(spec)
                )
            }
            Value::CountersOn(spec, None) => {
                format!(
                    "the number of counters on {}",
                    describe_counter_holder(spec)
                )
            }
            Value::SourcePower => "this creature's power".to_string(),
            Value::SourceToughness => "this creature's toughness".to_string(),
            Value::PowerOf(spec) => {
                format!("{} power", describe_value_choose_spec_possessive(spec))
            }
            Value::ToughnessOf(spec) => {
                format!("{} toughness", describe_value_choose_spec_possessive(spec))
            }
            Value::ManaValueOf(spec) => {
                if let ChooseSpec::Tagged(tag) = spec.base() {
                    if tag.as_str() == "triggering" {
                        return "that spell's mana value".to_string();
                    }
                    if tag.as_str() == crate::SOURCE_EXILED_TAG {
                        return "the exiled spell's mana value".to_string();
                    }
                }
                if spec.source_reference_surface().is_some() {
                    format!("{} mana value", describe_value_choose_spec_possessive(spec))
                } else {
                    "that card's mana value".to_string()
                }
            }
            Value::UnspentMana(player) => {
                let subject = player.description();
                let verb = if matches!(player, PlayerFilter::You) {
                    "have"
                } else {
                    "has"
                };
                format!("the amount of unspent mana {subject} {verb}")
            }
            Value::Add(left, right) => {
                format!(
                    "{} plus {}",
                    describe_value_expr(left),
                    describe_value_expr(right)
                )
            }
            Value::EventValue(EventValueSpec::Amount) => "that damage".to_string(),
            Value::EventValue(EventValueSpec::LifeAmount) => "that much life".to_string(),
            Value::EventValue(EventValueSpec::BlockersBeyondFirst { .. }) => {
                "a dynamic blocker count".to_string()
            }
            Value::EffectValue(_) => "that result".to_string(),
            Value::ColorsOfManaSpentToCastThisSpell => {
                "the number of colors of mana spent to cast this spell".to_string()
            }
            Value::ManaFromSourceSpentToCastThisSpell {
                source_filter,
                include_source_noun,
            } => {
                let mut source = source_filter.description();
                if *include_source_noun {
                    source.push_str(" source");
                }
                format!("the amount of mana from {source} spent to cast this spell")
            }
            Value::EffectMetric {
                metric: EffectMetric::OtherNumber,
                ..
            } => "the other result".to_string(),
            _ => "a dynamic value".to_string(),
        }
    }

    let describe_values = |values: &[i32]| -> String {
        match values.len() {
            0 => String::new(),
            1 => values[0].to_string(),
            2 => format!("{} or {}", values[0], values[1]),
            _ => {
                let head = values[..values.len() - 1]
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{head}, or {}", values[values.len() - 1])
            }
        }
    };
    match cmp {
        Comparison::Equal(v) => format!("{v}"),
        Comparison::OneOf(values) => describe_values(values),
        Comparison::NotEqual(v) => format!("not equal to {v}"),
        Comparison::LessThan(v) => format!("less than {v}"),
        Comparison::LessThanOrEqual(v) => format!("{v} or less"),
        Comparison::GreaterThan(v) => format!("greater than {v}"),
        Comparison::GreaterThanOrEqual(v) => format!("{v} or greater"),
        Comparison::EqualExpr(value) => format!("equal to {}", describe_value_expr(value)),
        Comparison::NotEqualExpr(value) => {
            format!("not equal to {}", describe_value_expr(value))
        }
        Comparison::LessThanExpr(value) => format!("less than {}", describe_value_expr(value)),
        Comparison::LessThanOrEqualExpr(value) => {
            if value.has_surface_hint(crate::ValueSurfaceHint::ExplicitComparison)
                || matches!(value.unhinted(), Value::CountersOnSource(_))
            {
                format!("less than or equal to {}", describe_value_expr(value))
            } else {
                format!("{} or less", describe_value_expr(value))
            }
        }
        Comparison::GreaterThanExpr(value) => {
            format!("greater than {}", describe_value_expr(value))
        }
        Comparison::GreaterThanOrEqualExpr(value) => {
            if value.has_surface_hint(crate::ValueSurfaceHint::ExplicitComparison)
                || matches!(value.unhinted(), Value::CountersOnSource(_))
            {
                format!("greater than or equal to {}", describe_value_expr(value))
            } else {
                format!("{} or greater", describe_value_expr(value))
            }
        }
    }
}

/// Exact mana-value filters conventionally omit "equal to" before X
/// ("a spell with mana value X"), while other dynamic expressions retain the
/// explicit comparator. Keep this axis-specific so power, toughness, and
/// other comparisons continue to render their required relation.
fn describe_mana_value_comparison(cmp: &Comparison) -> String {
    match cmp {
        Comparison::EqualExpr(value) if matches!(value.unhinted(), Value::X) => "X".to_string(),
        _ => describe_comparison(cmp),
    }
}

fn describe_value_choose_spec_possessive(spec: &ChooseSpec) -> String {
    if let Some(kind) = spec.sacrificed_object_kind() {
        return format!("the sacrificed {}'s", kind.noun());
    }
    if let Some(surface) = spec.source_reference_surface() {
        let subject = surface.display_text();
        return if subject.ends_with('s') {
            format!("{subject}'")
        } else {
            format!("{subject}'s")
        };
    }
    let subject = match spec.base() {
        ChooseSpec::Tagged(tag) if tag.as_str() == crate::EXPLOITED_TAG => {
            "the exploited creature".to_string()
        }
        ChooseSpec::Tagged(_) => "it".to_string(),
        ChooseSpec::Source => "this creature".to_string(),
        ChooseSpec::Target(_) => "that creature".to_string(),
        _ => "it".to_string(),
    };
    if subject == "it" {
        "its".to_string()
    } else if subject.ends_with('s') {
        format!("{subject}'")
    } else {
        format!("{subject}'s")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Comparison, ObjectFilter, ObjectFilterUnionConnective, ObjectRef, ParityRequirement,
        PlayerFilter, PtReference, StackObjectKind, TaggedObjectConstraint, TaggedOpbjectRelation,
        describe_comparison, describe_mana_value_comparison,
    };
    use crate::{CardType, CounterType, ObjectId, Subtype, TagKey, Value, ValueSurfaceHint, Zone};

    #[test]
    fn object_ref_helpers_preserve_payloads() {
        assert_eq!(
            ObjectRef::tagged("destroyed"),
            ObjectRef::Tagged(TagKey::from("destroyed"))
        );
        assert_eq!(
            ObjectRef::specific(ObjectId::from_raw(7)),
            ObjectRef::Specific(ObjectId::from_raw(7))
        );
    }

    #[test]
    fn player_filter_builders_and_descriptions_are_stable() {
        assert_eq!(
            PlayerFilter::target_player().description(),
            "target a player"
        );
        assert_eq!(
            PlayerFilter::target_opponent().description(),
            "target an opponent"
        );
        assert_eq!(
            PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::Defending).description(),
            "an opponent other than the defending player"
        );
        assert_eq!(
            PlayerFilter::CastCardTypeThisTurn(CardType::Artifact).description(),
            "a player who cast one or more artifact spells this turn"
        );
    }

    #[test]
    fn iterated_player_detection_only_flags_dynamic_variants() {
        assert!(PlayerFilter::IteratedPlayer.mentions_iterated_player());
        assert!(PlayerFilter::target_player().mentions_iterated_player() == false);
        assert!(
            PlayerFilter::excluding(PlayerFilter::IteratedPlayer, PlayerFilter::Opponent)
                .mentions_iterated_player()
        );
    }

    #[test]
    fn filter_support_types_keep_expected_defaults() {
        assert_eq!(PtReference::default(), PtReference::Effective);
        assert_eq!(ObjectRef::default(), ObjectRef::Target);
        assert_eq!(
            TaggedObjectConstraint {
                tag: TagKey::from("chosen"),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            }
            .relation,
            TaggedOpbjectRelation::IsTaggedObject
        );
        assert_eq!(
            StackObjectKind::SpellOrAbility,
            StackObjectKind::SpellOrAbility
        );
    }

    #[test]
    fn parity_requirement_formats_axes() {
        assert_eq!(ParityRequirement::Odd.explicit_label(), Some("odd"));
        assert_eq!(
            ParityRequirement::Even.describe_axis("mana value"),
            "with even mana value"
        );
        assert_eq!(
            ParityRequirement::Chosen.describe_axis("power"),
            "with power of the chosen quality"
        );
    }

    #[test]
    fn exact_x_mana_value_uses_canonical_surface() {
        assert_eq!(
            describe_mana_value_comparison(&Comparison::EqualExpr(Box::new(Value::X))),
            "X"
        );
        assert_eq!(
            describe_mana_value_comparison(&Comparison::EqualExpr(Box::new(Value::Fixed(2)))),
            "equal to 2"
        );
    }

    #[test]
    fn dynamic_comparison_surface_and_value_subject_are_preserved() {
        let life_total = Value::LifeTotal(PlayerFilter::You)
            .with_surface_hint(ValueSurfaceHint::ExplicitComparison);
        assert_eq!(
            describe_comparison(&Comparison::GreaterThanOrEqualExpr(Box::new(life_total))),
            "greater than or equal to your life total"
        );

        let vampires = Value::Count(
            ObjectFilter::default()
                .with_subtype(Subtype::Vampire)
                .you_control(),
        )
        .with_surface_hint(ValueSurfaceHint::ExplicitComparison);
        assert_eq!(
            describe_comparison(&Comparison::LessThanOrEqualExpr(Box::new(vampires))),
            "less than or equal to the number of Vampires you control"
        );

        let postfix = Value::LifeTotal(PlayerFilter::You);
        assert_eq!(
            describe_comparison(&Comparison::LessThanOrEqualExpr(Box::new(postfix))),
            "your life total or less"
        );
    }

    #[test]
    fn revealed_card_reference_survives_dynamic_filter_comparison_rendering() {
        let mana_value = Value::ManaValueOf(Box::new(crate::ChooseSpec::Tagged(TagKey::from(
            "__public_revealed",
        ))))
        .with_surface_hint(ValueSurfaceHint::RevealedCardReference);

        assert_eq!(
            describe_comparison(&Comparison::LessThanExpr(Box::new(mana_value))),
            "less than the revealed card's mana value"
        );
    }

    #[test]
    fn and_or_union_surface_renders_without_changing_filter_equality() {
        let semantic_filter = ObjectFilter::default()
            .with_type(CardType::Artifact)
            .with_type(CardType::Creature);
        let surfaced_filter = semantic_filter
            .clone()
            .with_union_connective(ObjectFilterUnionConnective::AndOr);

        assert_eq!(semantic_filter, surfaced_filter);
        assert_eq!(semantic_filter.description(), "artifact or creature");
        assert_eq!(surfaced_filter.description(), "artifact and/or creature");
        assert_eq!(
            surfaced_filter.union_connective(),
            ObjectFilterUnionConnective::AndOr
        );
    }

    #[test]
    fn plural_counter_surface_renders_without_changing_filter_equality() {
        let semantic_filter = ObjectFilter::default()
            .with_type(CardType::Creature)
            .you_control()
            .with_counter_type(CounterType::PlusOnePlusOne);
        let mut surfaced_filter = semantic_filter.clone();
        surfaced_filter.set_counter_requirement_surface(false, true, true);

        assert_eq!(semantic_filter, surfaced_filter);
        assert_eq!(
            semantic_filter.description(),
            "a creature you control with a +1/+1 counter on it"
        );
        assert_eq!(
            surfaced_filter.description(),
            "a creature you control with +1/+1 counters on them"
        );

        let mut any_counter = ObjectFilter::permanent().with_any_counter();
        any_counter.set_counter_requirement_surface(false, true, true);
        assert_eq!(any_counter.description(), "permanent with counters on them");
    }

    #[test]
    fn one_or_more_counter_surface_renders_without_changing_filter_equality() {
        let semantic_filter = ObjectFilter::planeswalker().with_counter_type(CounterType::Loyalty);
        let mut one_or_more = semantic_filter.clone();
        one_or_more.set_counter_requirement_surface(true, true, false);

        assert_eq!(semantic_filter, one_or_more);
        assert_eq!(
            one_or_more.description(),
            "planeswalker with one or more loyalty counters on it"
        );
    }

    #[test]
    fn branch_scoped_type_union_factors_shared_card_domain() {
        let mut filter = ObjectFilter {
            zone: Some(Zone::Graveyard),
            owner: Some(PlayerFilter::You),
            any_of: vec![
                ObjectFilter::default().with_type(CardType::Artifact),
                ObjectFilter {
                    card_types: vec![CardType::Enchantment],
                    excluded_subtypes: vec![Subtype::Aura],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        filter.set_explicit_card_noun(true);

        assert_eq!(
            filter.description(),
            "artifact or non-aura enchantment card in your graveyard"
        );
    }

    #[test]
    fn branch_scoped_type_union_does_not_repeat_a_shared_card_noun() {
        let mut creature_card = ObjectFilter::default().with_type(CardType::Creature);
        creature_card.set_explicit_card_noun(true);
        let filter = ObjectFilter {
            zone: Some(Zone::Library),
            owner: Some(PlayerFilter::You),
            any_of: vec![
                ObjectFilter::default().with_type(CardType::Artifact),
                creature_card,
            ],
            ..Default::default()
        }
        .with_union_connective(ObjectFilterUnionConnective::AndOr);

        assert_eq!(
            filter.description(),
            "artifact and/or creature card in your library"
        );
    }

    #[test]
    fn branch_scoped_type_union_factors_shared_battlefield_domain() {
        let filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            any_of: vec![
                ObjectFilter::default().with_type(CardType::Artifact),
                ObjectFilter {
                    card_types: vec![CardType::Enchantment],
                    excluded_subtypes: vec![Subtype::Aura],
                    ..Default::default()
                },
                ObjectFilter::default().with_type(CardType::Land),
            ],
            ..Default::default()
        };

        assert_eq!(
            filter.description(),
            "artifact, non-aura enchantment, or land"
        );
    }

    #[test]
    fn explicit_card_noun_survives_a_cleared_zone_without_changing_filter_equality() {
        let semantic_filter = ObjectFilter::creature();
        let mut surfaced_filter = semantic_filter.clone();
        surfaced_filter.set_explicit_card_noun(true);

        assert_eq!(semantic_filter, surfaced_filter);
        assert_eq!(semantic_filter.description(), "creature");
        assert_eq!(surfaced_filter.description(), "creature card");

        let mut untyped_card = ObjectFilter::default().nontoken();
        untyped_card.set_explicit_card_noun(true);
        assert_eq!(untyped_card.description(), "card");
    }

    #[test]
    fn original_printing_set_predicate_renders_as_a_typed_qualifier() {
        let filter = ObjectFilter {
            card_types: vec![CardType::Artifact],
            nontoken: true,
            name_originally_printed_in_set: Some("Antiquities".to_string()),
            ..Default::default()
        };

        assert!(filter.uses_non_pt_battlefield_characteristics());
        assert_eq!(
            filter.description(),
            "nontoken artifact with a name originally printed in the Antiquities expansion"
        );
    }
}
