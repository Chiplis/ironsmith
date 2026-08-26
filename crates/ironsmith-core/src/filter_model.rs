use crate::{
    CardType, ChoiceAggregateConstraint, ChoiceCount, ChooseSpec, Color, ColorSet, CounterType,
    EffectMetric, KeywordActionKind, ManaCost, ManaSymbol, ObjectId, PlayerId, PriorEffectAction,
    SourceReferenceSurface, StaticAbilityId, Subtype, SubtypeFamily, Supertype, TagKey, Value,
    Zone, effect_model::EventValueSpec,
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

fn correct_leading_indefinite_article(text: String) -> String {
    let Some(rest) = text.strip_prefix("a ") else {
        return text;
    };
    if rest
        .chars()
        .next()
        .is_some_and(|first| matches!(first.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        format!("an {rest}")
    } else {
        text
    }
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

/// Rejoin an exact-mana-cost union that shares every other characteristic.
/// `{1}` is an executable printed-cost predicate, not a mana-value spelling,
/// so this compaction is safe only when the typed branch costs are the sole
/// branch difference.
fn describe_exact_mana_cost_union(filter: &ObjectFilter) -> Option<String> {
    if filter.any_of.len() < 2
        || filter.has_conjunctive_set_surface()
        || filter.union_connective() != ObjectFilterUnionConnective::Or
    {
        return None;
    }

    let mut outer_remainder = filter.clone();
    outer_remainder.any_of.clear();
    outer_remainder.zone = None;
    outer_remainder.controller = None;
    outer_remainder.owner = None;
    outer_remainder.other = false;
    outer_remainder.union_surface = ObjectFilterUnionSurface::default();
    if outer_remainder != ObjectFilter::default() {
        return None;
    }

    let mut common_base = None;
    let mut costs = Vec::with_capacity(filter.any_of.len());
    for branch in &filter.any_of {
        if !branch.any_of.is_empty() {
            return None;
        }
        let mut base = branch.clone();
        let cost = base.exact_mana_cost.take()?;
        if let Some(expected) = common_base.as_ref() {
            if &base != expected {
                return None;
            }
        } else {
            common_base = Some(base);
        }
        costs.push(cost.to_oracle());
    }

    let mut base = common_base?;
    if base.controller.is_none() {
        base.controller = filter.controller.clone();
    }
    if base.owner.is_none() {
        base.owner = filter.owner.clone();
    }
    if filter.other {
        base.other = true;
    }
    Some(format!(
        "{} with mana cost {}",
        base.description(),
        describe_filter_union_list(costs, ObjectFilterUnionConnective::Or, false)
    ))
}

fn describe_conjunctive_filter_list(mut parts: Vec<String>) -> String {
    match parts.as_slice() {
        [] => return String::new(),
        [single] => return single.clone(),
        [first, second] => return format!("{first} and {second}"),
        _ => {}
    }
    let last = parts
        .pop()
        .expect("conjunctive list has at least three parts");
    format!("{}, and {last}", parts.join(", "))
}

/// A reference to an object for use in filters and effects.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// Authored source noun in a chosen-name relationship such as "a name chosen
/// for this enchantment."
///
/// The tagged same-name constraint remains the executable relationship. This
/// equality-transparent value only preserves which source noun Oracle used.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChosenNameSourceSurface {
    Artifact,
    Card,
    Creature,
    Enchantment,
    Permanent,
    Source,
}

impl ChosenNameSourceSurface {
    pub fn from_noun(noun: &str) -> Option<Self> {
        match noun {
            "artifact" => Some(Self::Artifact),
            "card" => Some(Self::Card),
            "creature" => Some(Self::Creature),
            "enchantment" => Some(Self::Enchantment),
            "permanent" => Some(Self::Permanent),
            "source" => Some(Self::Source),
            _ => None,
        }
    }

    pub const fn phrase(self) -> &'static str {
        match self {
            Self::Artifact => "this artifact",
            Self::Card => "this card",
            Self::Creature => "this creature",
            Self::Enchantment => "this enchantment",
            Self::Permanent => "this permanent",
            Self::Source => "this source",
        }
    }
}

/// Authored noun in a demonstrative condition subject such as "that land."
///
/// Tagged-object identity remains the runtime source of truth. This value is
/// equality-transparent presentation metadata retained on the comparison
/// filter so compiled text does not collapse an explicit antecedent to "it."
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemonstrativeAntecedentSurface {
    Artifact,
    Card,
    Creature,
    Enchantment,
    Land,
    Object,
    Permanent,
    Source,
    Spell,
    Token,
}

impl DemonstrativeAntecedentSurface {
    pub fn from_noun(noun: &str) -> Option<Self> {
        match noun {
            "artifact" => Some(Self::Artifact),
            "card" => Some(Self::Card),
            "creature" | "creatures" => Some(Self::Creature),
            "enchantment" => Some(Self::Enchantment),
            "land" => Some(Self::Land),
            "object" => Some(Self::Object),
            "permanent" => Some(Self::Permanent),
            "source" => Some(Self::Source),
            "spell" => Some(Self::Spell),
            "token" => Some(Self::Token),
            _ => None,
        }
    }

    pub const fn phrase(self) -> &'static str {
        match self {
            Self::Artifact => "that artifact",
            Self::Card => "that card",
            Self::Creature => "that creature",
            Self::Enchantment => "that enchantment",
            Self::Land => "that land",
            Self::Object => "that object",
            Self::Permanent => "that permanent",
            Self::Source => "that source",
            Self::Spell => "that spell",
            Self::Token => "that token",
        }
    }
}

/// Oracle-facing action used when a filter refers back to the object paid as
/// an additional cost. Object identity remains a tagged runtime relation;
/// this value only preserves whether the authored noun was sacrificed or
/// exiled.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// Authored opponent quantifier in a "played by ..." entry restriction.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayedByOpponentSurface {
    YourOpponents,
    AnOpponent,
    Opponents,
}

impl PlayedByOpponentSurface {
    pub const fn description(self) -> &'static str {
        match self {
            Self::YourOpponents => "your opponents",
            Self::AnOpponent => "an opponent",
            Self::Opponents => "opponents",
        }
    }
}

/// Oracle wording for an object's current-turn graveyard-entry history.
///
/// The `entered_graveyard_*` flags remain the executable semantics. This
/// equality-transparent surface distinguishes the broad canonical wording
/// "put there this turn" from an explicitly authored "from anywhere" clause.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraveyardEntryHistorySurface {
    PutThereThisTurn,
    PutThereFromAnywhereThisTurn,
    PutThereFromBattlefieldThisTurn,
}

/// Oracle-facing domain for a characteristic rule whose executable filter is
/// intentionally global. The filter still selects objects by its ordinary
/// semantic fields; this distinguishes the authored object categories that
/// jointly cover that global set.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalCharacteristicDomainSurface {
    CardsOutsideBattlefieldSpellsAndPermanents,
}

/// Presentation metadata for an [`ObjectFilter`].
///
/// `PartialEq` is intentionally semantic-transparent: `ObjectFilter` derives
/// equality and is used throughout lowering, deduplication, and runtime shape
/// checks. Oracle-only spelling choices must therefore compare equal to the
/// same runtime filter rendered with a canonical surface.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectFilterUnionSurface {
    connective: ObjectFilterUnionConnective,
    /// Oracle placed subtype arms before card-type arms in a mixed
    /// type-or-subtype union (for example, "Vehicles and/or creatures").
    /// Matching is order-independent; this preserves only authored order.
    subtype_before_card_type: bool,
    /// Oracle placed a shared `card` or `spell` noun after every member of a
    /// mixed union, as in "creature or Aura spell".
    terminal_noun_after_type_subtype_union: bool,
    /// Oracle used `and` to describe the members of one inclusive object set,
    /// as in "Plants and Treefolk you control".
    conjunctive_set_surface: bool,
    /// Oracle used a serial-comma `or` list instead of repeating `or`
    /// between every member.
    serial_or_list: bool,
    /// Oracle used one indefinite article shared by every member of a union,
    /// as in "a Kraken, Leviathan, Merfolk, Octopus, or Serpent."
    shared_indefinite_article: bool,
    /// Oracle placed controller scope after a restrictive `with`/`without`
    /// predicate, as in "permanent with fading you control." Controller
    /// identity remains semantic; this retains only authored word order.
    controller_after_qualifiers: bool,
    /// Oracle placed the controller after an enters event, as in "a land
    /// enters under an opponent's control." The ordinary controller filter
    /// remains the matching constraint; this retains only authored word order.
    enters_under_controller: bool,
    /// Oracle used the plural player pronoun for the actor and the controlled
    /// object in the same clause, as in "they return a land they control."
    /// Player identity remains the ordinary iterated-player filter.
    iterated_actor_pronoun: bool,
    /// Oracle used the pre-M10-style "played by [an opponent/your
    /// opponents]" entry surface. The controller filter remains the runtime
    /// meaning; this typed hint preserves the authored entry wording.
    played_by_opponent: Option<PlayedByOpponentSurface>,
    /// Oracle phrased a caster restriction as "except during its controller's
    /// turn." Runtime matching excludes spells cast by the active player;
    /// this hint retains the equivalent authored exception surface.
    except_during_controller_turn: bool,
    /// A cost modifier authored the candidate restriction as a trailing
    /// "if it has ..." clause. Ability markers remain the executable filter.
    trailing_candidate_ability_condition: bool,
    /// Oracle referred to a creature type selected by the current resolving
    /// instruction as "a type chosen this way" rather than the source's
    /// persistent singular "chosen type".
    chosen_type_this_way: bool,
    /// Oracle used the compact relational surface "with equal or lesser mana
    /// value" for a comparison against the object established by the prior
    /// instruction. The tagged constraint remains the executable relation.
    equal_or_lesser_mana_value: bool,
    one_or_more: bool,
    /// Oracle repeated an indefinite article for every arm of an explicit
    /// object-filter union (for example, "a Doctor card, a card with ..., or
    /// a Vehicle card"). This controls punctuation and articles only.
    explicit_branch_articles: bool,
    /// Oracle selected one member of a previously established tagged set with
    /// wording such as "one of them". Lowering owns the actual choice; this
    /// flag only preserves that authored reference in compiled text.
    one_of_tagged_set: bool,
    /// Oracle referred to a previously established plural object set using
    /// the bare pronoun `them`. The tagged constraint retains identity; this
    /// flag only distinguishes that surface from `those creatures`, etc.
    plural_pronoun_reference: bool,
    /// Oracle explicitly quantified this set with `all` or distributive
    /// `each`. This is presentation-only and does not change the matched set.
    set_quantifier: Option<crate::effect::SetQuantifierSurface>,
    /// Oracle authored this filter's object noun in the plural. Runtime
    /// matching is number-agnostic; this is retained for nested relation
    /// surfaces such as "attached to permanents you control."
    plural_object_noun: bool,
    /// Oracle placed a return destination before its object set, as in
    /// "return to your hand all enchantments." The destination and selected
    /// objects remain represented by the ordinary return effect.
    return_destination_first: bool,
    /// Oracle explicitly used `card`/`cards` for this filter's noun.
    ///
    /// Lowering may intentionally clear a nonbattlefield zone after it has
    /// encoded the actual movement or event elsewhere. Keep the noun surface
    /// independently so that the same semantic filter does not render as a
    /// battlefield `permanent` merely because its contextual zone moved.
    explicit_card_noun: bool,
    /// Card-type noun explicitly authored alongside another characteristic.
    ///
    /// Some canonical subtype renderings omit an inferred type noun (for
    /// example, `Urza's` rather than `Urza's land`). This equality-transparent
    /// hint retains an explicitly written noun when that distinction matters.
    explicit_card_type_noun: Option<CardType>,
    counter_requirement_one_or_more: bool,
    counter_requirement_plural_noun: bool,
    counter_requirement_plural_subject: bool,
    /// Oracle placed the ownership clause before a nonbattlefield zone, as in
    /// "cards you own in exile", rather than using the canonical possessive
    /// zone surface "cards in your exile".
    owner_before_zone: bool,
    /// Oracle placed a counter requirement after the zone clause, as in
    /// "cards in exile with a memory counter on them". Counter matching and
    /// zone matching remain ordinary semantic filter fields.
    counter_requirement_after_zone: bool,
    /// The enclosing distributive instruction began with an authored `Then`.
    /// This is retained on the iterated filter because source-sentence
    /// grouping may be flattened after a later lifecycle sentence is folded
    /// into the iteration payload.
    for_each_leading_then: bool,
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
    /// Oracle described this result as a card "put into a graveyard this
    /// way". A mill producer has the same executable result set, so its tag
    /// cannot preserve this authored wording by itself.
    put_into_graveyard_this_way: bool,
    /// Explicit noun/action for a reference to an additional-cost object.
    /// This is deliberately equality-transparent with the rest of this
    /// presentation metadata.
    additional_cost_object: Option<AdditionalCostObjectSurface>,
    /// Authored noun for a `SameNameAsTagged` antecedent. The tagged relation
    /// carries identity; this field only preserves the unambiguous noun.
    same_name_antecedent: Option<SameNameAntecedentSurface>,
    /// Authored source noun in a chosen-name relationship. The tagged
    /// constraint remains the executable name comparison.
    chosen_name_source: Option<ChosenNameSourceSurface>,
    /// Authored noun for an explicit demonstrative condition subject.
    demonstrative_antecedent: Option<DemonstrativeAntecedentSurface>,
    /// Authored relative clause for current-turn graveyard entry.
    graveyard_entry_history: Option<GraveyardEntryHistorySurface>,
    /// Authored multi-zone/domain expansion for a global characteristic rule.
    global_characteristic_domain: Option<GlobalCharacteristicDomainSurface>,
    /// Oracle explicitly named the battlefield in a current-turn entry
    /// relative clause. The ordinary zone and history predicate remain the
    /// executable semantics; this retains only the authored surface.
    entered_battlefield_explicit_surface: bool,
    /// Oracle used the causative entry surface `a player puts ... onto the
    /// battlefield`. The ordinary zone-change trigger and triggering-object
    /// controller relation remain the executable semantics.
    player_puts_onto_battlefield_surface: bool,
    /// Oracle introduced an entry-history condition with "you had ...
    /// enter" rather than the canonical past-tense relative clause.
    you_had_entry_surface: bool,
    /// Oracle placed a mana-source cast predicate after a granted ability as
    /// an `if` clause ("has split second if mana from ... was spent") rather
    /// than inside the affected-spell noun phrase. The ordinary
    /// `mana_from_source_spent_to_cast` filter remains executable.
    mana_source_spent_trailing_if_surface: bool,
    /// Oracle framed this turn-long stack-object grant as
    /// "as you cast ... this turn, they gain ...".
    as_you_cast_this_turn_surface: bool,
}

impl ObjectFilterUnionSurface {
    pub const fn new(connective: ObjectFilterUnionConnective) -> Self {
        Self {
            connective,
            subtype_before_card_type: false,
            terminal_noun_after_type_subtype_union: false,
            conjunctive_set_surface: false,
            serial_or_list: false,
            shared_indefinite_article: false,
            controller_after_qualifiers: false,
            enters_under_controller: false,
            iterated_actor_pronoun: false,
            played_by_opponent: None,
            except_during_controller_turn: false,
            trailing_candidate_ability_condition: false,
            chosen_type_this_way: false,
            equal_or_lesser_mana_value: false,
            one_or_more: false,
            explicit_branch_articles: false,
            one_of_tagged_set: false,
            plural_pronoun_reference: false,
            set_quantifier: None,
            plural_object_noun: false,
            return_destination_first: false,
            explicit_card_noun: false,
            explicit_card_type_noun: None,
            counter_requirement_one_or_more: false,
            counter_requirement_plural_noun: false,
            counter_requirement_plural_subject: false,
            owner_before_zone: false,
            counter_requirement_after_zone: false,
            for_each_leading_then: false,
            counter_exclusion_plural_noun: false,
            counter_exclusion_plural_subject: false,
            relative_attachment_state: false,
            relative_characteristic_list: false,
            prior_effect_action: None,
            put_into_graveyard_this_way: false,
            additional_cost_object: None,
            same_name_antecedent: None,
            chosen_name_source: None,
            demonstrative_antecedent: None,
            graveyard_entry_history: None,
            global_characteristic_domain: None,
            entered_battlefield_explicit_surface: false,
            player_puts_onto_battlefield_surface: false,
            you_had_entry_surface: false,
            mana_source_spent_trailing_if_surface: false,
            as_you_cast_this_turn_surface: false,
        }
    }

    pub const fn connective(self) -> ObjectFilterUnionConnective {
        self.connective
    }

    pub const fn with_connective(mut self, connective: ObjectFilterUnionConnective) -> Self {
        self.connective = connective;
        self
    }

    pub const fn with_subtype_before_card_type(mut self, subtype_first: bool) -> Self {
        self.subtype_before_card_type = subtype_first;
        self
    }

    pub const fn subtype_before_card_type(self) -> bool {
        self.subtype_before_card_type
    }

    pub const fn with_terminal_noun_after_type_subtype_union(mut self, terminal: bool) -> Self {
        self.terminal_noun_after_type_subtype_union = terminal;
        self
    }

    pub const fn terminal_noun_after_type_subtype_union(self) -> bool {
        self.terminal_noun_after_type_subtype_union
    }

    pub const fn with_conjunctive_set_surface(mut self, conjunctive: bool) -> Self {
        self.conjunctive_set_surface = conjunctive;
        self
    }

    pub const fn conjunctive_set_surface(self) -> bool {
        self.conjunctive_set_surface
    }

    pub const fn with_serial_or_list(mut self, serial: bool) -> Self {
        self.serial_or_list = serial;
        self
    }

    pub const fn serial_or_list(self) -> bool {
        self.serial_or_list
    }

    pub const fn with_shared_indefinite_article(mut self, shared: bool) -> Self {
        self.shared_indefinite_article = shared;
        self
    }

    pub const fn shared_indefinite_article(self) -> bool {
        self.shared_indefinite_article
    }

    pub const fn with_controller_after_qualifiers(mut self, postpositive: bool) -> Self {
        self.controller_after_qualifiers = postpositive;
        self
    }

    pub const fn controller_after_qualifiers(self) -> bool {
        self.controller_after_qualifiers
    }

    pub const fn with_enters_under_controller(mut self, postpositive: bool) -> Self {
        self.enters_under_controller = postpositive;
        self
    }

    pub const fn enters_under_controller(self) -> bool {
        self.enters_under_controller
    }

    pub const fn with_iterated_actor_pronoun(mut self, pronoun: bool) -> Self {
        self.iterated_actor_pronoun = pronoun;
        self
    }

    pub const fn iterated_actor_pronoun(self) -> bool {
        self.iterated_actor_pronoun
    }

    pub const fn with_played_by_opponent(
        mut self,
        surface: Option<PlayedByOpponentSurface>,
    ) -> Self {
        self.played_by_opponent = surface;
        self
    }

    pub const fn played_by_opponent(self) -> Option<PlayedByOpponentSurface> {
        self.played_by_opponent
    }

    pub const fn with_except_during_controller_turn(mut self, except: bool) -> Self {
        self.except_during_controller_turn = except;
        self
    }

    pub const fn except_during_controller_turn(self) -> bool {
        self.except_during_controller_turn
    }

    pub const fn with_trailing_candidate_ability_condition(mut self, trailing: bool) -> Self {
        self.trailing_candidate_ability_condition = trailing;
        self
    }

    pub const fn trailing_candidate_ability_condition(self) -> bool {
        self.trailing_candidate_ability_condition
    }

    pub const fn with_chosen_type_this_way(mut self, this_way: bool) -> Self {
        self.chosen_type_this_way = this_way;
        self
    }

    pub const fn chosen_type_this_way(self) -> bool {
        self.chosen_type_this_way
    }

    pub const fn with_equal_or_lesser_mana_value(mut self, compact: bool) -> Self {
        self.equal_or_lesser_mana_value = compact;
        self
    }

    pub const fn equal_or_lesser_mana_value(self) -> bool {
        self.equal_or_lesser_mana_value
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

    pub const fn with_plural_pronoun_reference(mut self, pronoun: bool) -> Self {
        self.plural_pronoun_reference = pronoun;
        self
    }

    pub const fn plural_pronoun_reference(self) -> bool {
        self.plural_pronoun_reference
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

    pub const fn with_plural_object_noun(mut self, plural: bool) -> Self {
        self.plural_object_noun = plural;
        self
    }

    pub const fn plural_object_noun(self) -> bool {
        self.plural_object_noun
    }

    pub const fn with_return_destination_first(mut self, destination_first: bool) -> Self {
        self.return_destination_first = destination_first;
        self
    }

    pub const fn return_destination_first(self) -> bool {
        self.return_destination_first
    }

    pub const fn explicit_card_noun(self) -> bool {
        self.explicit_card_noun
    }

    pub const fn with_explicit_card_noun(mut self, explicit_card_noun: bool) -> Self {
        self.explicit_card_noun = explicit_card_noun;
        self
    }

    pub const fn explicit_card_type_noun(self) -> Option<CardType> {
        self.explicit_card_type_noun
    }

    pub const fn with_explicit_card_type_noun(mut self, card_type: Option<CardType>) -> Self {
        self.explicit_card_type_noun = card_type;
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

    pub const fn with_owner_before_zone(mut self, owner_before_zone: bool) -> Self {
        self.owner_before_zone = owner_before_zone;
        self
    }

    pub const fn owner_before_zone(self) -> bool {
        self.owner_before_zone
    }

    pub const fn with_counter_requirement_after_zone(mut self, after_zone: bool) -> Self {
        self.counter_requirement_after_zone = after_zone;
        self
    }

    pub const fn counter_requirement_after_zone(self) -> bool {
        self.counter_requirement_after_zone
    }

    pub const fn with_for_each_leading_then(mut self, leading_then: bool) -> Self {
        self.for_each_leading_then = leading_then;
        self
    }

    pub const fn for_each_leading_then(self) -> bool {
        self.for_each_leading_then
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

    pub const fn with_put_into_graveyard_this_way(mut self, authored: bool) -> Self {
        self.put_into_graveyard_this_way = authored;
        self
    }

    pub const fn put_into_graveyard_this_way(self) -> bool {
        self.put_into_graveyard_this_way
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

    pub const fn with_chosen_name_source(
        mut self,
        surface: Option<ChosenNameSourceSurface>,
    ) -> Self {
        self.chosen_name_source = surface;
        self
    }

    pub const fn chosen_name_source(self) -> Option<ChosenNameSourceSurface> {
        self.chosen_name_source
    }

    pub const fn with_demonstrative_antecedent(
        mut self,
        surface: Option<DemonstrativeAntecedentSurface>,
    ) -> Self {
        self.demonstrative_antecedent = surface;
        self
    }

    pub const fn demonstrative_antecedent(self) -> Option<DemonstrativeAntecedentSurface> {
        self.demonstrative_antecedent
    }

    pub const fn with_graveyard_entry_history(
        mut self,
        surface: Option<GraveyardEntryHistorySurface>,
    ) -> Self {
        self.graveyard_entry_history = surface;
        self
    }

    pub const fn graveyard_entry_history(self) -> Option<GraveyardEntryHistorySurface> {
        self.graveyard_entry_history
    }

    pub const fn with_global_characteristic_domain(
        mut self,
        surface: Option<GlobalCharacteristicDomainSurface>,
    ) -> Self {
        self.global_characteristic_domain = surface;
        self
    }

    pub const fn global_characteristic_domain(self) -> Option<GlobalCharacteristicDomainSurface> {
        self.global_characteristic_domain
    }

    pub const fn with_entered_battlefield_explicit_surface(mut self, explicit: bool) -> Self {
        self.entered_battlefield_explicit_surface = explicit;
        self
    }

    pub const fn entered_battlefield_explicit_surface(self) -> bool {
        self.entered_battlefield_explicit_surface
    }

    pub const fn with_player_puts_onto_battlefield_surface(mut self, authored: bool) -> Self {
        self.player_puts_onto_battlefield_surface = authored;
        self
    }

    pub const fn player_puts_onto_battlefield_surface(self) -> bool {
        self.player_puts_onto_battlefield_surface
    }

    pub const fn with_you_had_entry_surface(mut self, authored: bool) -> Self {
        self.you_had_entry_surface = authored;
        self
    }

    pub const fn you_had_entry_surface(self) -> bool {
        self.you_had_entry_surface
    }

    pub const fn with_mana_source_spent_trailing_if_surface(mut self, trailing: bool) -> Self {
        self.mana_source_spent_trailing_if_surface = trailing;
        self
    }

    pub const fn mana_source_spent_trailing_if_surface(self) -> bool {
        self.mana_source_spent_trailing_if_surface
    }

    pub const fn with_as_you_cast_this_turn_surface(mut self, authored: bool) -> Self {
        self.as_you_cast_this_turn_surface = authored;
        self
    }

    pub const fn as_you_cast_this_turn_surface(self) -> bool {
        self.as_you_cast_this_turn_surface
    }
}

impl PartialEq for ObjectFilterUnionSurface {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for ObjectFilterUnionSurface {}

/// Which power/toughness reference a filter should use.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PtReference {
    #[default]
    Effective,
    Base,
}

/// Relationship between a candidate object's own power and toughness.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerToughnessRelation {
    PowerGreaterThanToughness,
    ToughnessGreaterThanPower,
    NotEqual,
}

/// Relationship an object may have with a tagged object set.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// The candidate shares a mana value with a different object in the
    /// tagged set. Unlike `SameManaValueAsTagged`, membership in the set does
    /// not make this relation vacuously true for the candidate itself.
    SameManaValueAsAnotherTagged,
    ManaValueLteTagged,
    ManaValueLtTagged,
    AttachedToTaggedObject,
    /// The candidate was among the attachments recorded on the tagged
    /// object's last-known snapshot. Unlike `AttachedToTaggedObject`, this
    /// remains true after the tagged object leaves the battlefield and the
    /// game's ordinary attachment cleanup detaches the candidate.
    WasAttachedToTaggedObject,
    SoulbondPartnerOfTagged,
    IsNotTaggedObject,
}

/// A characteristic that can be compared between a candidate object and a
/// separately filtered set of objects.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectCharacteristic {
    CardType,
    /// Runtime-equivalent to `CardType`, while preserving Oracle's
    /// "permanent type" surface.
    PermanentType,
    Subtype(SubtypeFamily),
    Color,
    ManaValue,
}

impl ObjectCharacteristic {
    pub fn sharing_phrase(self) -> String {
        match self {
            Self::CardType => "a card type".to_string(),
            Self::PermanentType => "a permanent type".to_string(),
            Self::Subtype(family) => format!("a {}", family.type_phrase()),
            Self::Color => "a color".to_string(),
            Self::ManaValue => "mana value".to_string(),
        }
    }
}

/// Whether a candidate must share at least one listed characteristic with the
/// comparison set, or share none of them.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectCharacteristicRelationKind {
    SharesAny,
    SharesNone,
}

/// A relation between the candidate object and all objects selected by
/// `comparison`.
///
/// Multiple characteristics are alternatives, matching Oracle constructions
/// such as "shares a color or mana value with the exiled card."
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectCharacteristicRelation {
    pub kind: ObjectCharacteristicRelationKind,
    pub characteristics: Vec<ObjectCharacteristic>,
    pub comparison: ObjectFilter,
}

impl ObjectCharacteristicRelation {
    pub fn shares(characteristics: Vec<ObjectCharacteristic>, comparison: ObjectFilter) -> Self {
        Self {
            kind: ObjectCharacteristicRelationKind::SharesAny,
            characteristics,
            comparison,
        }
    }

    pub fn shares_none(
        characteristics: Vec<ObjectCharacteristic>,
        comparison: ObjectFilter,
    ) -> Self {
        Self {
            kind: ObjectCharacteristicRelationKind::SharesNone,
            characteristics,
            comparison,
        }
    }

    pub fn comparison_description(&self) -> String {
        if self.comparison.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == crate::SOURCE_EXILED_TAG
        }) {
            return "the exiled card".to_string();
        }

        let description = self.comparison.description();
        let keep_bare_reference = self.comparison.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && matches!(constraint.tag.as_str(), "equipped" | "enchanted")
        });
        if keep_bare_reference {
            description
        } else {
            ensure_indefinite_article(description)
        }
    }
}

/// Alternative casting capability qualifier for card filters.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterConstraint {
    Any,
    Typed(CounterType),
    AtLeast {
        counter_type: Option<CounterType>,
        count: u32,
    },
}

/// A parity requirement for numeric object properties.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePowerRelation {
    LessThanSource,
}

/// Stack object kind constraint for stack-targeting filters.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackObjectKind {
    Spell,
    Ability,
    ActivatedAbility,
    TriggeredAbility,
    SpellOrAbility,
}

/// A tagged-object constraint used by object filters.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedObjectConstraint {
    pub tag: TagKey,
    pub relation: TaggedOpbjectRelation,
}

/// Filter for selecting players.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PlayerFilter {
    #[default]
    Any,
    You,
    NotYou,
    Opponent,
    Teammate,
    /// The nearest in-game player seated to the effect controller's left.
    PlayerToYourLeft,
    /// The nearest in-game player seated to the effect controller's right.
    PlayerToYourRight,
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
    /// A player the source creature attacked this turn.
    ///
    /// This is source-relative rather than controller-relative: it matches
    /// the player named by a `CreatureAttackedEvent` whose attacker is the
    /// current resolving source.
    AttackedBySourceThisTurn,
    /// A player matching `base` that the current source object has dealt
    /// positive damage to at any earlier point in this game.
    ///
    /// Source identity is object-instance-relative: a permanent that leaves
    /// and returns does not inherit the earlier object's damage history.
    WasDealtDamageBySourceThisGame {
        base: Box<PlayerFilter>,
    },
    /// A player matching `base` who was dealt positive combat damage this
    /// game by an object matching `sources` at the time it dealt that damage.
    WasDealtCombatDamageBySourcesThisGame {
        base: Box<PlayerFilter>,
        sources: Box<ObjectFilter>,
    },
    /// A player matching `base` who has lost life during the current turn.
    ///
    /// This is a target-legality fact as well as a resolution-time filter:
    /// effects such as "target player who lost life this turn" must reject a
    /// player before the target is announced when their history does not
    /// satisfy the qualifier.
    LostLifeThisTurn {
        base: Box<PlayerFilter>,
    },
    /// A player matching `base` who was dealt positive combat damage this
    /// turn by at least `minimum` distinct objects matching `sources` at the
    /// time they dealt that damage.
    WasDealtCombatDamageByDistinctSourcesThisTurn {
        base: Box<PlayerFilter>,
        sources: Box<ObjectFilter>,
        minimum: u32,
    },
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
    /// The unique in-game player who controls more objects matching `filter`
    /// than every other in-game player. No player matches when the lead is
    /// tied.
    ControlsMost {
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

    /// You and every teammate of yours. In a non-team game this matches only
    /// you, because every other player is an opponent.
    pub fn your_team() -> Self {
        Self::excluding(Self::Any, Self::Opponent)
    }

    pub fn is_your_team(&self) -> bool {
        matches!(
            self,
            Self::Excluding { base, excluded }
                if matches!(base.as_ref(), Self::Any)
                    && matches!(excluded.as_ref(), Self::Opponent)
        )
    }

    /// Returns the otherwise-legal player filter for an authored relational
    /// target such as "another target player." The referenced earlier target
    /// is enforced by target-assignment distinctness, not by independently
    /// matching that unresolved reference as part of the new target's domain.
    pub fn relative_target_exclusion_base(&self) -> Option<&Self> {
        let Self::Excluding { base, excluded } = self else {
            return None;
        };
        matches!(excluded.as_ref(), Self::Target(_) | Self::AliasedTarget(_))
            .then_some(base.as_ref())
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

    pub fn was_dealt_damage_by_source_this_game(base: PlayerFilter) -> Self {
        Self::WasDealtDamageBySourceThisGame {
            base: Box::new(base),
        }
    }

    pub fn was_dealt_combat_damage_by_sources_this_game(
        base: PlayerFilter,
        sources: ObjectFilter,
    ) -> Self {
        Self::WasDealtCombatDamageBySourcesThisGame {
            base: Box::new(base),
            sources: Box::new(sources),
        }
    }

    pub fn lost_life_this_turn(base: PlayerFilter) -> Self {
        Self::LostLifeThisTurn {
            base: Box::new(base),
        }
    }

    pub fn was_dealt_combat_damage_by_distinct_sources_this_turn(
        base: PlayerFilter,
        sources: ObjectFilter,
        minimum: u32,
    ) -> Self {
        Self::WasDealtCombatDamageByDistinctSourcesThisTurn {
            base: Box::new(base),
            sources: Box::new(sources),
            minimum,
        }
    }

    pub fn mentions_iterated_player(&self) -> bool {
        match self {
            Self::IteratedPlayer => true,
            Self::Target(inner) | Self::AliasedTarget(inner) => inner.mentions_iterated_player(),
            Self::CardsInHandAtLeastMoreThanYou { base, .. } => base.mentions_iterated_player(),
            Self::WasDealtDamageBySourceThisGame { base } => base.mentions_iterated_player(),
            Self::WasDealtCombatDamageBySourcesThisGame { base, sources } => {
                base.mentions_iterated_player() || sources.mentions_iterated_player()
            }
            Self::LostLifeThisTurn { base } => base.mentions_iterated_player(),
            Self::WasDealtCombatDamageByDistinctSourcesThisTurn { base, sources, .. } => {
                base.mentions_iterated_player() || sources.mentions_iterated_player()
            }
            Self::HasMoreLifeThanYou { base } => base.mentions_iterated_player(),
            Self::OpponentWithMoreControlledObjectsThan { player, filter } => {
                player.mentions_iterated_player() || filter.mentions_iterated_player()
            }
            Self::ControlsMost { filter } => filter.mentions_iterated_player(),
            Self::MaxSpeed { base, .. } => base.mentions_iterated_player(),
            Self::Excluding { base, excluded } => {
                base.mentions_iterated_player() || excluded.mentions_iterated_player()
            }
            Self::Any
            | Self::You
            | Self::NotYou
            | Self::Opponent
            | Self::Teammate
            | Self::PlayerToYourLeft
            | Self::PlayerToYourRight
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
            | Self::AttackedBySourceThisTurn
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
            Self::PlayerToYourLeft => "the player to your left".to_string(),
            Self::PlayerToYourRight => "the player to your right".to_string(),
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
            Self::AttackedBySourceThisTurn => {
                "a player this creature attacked this turn".to_string()
            }
            Self::WasDealtDamageBySourceThisGame { base } => format!(
                "{} this source has dealt damage to this game",
                base.description()
            ),
            Self::WasDealtCombatDamageBySourcesThisGame { base, sources } => format!(
                "{} dealt combat damage this game by {}",
                base.description(),
                sources.description()
            ),
            Self::LostLifeThisTurn { base } => {
                format!("{} who lost life this turn", base.description())
            }
            Self::WasDealtCombatDamageByDistinctSourcesThisTurn {
                base,
                sources,
                minimum,
            } => format!(
                "{} who was dealt combat damage by {} this turn",
                base.description(),
                describe_distinct_source_threshold(sources, *minimum)
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
            Self::ControlsMost { filter } => format!(
                "the player who controls the most {}",
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// Turn-scoped counter-placement provenance required by an object filter.
///
/// This is intentionally distinct from [`CounterConstraint`]. A permanent can
/// still have a counter that was placed on an earlier turn or by a different
/// player; this constraint asks which player controlled the source of the
/// counter-placement event for this exact object during the current turn.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct CountersPutOnThisTurnConstraint {
    /// `None` matches counters of any type.
    pub counter_type: Option<CounterType>,
    /// The controller recorded on the source of the counter-placement event.
    pub source_controller: PlayerFilter,
    /// Minimum total number placed by matching sources this turn.
    pub minimum: u32,
}

impl CountersPutOnThisTurnConstraint {
    pub fn new(
        counter_type: Option<CounterType>,
        source_controller: PlayerFilter,
        minimum: u32,
    ) -> Self {
        Self {
            counter_type,
            source_controller,
            minimum,
        }
    }
}

/// Oracle spelling retained for an excluded literal card name.
///
/// The normalized value in [`ObjectFilter::excluded_name`] remains the
/// semantic source of truth for matching and filter equality. This wrapper is
/// deliberately equality-transparent so capitalization and punctuation never
/// affect lowering, deduplication, or runtime behavior.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct ExcludedNameSurface(Option<String>);

impl ExcludedNameSurface {
    pub fn new(surface: impl Into<String>) -> Self {
        Self(Some(surface.into()))
    }

    pub fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

impl PartialEq for ExcludedNameSurface {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

/// Filter for selecting objects (permanents, spells, cards).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectFilter {
    pub zone: Option<Zone>,
    pub controller: Option<PlayerFilter>,
    pub cast_by: Option<PlayerFilter>,
    /// A stack spell must not have been cast from this zone.
    ///
    /// This is distinct from `zone`: for stack spells, a non-stack `zone`
    /// is the positive "cast from <zone>" constraint, while this field
    /// represents Oracle restrictions such as "a spell that wasn't cast
    /// from its owner's hand."
    pub excluded_cast_origin_zone: Option<Zone>,
    pub cast_this_turn: bool,
    pub first_spell_cast_each_turn: bool,
    /// Exact ordinal among spells matching this filter that the caster has
    /// cast this turn. `None` is the ordinary unrestricted set; `Some(2)` is
    /// the reusable surface used by "the second spell you cast each turn".
    pub spell_cast_ordinal_each_turn: Option<u32>,
    /// A stack spell must have had mana produced by a matching source spent
    /// to cast it. The runtime evaluates this against the source snapshots
    /// recorded on the spell as each mana unit is paid.
    pub mana_from_source_spent_to_cast: Option<Box<ObjectFilter>>,
    pub owner: Option<PlayerFilter>,
    pub single_graveyard: bool,
    pub targets_player: Option<PlayerFilter>,
    pub targets_object: Option<Box<ObjectFilter>>,
    pub targets_any_of: bool,
    pub stack_kind: Option<StackObjectKind>,
    pub target_count: Option<ChoiceCount>,
    pub target_set_same_controller: bool,
    pub target_set_different_controllers: bool,
    /// Constraint on the selected target set rather than on each candidate.
    ///
    /// This is boxed because an aggregate maximum may itself contain a
    /// [`Value`] that references a [`ChooseSpec`], which contains an
    /// `ObjectFilter` in turn.
    pub target_set_aggregate_constraint: Option<Box<ChoiceAggregateConstraint>>,
    pub targets_only_player: Option<PlayerFilter>,
    pub targets_only_object: Option<Box<ObjectFilter>>,
    pub targets_only_any_of: bool,
    pub could_be_targeted_by: Option<TargetabilityConstraint>,
    pub card_types: Vec<CardType>,
    pub all_card_types: Vec<CardType>,
    pub excluded_card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    /// Every listed subtype must be present. This represents compound
    /// subtype phrases such as "Eldrazi Spawn"; `subtypes` remains the
    /// inclusive-any form used by "Elf or Goblin".
    pub all_subtypes: Vec<Subtype>,
    pub type_or_subtype_union: bool,
    pub union_surface: ObjectFilterUnionSurface,
    pub excluded_subtypes: Vec<Subtype>,
    pub supertypes: Vec<Supertype>,
    pub excluded_supertypes: Vec<Supertype>,
    pub colors: Option<ColorSet>,
    pub required_colors: Option<ColorSet>,
    pub chosen_color: bool,
    /// Candidate shares at least one color with the pregame draft choices
    /// recorded for the indicated named card group.
    pub colors_chosen_while_drafting_named: Option<String>,
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
    /// Excludes every creature type accumulated by choices made during the
    /// current source's resolution, rather than only its last singular type.
    pub excluded_any_chosen_creature_type: bool,
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
    /// Requires a permanent currently designated as suspected.
    pub suspected: bool,
    pub sticker: Option<KeywordActionKind>,
    pub token: bool,
    pub nontoken: bool,
    pub face_down: Option<bool>,
    /// Requires a card currently marked as having been exiled by the foretell
    /// special action. This is distinct from merely being face down in exile.
    pub foretold: bool,
    pub other: bool,
    pub tapped: bool,
    pub untapped: bool,
    pub attacking: bool,
    /// Requires this creature to be its controller's only declared attacker
    /// in the current combat.
    pub attacking_alone: bool,
    pub attacked_this_turn: bool,
    /// Requires an object that was the source of an activated ability this
    /// turn. For planeswalkers, Oracle conventionally phrases this as a
    /// planeswalker "that was activated this turn."
    pub ability_activated_this_turn: bool,
    /// Requires a creature that was declared as a blocker during this turn.
    pub blocked_this_turn: bool,
    pub didnt_attack_this_turn: bool,
    /// Requires a creature that is legally able to attack. This is used with
    /// turn history for instructions that affect creatures that did not
    /// attack, except for creatures that couldn't attack.
    pub could_have_attacked_this_turn: bool,
    pub attacking_player_or_planeswalker_controlled_by: Option<PlayerFilter>,
    /// When set with `attacking_player_or_planeswalker_controlled_by`, require
    /// the attack target itself to be that player rather than a planeswalker
    /// they control.
    pub attacking_player_only: bool,
    /// Requires a Battle whose current designated protector matches this
    /// player. This is distinct from controller/owner: Sieges are normally
    /// controlled by the player who cast them and protected by an opponent.
    pub protected_by: Option<PlayerFilter>,
    /// The battlefield object this object is attached to must match this
    /// filter. Unlike a tagged-object relation, this is an intrinsic selector
    /// and is valid without a prior effect establishing a tag.
    pub attached_to_object: Option<Box<ObjectFilter>>,
    pub attached_to_player: Option<PlayerFilter>,
    /// At least one attachment on this object must match the inner filter
    /// ("a creature with a legendary Equipment attached to it").
    pub with_attached_object: Option<Box<ObjectFilter>>,
    /// No attachment on this object may match the inner filter. This is the
    /// executable complement of `with_attached_object` for selectors such as
    /// "creatures that aren't enchanted."
    pub without_attached_object: Option<Box<ObjectFilter>>,
    pub nonattacking: bool,
    pub enlist_eligible: bool,
    pub blocking: bool,
    pub nonblocking: bool,
    pub blocked: bool,
    pub blocked_by: Option<ObjectRef>,
    pub blocked_by_source: bool,
    /// This creature either blocked an object matching the nested filter or
    /// was blocked by one this turn. Runtime matching uses the declaration
    /// event's object snapshots so the other creature can be checked using
    /// last known information after it leaves the battlefield.
    pub blocked_or_was_blocked_by_this_turn: Option<Box<ObjectFilter>>,
    pub unblocked: bool,
    /// Requires the candidate to be one of the object targets in the current
    /// resolution context. This is an identity relation, not merely the
    /// Oracle-facing `target` determiner used by a choice specification.
    pub is_target_object: bool,
    pub in_combat_with_source: bool,
    /// Requires the candidate creature to be in the current combat with the
    /// referenced creature: either it blocks that creature or that creature
    /// blocks it. This is distinct from `in_combat_with_source` because some
    /// spells select a creature and then affect the creatures fighting it.
    pub in_combat_with: Option<ObjectRef>,
    pub entered_since_your_last_turn_ended: bool,
    /// Requires an object that has not entered the battlefield during the
    /// current turn.
    pub didnt_enter_battlefield_this_turn: bool,
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
    /// The object moved from a library to a graveyard during the current turn.
    /// This is stable-identity history, not merely a present-zone qualifier.
    pub entered_graveyard_from_library_this_turn: bool,
    pub surveilled_this_turn: bool,
    /// Requires this exact object to have received matching counters this turn
    /// from a source controlled by the matching player.
    pub counters_put_on_this_turn: Option<CountersPutOnThisTurnConstraint>,
    pub discarded_or_cycled_this_turn_by: Option<PlayerFilter>,
    pub was_dealt_damage_this_turn: bool,
    /// Active voice: the object itself DEALT damage this turn
    /// ("target creature that dealt damage this turn").
    pub dealt_damage_this_turn: bool,
    pub dealt_damage_by_source_this_turn: Option<crate::DamagedBySource>,
    /// The current source object has dealt positive damage to this exact
    /// object at any earlier point in the game.
    pub was_dealt_damage_by_source_this_game: bool,
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
    /// Requires the candidate's printed mana cost to equal this exact cost.
    /// This is intentionally distinct from mana value: `{1}` must not match
    /// `{W}`, even though both have mana value 1.
    pub exact_mana_cost: Option<ManaCost>,
    pub has_mana_cost: bool,
    /// Requires at least one printed mana-cost pip that can be paid with life
    /// (a Phyrexian mana symbol). Oracle represents this family as `{H}` in
    /// characteristic filters even though actual card costs use `{W/P}`, etc.
    pub has_phyrexian_mana_symbol: bool,
    /// Requires an object with a mana ability capable of producing at least
    /// one of these symbols. This is a capability predicate, not a mana-cost
    /// or color-characteristic constraint.
    pub could_produce_mana: Vec<ManaSymbol>,
    pub has_tap_activated_ability: bool,
    pub no_abilities: bool,
    pub no_x_in_cost: bool,
    pub has_x_in_cost: bool,
    pub with_counter: Option<CounterConstraint>,
    pub without_counter: Option<CounterConstraint>,
    pub total_counters_parity: Option<ParityRequirement>,
    pub name: Option<String>,
    pub excluded_name: Option<String>,
    pub excluded_name_surface: ExcludedNameSurface,
    /// The candidate's current name must belong to an oracle identity whose
    /// earliest eligible paper printing was in this expansion.
    pub name_originally_printed_in_set: Option<String>,
    pub distinct_names: bool,
    /// Selection-set constraint: chosen objects must have pairwise distinct
    /// mana values. This does not change whether an individual object matches.
    pub distinct_mana_values: bool,
    pub distinct_powers: bool,
    pub distinct_creature_types: bool,
    /// Selection-set constraint: chosen cards must be assignable to distinct
    /// card-type slots. A multitype card may satisfy any one of its types, but
    /// the same type cannot be assigned to two chosen cards.
    pub one_per_card_type: bool,
    pub alternative_cast: Option<AlternativeCastKind>,
    pub static_abilities: Vec<StaticAbilityId>,
    pub excluded_static_abilities: Vec<StaticAbilityId>,
    pub ability_markers: Vec<String>,
    pub excluded_ability_markers: Vec<String>,
    pub no_shared_creature_types_with: Vec<ObjectFilter>,
    pub characteristic_relations: Vec<ObjectCharacteristicRelation>,
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
    /// Whether this filter carries the complete permanent card-type domain.
    ///
    /// Parsers use this typed representation for an authored `permanent`
    /// noun when the surrounding filter needs to retain object
    /// characteristics. Keeping the distinction from an otherwise untyped
    /// battlefield filter lets renderers preserve the authored noun.
    pub fn has_all_permanent_card_types(&self) -> bool {
        const PERMANENT_CARD_TYPES: [CardType; 6] = [
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Battle,
        ];

        self.card_types.len() == PERMANENT_CARD_TYPES.len()
            && PERMANENT_CARD_TYPES
                .iter()
                .all(|card_type| self.card_types.contains(card_type))
    }

    /// Whether this stack filter carries the complete permanent-spell domain.
    ///
    /// Lands are deliberately absent because they cannot be spells. Keeping
    /// this check separate from [`Self::has_all_permanent_card_types`] lets a
    /// conditional retain its executable card-type filter while rendering the
    /// authored aggregate noun `permanent spell`.
    pub fn has_all_permanent_spell_card_types(&self) -> bool {
        const PERMANENT_SPELL_CARD_TYPES: [CardType; 5] = [
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Planeswalker,
            CardType::Battle,
        ];

        matches!(self.zone, Some(Zone::Stack))
            && matches!(self.stack_kind, Some(StackObjectKind::Spell))
            && self.card_types.len() == PERMANENT_SPELL_CARD_TYPES.len()
            && PERMANENT_SPELL_CARD_TYPES
                .iter()
                .all(|card_type| self.card_types.contains(card_type))
    }

    pub fn set_controller_after_qualifiers_surface(&mut self, postpositive: bool) {
        self.union_surface = self
            .union_surface
            .with_controller_after_qualifiers(postpositive);
    }

    pub fn has_controller_after_qualifiers_surface(&self) -> bool {
        self.union_surface.controller_after_qualifiers()
    }

    pub fn set_enters_under_controller_surface(&mut self, postpositive: bool) {
        self.union_surface = self
            .union_surface
            .with_enters_under_controller(postpositive);
    }

    pub fn has_enters_under_controller_surface(&self) -> bool {
        self.union_surface.enters_under_controller()
    }

    pub fn set_iterated_actor_pronoun_surface(&mut self, pronoun: bool) {
        self.union_surface = self.union_surface.with_iterated_actor_pronoun(pronoun);
    }

    pub fn has_iterated_actor_pronoun_surface(&self) -> bool {
        self.union_surface.iterated_actor_pronoun()
    }

    pub fn set_played_by_opponent_surface(&mut self, surface: PlayedByOpponentSurface) {
        self.union_surface = self.union_surface.with_played_by_opponent(Some(surface));
    }

    pub fn played_by_opponent_surface(&self) -> Option<PlayedByOpponentSurface> {
        self.union_surface.played_by_opponent()
    }

    pub fn set_except_during_controller_turn_surface(&mut self, except: bool) {
        self.union_surface = self
            .union_surface
            .with_except_during_controller_turn(except);
    }

    pub fn has_except_during_controller_turn_surface(&self) -> bool {
        self.union_surface.except_during_controller_turn()
    }

    pub fn set_trailing_candidate_ability_condition_surface(&mut self, trailing: bool) {
        self.union_surface = self
            .union_surface
            .with_trailing_candidate_ability_condition(trailing);
    }

    pub const fn has_trailing_candidate_ability_condition_surface(&self) -> bool {
        self.union_surface.trailing_candidate_ability_condition()
    }

    pub fn set_chosen_type_this_way_surface(&mut self, this_way: bool) {
        self.union_surface = self.union_surface.with_chosen_type_this_way(this_way);
    }

    pub const fn has_chosen_type_this_way_surface(&self) -> bool {
        self.union_surface.chosen_type_this_way()
    }

    pub fn mentions_iterated_player(&self) -> bool {
        [
            self.controller.as_ref(),
            self.cast_by.as_ref(),
            self.owner.as_ref(),
            self.targets_player.as_ref(),
            self.targets_only_player.as_ref(),
            self.attacking_player_or_planeswalker_controlled_by.as_ref(),
            self.protected_by.as_ref(),
            self.attached_to_player.as_ref(),
            self.entered_battlefield_controller.as_ref(),
            self.counters_put_on_this_turn
                .as_ref()
                .map(|constraint| &constraint.source_controller),
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
                .mana_from_source_spent_to_cast
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
                .blocked_or_was_blocked_by_this_turn
                .as_deref()
                .is_some_and(ObjectFilter::mentions_iterated_player)
            || self
                .no_shared_creature_types_with
                .iter()
                .any(ObjectFilter::mentions_iterated_player)
            || self
                .characteristic_relations
                .iter()
                .any(|relation| relation.comparison.mentions_iterated_player())
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

    pub fn set_subtype_before_card_type_union_surface(&mut self, subtype_first: bool) {
        self.union_surface = self
            .union_surface
            .with_subtype_before_card_type(subtype_first);
    }

    pub const fn has_subtype_before_card_type_union_surface(&self) -> bool {
        self.union_surface.subtype_before_card_type()
    }

    pub fn set_terminal_noun_after_type_subtype_union_surface(&mut self, terminal: bool) {
        self.union_surface = self
            .union_surface
            .with_terminal_noun_after_type_subtype_union(terminal);
    }

    pub const fn has_terminal_noun_after_type_subtype_union_surface(&self) -> bool {
        self.union_surface.terminal_noun_after_type_subtype_union()
    }

    pub fn set_conjunctive_set_surface(&mut self, conjunctive: bool) {
        self.union_surface = self.union_surface.with_conjunctive_set_surface(conjunctive);
    }

    pub const fn has_conjunctive_set_surface(&self) -> bool {
        self.union_surface.conjunctive_set_surface()
    }

    pub fn set_serial_or_list_surface(&mut self, serial: bool) {
        self.union_surface = self.union_surface.with_serial_or_list(serial);
    }

    pub const fn has_serial_or_list_surface(&self) -> bool {
        self.union_surface.serial_or_list()
    }

    pub fn set_shared_indefinite_article_surface(&mut self, shared: bool) {
        self.union_surface = self.union_surface.with_shared_indefinite_article(shared);
    }

    pub const fn has_shared_indefinite_article_surface(&self) -> bool {
        self.union_surface.shared_indefinite_article()
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

    pub fn set_plural_pronoun_reference_surface(&mut self, pronoun: bool) {
        self.union_surface = self.union_surface.with_plural_pronoun_reference(pronoun);
    }

    pub const fn has_plural_pronoun_reference_surface(&self) -> bool {
        self.union_surface.plural_pronoun_reference()
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

    /// Preserve whether this filter's noun was authored in the plural.
    /// This is useful for nested relation filters whose ordinary singular
    /// description would otherwise erase "permanents" or "creatures."
    pub fn set_plural_object_noun_surface(&mut self, plural: bool) {
        self.union_surface = self.union_surface.with_plural_object_noun(plural);
    }

    pub const fn has_plural_object_noun_surface(&self) -> bool {
        self.union_surface.plural_object_noun()
    }

    /// Preserve destination-first return word order without changing the
    /// selected objects or destination semantics.
    pub fn set_return_destination_first_surface(&mut self, destination_first: bool) {
        self.union_surface = self
            .union_surface
            .with_return_destination_first(destination_first);
    }

    pub const fn has_return_destination_first_surface(&self) -> bool {
        self.union_surface.return_destination_first()
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

    /// Preserve a card-type noun that Oracle wrote explicitly even when the
    /// semantic filter could infer it from an adjacent subtype.
    pub fn set_explicit_card_type_noun(&mut self, card_type: Option<CardType>) {
        self.union_surface = self.union_surface.with_explicit_card_type_noun(card_type);
    }

    pub const fn explicit_card_type_noun(&self) -> Option<CardType> {
        self.union_surface.explicit_card_type_noun()
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

    pub fn set_owner_before_zone_surface(&mut self, owner_before_zone: bool) {
        self.union_surface = self.union_surface.with_owner_before_zone(owner_before_zone);
    }

    pub const fn has_owner_before_zone_surface(&self) -> bool {
        self.union_surface.owner_before_zone()
    }

    pub fn set_counter_requirement_after_zone_surface(&mut self, after_zone: bool) {
        self.union_surface = self
            .union_surface
            .with_counter_requirement_after_zone(after_zone);
    }

    pub const fn has_counter_requirement_after_zone_surface(&self) -> bool {
        self.union_surface.counter_requirement_after_zone()
    }

    pub fn set_for_each_leading_then_surface(&mut self, leading_then: bool) {
        self.union_surface = self.union_surface.with_for_each_leading_then(leading_then);
    }

    pub const fn has_for_each_leading_then_surface(&self) -> bool {
        self.union_surface.for_each_leading_then()
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

    /// Preserve "put into a graveyard this way" independently from the
    /// producer action used for executable result correlation.
    pub fn set_put_into_graveyard_this_way_surface(&mut self, authored: bool) {
        self.union_surface = self
            .union_surface
            .with_put_into_graveyard_this_way(authored);
    }

    pub const fn has_put_into_graveyard_this_way_surface(&self) -> bool {
        self.union_surface.put_into_graveyard_this_way()
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

    /// Preserve the authored source noun of a chosen-name relationship
    /// without changing the tagged runtime comparison.
    pub fn set_chosen_name_source_surface(&mut self, surface: Option<ChosenNameSourceSurface>) {
        self.union_surface = self.union_surface.with_chosen_name_source(surface);
    }

    pub const fn chosen_name_source_surface(&self) -> Option<ChosenNameSourceSurface> {
        self.union_surface.chosen_name_source()
    }

    /// Preserve the authored noun of an explicit demonstrative condition
    /// subject without changing filter matching.
    pub fn set_demonstrative_antecedent_surface(
        &mut self,
        surface: Option<DemonstrativeAntecedentSurface>,
    ) {
        self.union_surface = self.union_surface.with_demonstrative_antecedent(surface);
    }

    pub const fn demonstrative_antecedent_surface(&self) -> Option<DemonstrativeAntecedentSurface> {
        self.union_surface.demonstrative_antecedent()
    }

    /// Preserve the authored graveyard-entry relative clause without changing
    /// the executable current-turn history predicate.
    pub fn set_graveyard_entry_history_surface(
        &mut self,
        surface: Option<GraveyardEntryHistorySurface>,
    ) {
        self.union_surface = self.union_surface.with_graveyard_entry_history(surface);
    }

    pub const fn graveyard_entry_history_surface(&self) -> Option<GraveyardEntryHistorySurface> {
        self.union_surface.graveyard_entry_history()
    }

    /// Preserve the authored categories of an executable global
    /// characteristic rule. This is equality-transparent presentation data;
    /// the ordinary filter fields remain the runtime selector.
    pub fn set_global_characteristic_domain_surface(
        &mut self,
        surface: Option<GlobalCharacteristicDomainSurface>,
    ) {
        self.union_surface = self
            .union_surface
            .with_global_characteristic_domain(surface);
    }

    pub const fn global_characteristic_domain_surface(
        &self,
    ) -> Option<GlobalCharacteristicDomainSurface> {
        self.union_surface.global_characteristic_domain()
    }

    /// Preserve whether Oracle explicitly named the battlefield in this
    /// current-turn entry clause without changing filter matching.
    pub fn set_entered_battlefield_explicit_surface(&mut self, explicit: bool) {
        self.union_surface = self
            .union_surface
            .with_entered_battlefield_explicit_surface(explicit);
    }

    pub const fn has_entered_battlefield_explicit_surface(&self) -> bool {
        self.union_surface.entered_battlefield_explicit_surface()
    }

    /// Preserve the causative `a player puts ... onto the battlefield`
    /// spelling without changing object-filter matching.
    pub fn set_player_puts_onto_battlefield_surface(&mut self, authored: bool) {
        self.union_surface = self
            .union_surface
            .with_player_puts_onto_battlefield_surface(authored);
    }

    pub const fn has_player_puts_onto_battlefield_surface(&self) -> bool {
        self.union_surface.player_puts_onto_battlefield_surface()
    }

    /// Preserve the authored "you had ... enter" history surface without
    /// changing the executable entry-history filter.
    pub fn set_you_had_entry_surface(&mut self, authored: bool) {
        self.union_surface = self.union_surface.with_you_had_entry_surface(authored);
    }

    pub const fn has_you_had_entry_surface(&self) -> bool {
        self.union_surface.you_had_entry_surface()
    }

    pub fn set_mana_source_spent_trailing_if_surface(&mut self, trailing: bool) {
        self.union_surface = self
            .union_surface
            .with_mana_source_spent_trailing_if_surface(trailing);
    }

    pub const fn has_mana_source_spent_trailing_if_surface(&self) -> bool {
        self.union_surface.mana_source_spent_trailing_if_surface()
    }

    pub fn set_as_you_cast_this_turn_surface(&mut self, authored: bool) {
        self.union_surface = self
            .union_surface
            .with_as_you_cast_this_turn_surface(authored);
    }

    pub const fn has_as_you_cast_this_turn_surface(&self) -> bool {
        self.union_surface.as_you_cast_this_turn_surface()
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
                .blocked_or_was_blocked_by_this_turn
                .as_deref()
                .is_some_and(Self::uses_power_or_toughness_characteristics)
            || self.characteristic_relations.iter().any(|relation| {
                relation
                    .comparison
                    .uses_power_or_toughness_characteristics()
            })
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
            || !self.all_subtypes.is_empty()
            || self.type_or_subtype_union
            || !self.excluded_subtypes.is_empty()
            || !self.supertypes.is_empty()
            || !self.excluded_supertypes.is_empty()
            || self.colors.is_some()
            || self.required_colors.is_some()
            || self.chosen_color
            || self.colors_chosen_while_drafting_named.is_some()
            || self.chosen_land_type
            || self.has_basic_land_type
            || self.has_nonbasic_land_type
            || self.chosen_creature_type
            || self.chosen_card_type
            || self.excluded_chosen_creature_type
            || self.excluded_any_chosen_creature_type
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
            || self.blocked_or_was_blocked_by_this_turn.is_some()
            || self.attached_to_player.is_some()
            || self.surveilled_this_turn
            || self.counters_put_on_this_turn.is_some()
            || self.discarded_or_cycled_this_turn_by.is_some()
            || self.drawn_this_turn
            || self.mana_value.is_some()
            || self.mana_value_parity.is_some()
            || self.mana_value_eq_counters_on_source.is_some()
            || self.exact_mana_cost.is_some()
            || self.has_mana_cost
            || self.has_phyrexian_mana_symbol
            || !self.could_produce_mana.is_empty()
            || self.has_tap_activated_ability
            || self.no_abilities
            || self.no_x_in_cost
            || self.has_x_in_cost
            || self.name.is_some()
            || self.excluded_name.is_some()
            || self.name_originally_printed_in_set.is_some()
            || self.distinct_mana_values
            || self.one_per_card_type
            || self.alternative_cast.is_some()
            || !self.static_abilities.is_empty()
            || !self.excluded_static_abilities.is_empty()
            || !self.ability_markers.is_empty()
            || !self.excluded_ability_markers.is_empty()
            || !self.no_shared_creature_types_with.is_empty()
            || !self.characteristic_relations.is_empty()
            || self.shares_creature_type_with_source
            || !self.tagged_constraints.is_empty()
            || self
                .attached_to_object
                .as_deref()
                .is_some_and(Self::uses_non_pt_battlefield_characteristics)
            || self
                .blocked_or_was_blocked_by_this_turn
                .as_deref()
                .is_some_and(Self::uses_non_pt_battlefield_characteristics)
            || self.characteristic_relations.iter().any(|relation| {
                relation
                    .comparison
                    .uses_non_pt_battlefield_characteristics()
            })
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
        let generic = Self {
            zone: self.zone,
            owner: self.owner.clone(),
            controller: self.controller.clone(),
            cast_by: self.cast_by.clone(),
            excluded_cast_origin_zone: self.excluded_cast_origin_zone,
            first_spell_cast_each_turn: self.first_spell_cast_each_turn,
            spell_cast_ordinal_each_turn: self.spell_cast_ordinal_each_turn,
            single_graveyard: self.single_graveyard,
            ..Self::default()
        };
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

    pub fn protected_by(mut self, player: PlayerFilter) -> Self {
        self.protected_by = Some(player);
        self
    }

    pub fn cast_by(mut self, caster: PlayerFilter) -> Self {
        self.cast_by = Some(caster);
        self
    }

    pub fn cast_by_you(self) -> Self {
        self.cast_by(PlayerFilter::You)
    }

    pub fn not_cast_from_zone(mut self, zone: Zone) -> Self {
        self.excluded_cast_origin_zone = Some(zone);
        self
    }

    pub fn first_spell_cast_each_turn(mut self) -> Self {
        self.first_spell_cast_each_turn = true;
        self
    }

    pub fn spell_cast_ordinal_each_turn(mut self, ordinal: u32) -> Self {
        self.spell_cast_ordinal_each_turn = Some(ordinal);
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

    pub fn with_all_subtype(mut self, subtype: Subtype) -> Self {
        self.all_subtypes.push(subtype);
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

    pub fn of_colors_chosen_while_drafting_named(mut self, card_name: impl Into<String>) -> Self {
        self.colors_chosen_while_drafting_named = Some(card_name.into());
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

    pub fn with_counters_put_on_this_turn(
        mut self,
        constraint: CountersPutOnThisTurnConstraint,
    ) -> Self {
        self.counters_put_on_this_turn = Some(constraint);
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

    pub fn not_of_any_chosen_creature_type(mut self) -> Self {
        self.excluded_any_chosen_creature_type = true;
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

    pub fn suspected(mut self) -> Self {
        self.suspected = true;
        self
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn not_named(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.excluded_name_surface = ExcludedNameSurface::new(name.clone());
        self.excluded_name = Some(name);
        self
    }

    pub fn set_excluded_name_surface(&mut self, surface: impl Into<String>) {
        self.excluded_name_surface = ExcludedNameSurface::new(surface);
    }

    pub fn excluded_name_surface(&self) -> Option<&str> {
        self.excluded_name_surface.as_deref()
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

    pub fn sharing_characteristics_with(
        mut self,
        characteristics: Vec<ObjectCharacteristic>,
        comparison: ObjectFilter,
    ) -> Self {
        self.characteristic_relations
            .push(ObjectCharacteristicRelation::shares(
                characteristics,
                comparison,
            ));
        self
    }

    pub fn sharing_no_characteristics_with(
        mut self,
        characteristics: Vec<ObjectCharacteristic>,
        comparison: ObjectFilter,
    ) -> Self {
        self.characteristic_relations
            .push(ObjectCharacteristicRelation::shares_none(
                characteristics,
                comparison,
            ));
        self
    }

    pub fn with_tap_activated_ability(mut self) -> Self {
        self.has_tap_activated_ability = true;
        self
    }

    pub fn match_tagged(mut self, tag: impl Into<TagKey>, relation: TaggedOpbjectRelation) -> Self {
        let constraint = TaggedObjectConstraint {
            tag: tag.into(),
            relation,
        };
        if !self.tagged_constraints.contains(&constraint) {
            self.tagged_constraints.push(constraint);
        }
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
        if let Some(description) = describe_relative_characteristic_list_filter(self) {
            return description;
        }
        if let Some(description) = describe_characteristic_or_mana_production_union(self) {
            return description;
        }
        if let Some(description) = describe_branch_scoped_card_type_union(self) {
            return description;
        }
        if let Some(description) =
            describe_controlled_battlefield_and_owned_nonbattlefield_card_union(self)
        {
            return description;
        }
        if let Some(description) = describe_owned_nonbattlefield_card_union(self) {
            return description;
        }
        if let Some(description) = describe_owner_scoped_zone_union(self) {
            return description;
        }
        if let Some(description) = describe_you_own_or_control_union(self) {
            return description;
        }
        if let Some(description) = describe_possessive_commander_subject(self) {
            return description;
        }
        if let Some(description) = describe_exact_mana_cost_union(self) {
            return description;
        }
        if any_of_keyword_clause.is_none() && !self.any_of.is_empty() {
            let explicit_branch_articles = self.has_explicit_union_branch_articles();
            let branch_descriptions = self
                .any_of
                .iter()
                .map(|branch| {
                    let mut described_branch = branch.clone();
                    if described_branch.controller.is_none() {
                        described_branch.controller = self.controller.clone();
                    }
                    if described_branch.owner.is_none() {
                        described_branch.owner = self.owner.clone();
                    }
                    if self.other {
                        described_branch.other = true;
                    }
                    described_branch.description()
                })
                .map(correct_leading_indefinite_article)
                .map(|description| {
                    if explicit_branch_articles {
                        ensure_indefinite_article(description)
                    } else {
                        description
                    }
                })
                .collect();
            let mut description = if self.has_conjunctive_set_surface() {
                describe_conjunctive_filter_list(branch_descriptions)
            } else {
                describe_filter_union_list(
                    branch_descriptions,
                    self.union_connective(),
                    explicit_branch_articles,
                )
            };
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
        if self.is_target_object {
            parts.push("target".to_string());
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
        if self.suspected {
            parts.push("suspected".to_string());
        }

        let has_leading_determiner =
            self.other || self.is_target_object || has_target_tag || has_chosen_tag || self.source;

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
                PlayerFilter::AttackedBySourceThisTurn => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::WasDealtDamageBySourceThisGame { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::LostLifeThisTurn { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. } => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    post_noun_qualifiers.push(format!("controlled by {}", ctrl.description()));
                }
                PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::HasMoreLifeThanYou { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::ControlsMost { .. } => {
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
                PlayerFilter::PlayerToYourLeft | PlayerFilter::PlayerToYourRight => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::Defending => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("defending player controls".to_string());
                }
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
                PlayerFilter::Excluding { .. } if ctrl.is_your_team() => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("your team controls".to_string());
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
                    let target_kind = inner_desc
                        .strip_prefix("a ")
                        .or_else(|| inner_desc.strip_prefix("an "))
                        .unwrap_or(&inner_desc);
                    controller_suffix = Some(format!("target {target_kind} controls"));
                }
                PlayerFilter::AliasedTarget(_) => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("that player controls".to_string());
                }
                PlayerFilter::ControllerOf(_) => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("its controller controls".to_string());
                }
                PlayerFilter::OwnerOf(_) => parts.push("an owner's".to_string()),
                PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
                    parts.push("that player's".to_string())
                }
            }
        }

        if let Some(cast_by) = &self.cast_by {
            post_noun_qualifiers.push(format!("cast by {}", describe_player_filter(cast_by)));
        }
        if let Some(zone) = self.excluded_cast_origin_zone {
            let origin = match zone {
                Zone::Hand => "its owner's hand".to_string(),
                Zone::Graveyard => "a graveyard".to_string(),
                Zone::Library => "a library".to_string(),
                Zone::Exile => "exile".to_string(),
                Zone::Command => "the command zone".to_string(),
                Zone::Battlefield => "the battlefield".to_string(),
                Zone::Stack => "the stack".to_string(),
                Zone::Ante => "ante".to_string(),
                Zone::OutsideGame => "outside the game".to_string(),
            };
            post_noun_qualifiers.push(format!("that wasn't cast from {origin}"));
        }
        if self.cast_this_turn {
            post_noun_qualifiers.push("cast this turn".to_string());
        }
        if self.first_spell_cast_each_turn {
            post_noun_qualifiers.push("first spell cast each turn".to_string());
        }
        if let Some(ordinal) = self.spell_cast_ordinal_each_turn {
            let word = match ordinal {
                1 => "first".to_string(),
                2 => "second".to_string(),
                3 => "third".to_string(),
                other => format!("{other}th"),
            };
            post_noun_qualifiers.push(format!("{word} spell cast each turn"));
        }
        if let Some(source_filter) = &self.mana_from_source_spent_to_cast {
            post_noun_qualifiers.push(format!(
                "that mana from {} was spent to cast",
                ensure_indefinite_article(source_filter.description())
            ));
        }

        let owner_conveyed_by_zone = matches!(
            self.zone,
            Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
        ) && !self.foretold
            && !self.has_owner_before_zone_surface();
        if !owner_conveyed_by_zone && let Some(ref owner) = self.owner {
            if controller_suffix.is_none() && !has_leading_determiner {
                parts.insert(0, "a".to_string());
            }
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
                PlayerFilter::AttackedBySourceThisTurn => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::WasDealtDamageBySourceThisGame { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::LostLifeThisTurn { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::HasMoreLifeThanYou { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::ControlsMost { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::MaxSpeed { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::ChosenPlayer => "the chosen player owns".to_string(),
                PlayerFilter::TaggedPlayer(_) => "that player owns".to_string(),
                PlayerFilter::Teammate => "a teammate owns".to_string(),
                PlayerFilter::PlayerToYourLeft | PlayerFilter::PlayerToYourRight => {
                    format!("{} owns", describe_player_filter(owner))
                }
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
        if self.foretold {
            parts.push("foretold".to_string());
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
        if let Some(card_name) = &self.colors_chosen_while_drafting_named {
            post_noun_qualifiers.push(format!(
                "that's one or more of the colors chosen as you drafted cards named {card_name}"
            ));
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
        // Chosen-quality back-references follow the controller suffix in
        // oracle order ("creatures you control of the chosen type") — but
        // only when there IS one; zone qualifiers ("cards of that type from
        // their graveyard") keep the chosen phrase next to the noun.
        let defer_chosen_qualifiers = controller_suffix.is_some() || owner_suffix.is_some();
        let mut chosen_trailing_qualifiers: Vec<String> = Vec::new();
        if self.chosen_creature_type {
            let qualifier = if self.has_chosen_type_this_way_surface() {
                "of a type chosen this way"
            } else {
                "of the chosen type"
            };
            if defer_chosen_qualifiers {
                chosen_trailing_qualifiers.push(qualifier.to_string());
            } else {
                post_noun_qualifiers.push(qualifier.to_string());
            }
        }
        if self.chosen_card_type {
            if defer_chosen_qualifiers {
                chosen_trailing_qualifiers.push("of the chosen type".to_string());
            } else {
                post_noun_qualifiers.push("of the chosen type".to_string());
            }
        }
        if let Some(set_name) = &self.name_originally_printed_in_set {
            post_noun_qualifiers.push(format!(
                "with a name originally printed in the {set_name} expansion"
            ));
        }
        if self.excluded_chosen_creature_type || self.excluded_any_chosen_creature_type {
            let qualifier = if self.has_chosen_type_this_way_surface() {
                "that aren't of a type chosen this way"
            } else {
                "that aren't of the chosen type"
            };
            if defer_chosen_qualifiers {
                chosen_trailing_qualifiers.push(qualifier.to_string());
            } else {
                post_noun_qualifiers.push(qualifier.to_string());
            }
        }
        if !self.no_shared_creature_types_with.is_empty() {
            let comparison = self
                .no_shared_creature_types_with
                .iter()
                .map(|filter| ensure_indefinite_article(filter.description()))
                .collect::<Vec<_>>()
                .join(" or ");
            post_noun_qualifiers.push(format!(
                "that doesn't share a creature type with {comparison}"
            ));
        }
        for relation in &self.characteristic_relations {
            let characteristics = relation
                .characteristics
                .iter()
                .map(|characteristic| characteristic.sharing_phrase())
                .collect::<Vec<_>>()
                .join(" or ");
            let verb = match relation.kind {
                ObjectCharacteristicRelationKind::SharesAny => "shares",
                ObjectCharacteristicRelationKind::SharesNone => "doesn't share",
            };
            post_noun_qualifiers.push(format!(
                "that {verb} {characteristics} with {}",
                relation.comparison_description()
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
                TaggedOpbjectRelation::SameManaValueAsAnotherTagged => {
                    post_noun_qualifiers
                        .push("with the same mana value as another tagged object".to_string());
                }
                TaggedOpbjectRelation::ManaValueLteTagged => {
                    if self.union_surface.equal_or_lesser_mana_value() {
                        post_noun_qualifiers.push("with equal or lesser mana value".to_string());
                    } else if constraint.tag.as_str() == "triggering" {
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
                TaggedOpbjectRelation::WasAttachedToTaggedObject => {
                    post_noun_qualifiers.push("that was attached to it".to_string());
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
            for (index, card_type) in self.excluded_card_types.iter().enumerate() {
                let comma = (index + 1 < self.excluded_card_types.len()).then_some(",");
                parts.push(format!(
                    "non{}{}",
                    describe_card_type_word(*card_type),
                    comma.unwrap_or_default()
                ));
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
            // Oracle order puts the color-count clause after the controller
            // suffix ("permanents you control that are exactly two colors").
            let clause = if exactly_two_colors {
                "that are exactly two colors".to_string()
            } else {
                "that are not exactly two colors".to_string()
            };
            if defer_chosen_qualifiers {
                chosen_trailing_qualifiers.push(clause);
            } else {
                post_noun_qualifiers.push(clause);
            }
        }
        if self.historic {
            parts.push("historic".to_string());
        }
        if self.nonhistoric {
            post_noun_qualifiers.push("that's not historic".to_string());
        }
        if self.is_commander
            && !(self.card_types.is_empty()
                && self.all_card_types.is_empty()
                && self.subtypes.is_empty()
                && self.all_subtypes.is_empty()
                && !self.token)
        {
            // With no type noun, "commander" IS the noun ("Commanders you
            // control"), handled by the default-noun selection below.
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
        if let Some(combat_partner) = &self.blocked_or_was_blocked_by_this_turn {
            let mut partner_description = combat_partner.description();
            if combat_partner.card_types.as_slice() == [CardType::Creature]
                && !combat_partner.subtypes.is_empty()
            {
                partner_description = partner_description.replacen(" creature", "", 1);
            }
            post_noun_qualifiers.push(format!(
                "that blocked or was blocked by {} this turn",
                ensure_indefinite_article(partner_description)
            ));
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
            if self.attacking
                && !self.attacking_alone
                && self
                    .attacking_player_or_planeswalker_controlled_by
                    .is_none()
            {
                parts.push("attacking".to_string());
            }
            if self.blocking && !self.in_combat_with_source && self.in_combat_with.is_none() {
                parts.push("blocking".to_string());
            }
        }
        if self.attacking_alone {
            post_noun_qualifiers.push("that's attacking alone".to_string());
        }
        if self.attacked_this_turn {
            post_noun_qualifiers.push("that attacked this turn".to_string());
        }
        if self.ability_activated_this_turn {
            let clause = if self.card_types == [CardType::Planeswalker] {
                "that was activated this turn"
            } else {
                "that had an ability activated this turn"
            };
            post_noun_qualifiers.push(clause.to_string());
        }
        if self.blocked_this_turn {
            post_noun_qualifiers.push("that blocked this turn".to_string());
        }
        if self.didnt_attack_this_turn {
            let clause = if self.could_have_attacked_this_turn {
                "that didn't attack this turn, except for creatures that couldn't attack"
            } else if self.didnt_enter_battlefield_this_turn {
                "that didn't attack or enter this turn"
            } else {
                "that didn't attack this turn"
            };
            post_noun_qualifiers.push(clause.to_string());
        } else if self.could_have_attacked_this_turn {
            post_noun_qualifiers.push("that could have attacked this turn".to_string());
        }
        if let Some(player_filter) = &self.attacking_player_or_planeswalker_controlled_by {
            if matches!(player_filter, PlayerFilter::Opponent) && !self.attacking_player_only {
                post_noun_qualifiers
                    .push("attacking your opponents and/or planeswalkers they control".to_string());
            } else {
                let player_text = if matches!(player_filter, PlayerFilter::ChosenPlayer) {
                    "the last chosen player".to_string()
                } else {
                    player_filter.description()
                };
                if self.attacking_player_only {
                    let relation = if matches!(player_filter, PlayerFilter::ChosenPlayer) {
                        format!("attacking {player_text}")
                    } else {
                        format!("that's attacking {player_text}")
                    };
                    post_noun_qualifiers.push(relation);
                } else {
                    let controller_pronoun = if matches!(player_filter, PlayerFilter::You) {
                        "you"
                    } else {
                        "they"
                    };
                    post_noun_qualifiers.push(format!(
                        "that's attacking {player_text} or a planeswalker {controller_pronoun} control"
                    ));
                }
            }
        }
        if let Some(player_filter) = &self.protected_by {
            let player = match player_filter {
                PlayerFilter::IteratedPlayer => "that player".to_string(),
                other => other.description(),
            };
            post_noun_qualifiers.push(format!("{player} protects"));
        }
        if self.in_combat_with_source {
            post_noun_qualifiers.push(if self.blocking {
                "blocking this creature".to_string()
            } else {
                "blocking or blocked by this creature".to_string()
            });
        }
        if let Some(reference) = &self.in_combat_with {
            let reference = match reference {
                ObjectRef::Target => "target creature",
                ObjectRef::Specific(_) => "that creature",
                ObjectRef::Tagged(tag) if tag.as_str() == "blocking" => "the blocking creature",
                ObjectRef::Tagged(_) => "that creature",
            };
            post_noun_qualifiers.push(if self.blocking {
                format!("blocking {reference}")
            } else {
                format!("blocking or blocked by {reference}")
            });
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
        if self.didnt_enter_battlefield_this_turn && !self.didnt_attack_this_turn {
            post_noun_qualifiers.push("that didn't enter this turn".to_string());
        }
        if self.no_abilities {
            post_noun_qualifiers.push("with no abilities".to_string());
        }
        if !self.could_produce_mana.is_empty() {
            post_noun_qualifiers.push(format!(
                "that could produce {}",
                describe_mana_symbol_list(&self.could_produce_mana)
            ));
        }

        let subtype_implies_type = (!self.subtypes.is_empty() || !self.all_subtypes.is_empty())
            && matches!(self.zone, None | Some(Zone::Battlefield))
            && self.all_card_types.is_empty()
            && self.card_types.is_empty();
        let has_all_permanent_types = self.has_all_permanent_card_types();
        let has_all_permanent_spell_types = self.has_all_permanent_spell_card_types();
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
            } else if has_all_permanent_types || has_all_permanent_spell_types {
                Some((true, "permanent".to_string()))
            } else {
                let card_type_phrase = if self.has_conjunctive_set_surface() {
                    describe_conjunctive_filter_list(
                        self.card_types
                            .iter()
                            .map(|card_type| card_type.name().to_string())
                            .collect(),
                    )
                } else {
                    describe_card_type_list(&self.card_types, self.union_connective())
                };
                Some((true, card_type_phrase))
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
                    Some(Zone::Battlefield) | None if self.is_commander => "commander",
                    Some(Zone::Battlefield) | None => "permanent",
                    Some(Zone::Stack) => {
                        let kind = self.stack_kind.unwrap_or({
                            if self.has_mana_cost {
                                StackObjectKind::Spell
                            } else {
                                StackObjectKind::SpellOrAbility
                            }
                        });
                        if kind == StackObjectKind::SpellOrAbility
                            && self.has_conjunctive_set_surface()
                        {
                            "spell and ability"
                        } else {
                            describe_stack_object_kind(kind)
                        }
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

        let subtype_parts = if !self.subtypes.is_empty() || !self.all_subtypes.is_empty() {
            let mut parts = if self.all_subtypes.is_empty() {
                Vec::new()
            } else {
                vec![
                    self.all_subtypes
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" "),
                ]
            };
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
            // A subtype that the same filter also excludes is already spelled out by
            // the "non-<subtype>" prefix, so repeating it as the noun doubles the
            // word ("non-human Human creature"). Only drop it when a card type is
            // still around to supply the noun, since a subtype-only filter leans on
            // the subtype word itself.
            if !self.excluded_subtypes.is_empty() && !subtype_implies_type {
                remaining.retain(|subtype| !self.excluded_subtypes.contains(subtype));
            }
            parts.extend(remaining.iter().map(std::string::ToString::to_string));
            parts
        } else {
            Vec::new()
        };
        let subtype_phrase = (!subtype_parts.is_empty()).then(|| {
            let description = if self.has_conjunctive_set_surface() {
                describe_conjunctive_filter_list(subtype_parts.clone())
            } else {
                describe_filter_union_list(
                    subtype_parts.clone(),
                    self.union_connective(),
                    self.has_serial_or_list_surface(),
                )
            };
            if self.has_shared_indefinite_article_surface() {
                ensure_indefinite_article(description)
            } else {
                description
            }
        });

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
        if self.has_plural_object_noun_surface()
            && matches!(
                self.attacking_player_or_planeswalker_controlled_by,
                Some(PlayerFilter::Opponent)
            )
            && !self.attacking_player_only
            && let Some((_, phrase)) = type_phrase.as_mut()
        {
            *phrase = pluralize_count_terminal_word(phrase);
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
                (Some((_, mut type_phrase)), Some(subtype_phrase)) => {
                    let terminal_noun = self
                        .has_terminal_noun_after_type_subtype_union_surface()
                        .then(|| {
                            [" card", " spell"].into_iter().find_map(|suffix| {
                                type_phrase
                                    .strip_suffix(suffix)
                                    .map(|head| (head.to_string(), suffix.trim().to_string()))
                            })
                        })
                        .flatten();
                    if let Some((head, _)) = &terminal_noun {
                        type_phrase = head.clone();
                    }

                    let subtype_first = self.has_subtype_before_card_type_union_surface();
                    let preserve_individual_arms = subtype_first || terminal_noun.is_some();
                    let mut union_parts = if preserve_individual_arms {
                        subtype_parts
                    } else {
                        vec![subtype_phrase]
                    };
                    if subtype_first {
                        union_parts.push(type_phrase);
                    } else {
                        union_parts.insert(0, type_phrase);
                    }
                    let mut description = describe_filter_union_list(
                        union_parts,
                        self.union_connective(),
                        preserve_individual_arms,
                    );
                    if let Some((_, terminal_noun)) = terminal_noun {
                        description.push(' ');
                        description.push_str(&terminal_noun);
                    }
                    parts.push(description);
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
                (Some((_, type_phrase)), Some(subtype_phrase)) if land_only => {
                    parts.push(subtype_phrase);
                    if self.explicit_card_type_noun() == Some(CardType::Land) {
                        parts.push(type_phrase);
                    } else if self.has_explicit_card_noun()
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

        // Oracle places controller and owner scope immediately after the noun,
        // before restrictive qualifiers: "a creature you control with
        // deathtouch", not "a creature with deathtouch you control". Keep the
        // scope attached to the noun here so every later AST-derived
        // qualifier follows it consistently.
        match (controller_suffix.take(), owner_suffix.take()) {
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
                if controller == "you don't control" && owner == "you don't own" =>
            {
                parts.push("you neither own nor control".to_string());
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

        if !post_noun_qualifiers.is_empty() {
            // Independent reference-resolution paths can retain aliases for
            // the same tagged producer (for example, both a sacrifice result
            // tag and its cost tag). Keep every typed constraint in the AST,
            // but do not repeat an identical English qualifier when those
            // aliases describe the same relation.
            let mut rendered_qualifiers = Vec::with_capacity(post_noun_qualifiers.len());
            for qualifier in &post_noun_qualifiers {
                if !rendered_qualifiers.contains(qualifier) {
                    rendered_qualifiers.push(qualifier.clone());
                }
            }
            parts.extend(rendered_qualifiers);
        }
        if self.distinct_names {
            parts.push("with different names".to_string());
        }
        if self.distinct_mana_values {
            parts.push("with different mana values".to_string());
        }
        if self.distinct_powers {
            parts.push("with different powers".to_string());
        }
        if self.distinct_creature_types {
            parts.push("that share no creature types".to_string());
        }
        if self.one_per_card_type {
            parts.push("with at most one card of each card type".to_string());
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
                return format!(
                    "{} with that name",
                    ensure_indefinite_article(parts.join(" "))
                );
            }
            return format!(
                "{} named {}",
                ensure_indefinite_article(parts.join(" ")),
                name
            );
        }
        if let Some(ref name) = self.excluded_name {
            let name = self.excluded_name_surface().unwrap_or(name);
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
                    PowerToughnessRelation::NotEqual => {
                        parts.push("with power and toughness that aren't equal".to_string());
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
        if let Some(ref exact_mana_cost) = self.exact_mana_cost {
            parts.push(format!("with mana cost {}", exact_mana_cost.to_oracle()));
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
        if let Some(constraint) = self.target_set_aggregate_constraint.as_deref() {
            let metric = match constraint.metric {
                crate::ChoiceAggregateMetric::Power => "power",
                crate::ChoiceAggregateMetric::Toughness => "toughness",
                crate::ChoiceAggregateMetric::ManaValue => "mana value",
            };
            if let Some(minimum) = constraint.minimum.as_ref() {
                let minimum = match minimum.unhinted() {
                    Value::Fixed(minimum) => format!("{minimum} or greater"),
                    _ => describe_comparison(&Comparison::GreaterThanOrEqualExpr(Box::new(
                        minimum.clone(),
                    ))),
                };
                parts.push(format!("with total {metric} {minimum}"));
            } else {
                let maximum = match constraint.maximum.unhinted() {
                    Value::Fixed(maximum) => format!("{maximum} or less"),
                    _ => describe_comparison(&Comparison::LessThanOrEqualExpr(Box::new(
                        constraint.maximum.clone(),
                    ))),
                };
                parts.push(format!("with total {metric} {maximum}"));
            }
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
        if self.has_phyrexian_mana_symbol {
            parts.push("with {H} in its mana cost".to_string());
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
        if self.excluded_static_abilities.len() > 1 {
            // Oracle writes a multi-keyword exclusion as one serial clause
            // ("that doesn't have first strike, double strike, vigilance, or
            // haste"), never as repeated "without" parts.
            let labels = self
                .excluded_static_abilities
                .iter()
                .filter_map(|ability| describe_filter_static_ability(*ability))
                .collect::<Vec<_>>();
            if let [leading @ .., last] = labels.as_slice()
                && !leading.is_empty()
            {
                parts.push(format!(
                    "that doesn't have {}, or {last}",
                    leading.join(", ")
                ));
            }
        } else {
            for ability in &self.excluded_static_abilities {
                if let Some(label) = describe_filter_static_ability(*ability) {
                    parts.push(format!("without {}", label));
                }
            }
        }
        for marker in &self.excluded_ability_markers {
            parts.push(format!("without {}", marker.to_ascii_lowercase()));
        }
        if let Some(counter_requirement) = self
            .with_counter
            .filter(|_| !self.has_counter_requirement_after_zone_surface())
        {
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
                if self.foretold && zone == Zone::Exile {
                    parts.push("in exile".to_string());
                } else if self.has_owner_before_zone_surface() {
                    parts.push(format!("in {}", zone_name));
                } else if let Some(owner) = &self.owner {
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

        if let Some(counter_requirement) = self
            .with_counter
            .filter(|_| self.has_counter_requirement_after_zone_surface())
        {
            let (one_or_more, plural_noun, plural_subject) = self.counter_requirement_surface();
            parts.push(format!(
                "with {}{} on {}",
                if one_or_more { "one or more " } else { "" },
                describe_counter_constraint(counter_requirement, plural_noun),
                if plural_subject { "them" } else { "it" }
            ));
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
            } else if self.has_entered_battlefield_explicit_surface() {
                "that entered the battlefield this turn".to_string()
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

        if self.entered_graveyard_from_library_this_turn && self.zone == Some(Zone::Graveyard) {
            parts.push("that was put there from their library this turn".to_string());
        } else if self.entered_graveyard_from_battlefield_this_turn
            && self.zone == Some(Zone::Graveyard)
        {
            parts.push("that was put there from the battlefield this turn".to_string());
        } else if self.entered_graveyard_this_turn && self.zone == Some(Zone::Graveyard) {
            let clause = match self.graveyard_entry_history_surface() {
                Some(GraveyardEntryHistorySurface::PutThereFromAnywhereThisTurn) => {
                    "that was put there from anywhere this turn"
                }
                _ => "that was put there this turn",
            };
            parts.push(clause.to_string());
        }
        if self.surveilled_this_turn {
            parts.push("you've surveilled this turn".to_string());
        }
        if let Some(constraint) = &self.counters_put_on_this_turn {
            parts.push(describe_counters_put_on_this_turn_constraint(constraint));
        }
        if let Some(player) = &self.discarded_or_cycled_this_turn_by {
            let actor = describe_player_filter(player);
            parts.push(format!("{actor} cycled or discarded this turn"));
        }

        if self.was_dealt_damage_this_turn {
            parts.push("that was dealt damage this turn".to_string());
        }
        if self.dealt_damage_this_turn {
            parts.push("that dealt damage this turn".to_string());
        }
        if let Some(damager) = &self.dealt_damage_by_source_this_turn {
            let source = match damager {
                crate::DamagedBySource::ThisCreature => "this creature",
                crate::DamagedBySource::EquippedCreature => "equipped creature",
                crate::DamagedBySource::EnchantedCreature => "enchanted creature",
            };
            parts.push(format!("that was dealt damage by {source} this turn"));
        }
        if self.was_dealt_damage_by_source_this_game {
            parts.push("that this source has dealt damage to this game".to_string());
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

        parts.extend(chosen_trailing_qualifiers);

        if let Some(attached_to) = &self.attached_to_object {
            parts.push(format!(
                "attached to {}",
                ensure_indefinite_article(attached_to.description())
            ));
        }
        if let Some(with_attached) = &self.with_attached_object {
            let inner = with_attached.description();
            let surfaced = if inner.starts_with("another ") || inner.starts_with("other ") {
                inner
            } else {
                ensure_indefinite_article(inner)
            };
            parts.push(format!("with {surfaced} attached to it"));
        }
        if let Some(without_attached) = &self.without_attached_object {
            let is_aura = without_attached.zone == Some(Zone::Battlefield)
                && without_attached.card_types == [CardType::Enchantment]
                && without_attached.subtypes == [Subtype::Aura]
                && {
                    let mut semantic = (**without_attached).clone();
                    semantic.zone = None;
                    semantic.card_types.clear();
                    semantic.subtypes.clear();
                    semantic == ObjectFilter::default()
                };
            if is_aura {
                parts.push("that isn't enchanted".to_string());
            } else {
                parts.push(format!(
                    "without {} attached to it",
                    ensure_indefinite_article(without_attached.description())
                ));
            }
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

        correct_leading_indefinite_article(parts.join(" "))
    }
}

fn describe_mana_symbol_for_filter(symbol: ManaSymbol) -> String {
    match symbol {
        ManaSymbol::White => "{W}".to_string(),
        ManaSymbol::Blue => "{U}".to_string(),
        ManaSymbol::Black => "{B}".to_string(),
        ManaSymbol::Red => "{R}".to_string(),
        ManaSymbol::Green => "{G}".to_string(),
        ManaSymbol::Colorless => "{C}".to_string(),
        ManaSymbol::Generic(value) => format!("{{{value}}}"),
        ManaSymbol::Snow => "{S}".to_string(),
        ManaSymbol::Life(_) => "{P}".to_string(),
        ManaSymbol::X => "{X}".to_string(),
    }
}

fn describe_mana_symbol_list(symbols: &[ManaSymbol]) -> String {
    describe_filter_union_list(
        symbols
            .iter()
            .copied()
            .map(describe_mana_symbol_for_filter)
            .collect(),
        ObjectFilterUnionConnective::Or,
        false,
    )
}

/// Factor a shared object domain around a characteristic/capability union.
///
/// This is the executable shape of phrases such as "land that is snow or
/// could produce {C}": the land domain applies to both arms, while being snow
/// and mana-production capability remain independent predicates.
fn describe_characteristic_or_mana_production_union(filter: &ObjectFilter) -> Option<String> {
    let [first, second] = filter.any_of.as_slice() else {
        return None;
    };

    fn supertype_branch(branch: &ObjectFilter) -> Option<Supertype> {
        let [supertype] = branch.supertypes.as_slice() else {
            return None;
        };
        let mut remainder = branch.clone();
        remainder.supertypes.clear();
        (remainder == ObjectFilter::default()).then_some(*supertype)
    }

    fn mana_branch(branch: &ObjectFilter) -> Option<&[ManaSymbol]> {
        if branch.could_produce_mana.is_empty() {
            return None;
        }
        let mut remainder = branch.clone();
        remainder.could_produce_mana.clear();
        (remainder == ObjectFilter::default()).then_some(branch.could_produce_mana.as_slice())
    }

    let (supertype, mana) = supertype_branch(first)
        .zip(mana_branch(second))
        .or_else(|| supertype_branch(second).zip(mana_branch(first)))?;
    let mut shared = filter.clone();
    shared.any_of.clear();
    let subject = shared.description();
    let copula = if shared.has_plural_object_noun_surface() {
        "are"
    } else {
        "is"
    };
    Some(format!(
        "{subject} that {copula} {} or could produce {}",
        supertype.name(),
        describe_mana_symbol_list(mana)
    ))
}

fn source_reference_surface_text(surface: &SourceReferenceSurface) -> String {
    surface.display_text()
}

/// Compact a controlled battlefield set coordinated with the same class of
/// owned cards in every nonbattlefield zone.
pub fn describe_controlled_battlefield_and_owned_nonbattlefield_card_union(
    filter: &ObjectFilter,
) -> Option<String> {
    if !filter.has_conjunctive_set_surface() || filter.any_of.len() != 6 {
        return None;
    }

    let mut outer = filter.clone();
    outer.any_of.clear();
    outer.union_surface = ObjectFilterUnionSurface::default();
    if outer != ObjectFilter::default() {
        return None;
    }

    let battlefield_index = filter.any_of.iter().position(|branch| {
        branch.zone == Some(Zone::Battlefield)
            && branch.controller == Some(PlayerFilter::You)
            && branch.owner.is_none()
    })?;
    let battlefield = &filter.any_of[battlefield_index];
    let mut battlefield_basis = battlefield.clone();
    battlefield_basis.zone = None;
    battlefield_basis.controller = None;

    let nonbattlefield_branches = filter
        .any_of
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != battlefield_index)
        .map(|(_, branch)| branch.clone())
        .collect::<Vec<_>>();
    for branch in &nonbattlefield_branches {
        let mut basis = branch.clone();
        basis.zone = None;
        basis.owner = None;
        if basis != battlefield_basis {
            return None;
        }
    }
    let nonbattlefield = ObjectFilter {
        any_of: nonbattlefield_branches,
        ..ObjectFilter::default()
    };
    let owned_cards = describe_owned_nonbattlefield_card_union(&nonbattlefield)?;

    let mut battlefield_surface = battlefield.clone();
    battlefield_surface.set_plural_object_noun_surface(false);
    Some(format!(
        "{} and {owned_cards}",
        describe_count_filter_subject(&battlefield_surface)
    ))
}

/// Compact the five owner-scoped nonbattlefield zones back into Oracle's
/// canonical "cards you own that aren't on the battlefield" subject.
///
/// The branches remain separate typed runtime selectors. This renderer only
/// applies when all five branches have the same object constraints and differ
/// solely by zone.
pub fn describe_owned_nonbattlefield_card_union(filter: &ObjectFilter) -> Option<String> {
    if filter.union_connective() != ObjectFilterUnionConnective::Or || filter.any_of.len() != 5 {
        return None;
    }

    let mut outer = filter.clone();
    outer.any_of.clear();
    outer.union_surface = ObjectFilterUnionSurface::default();
    if outer != ObjectFilter::default() {
        return None;
    }

    let mut seen = [false; 5];
    let mut shared_basis: Option<ObjectFilter> = None;
    for branch in &filter.any_of {
        if branch.owner != Some(PlayerFilter::You) || branch.controller.is_some() {
            return None;
        }
        let zone_index = match branch.zone? {
            Zone::Hand => 0,
            Zone::Library => 1,
            Zone::Graveyard => 2,
            Zone::Exile => 3,
            Zone::Command => 4,
            _ => return None,
        };
        if std::mem::replace(&mut seen[zone_index], true) {
            return None;
        }

        let mut basis = branch.clone();
        basis.zone = None;
        basis.owner = None;
        match &shared_basis {
            Some(shared) if shared != &basis => return None,
            Some(_) => {}
            None => shared_basis = Some(basis),
        }
    }
    if !seen.into_iter().all(|present| present) {
        return None;
    }

    let mut shared_basis = shared_basis?;
    shared_basis.set_plural_object_noun_surface(false);
    let description = shared_basis.description();
    let noun = description
        .strip_prefix("an ")
        .or_else(|| description.strip_prefix("a "))
        .or_else(|| description.strip_prefix("the "))
        .unwrap_or(&description)
        .trim();
    let noun = noun.strip_suffix(" card")?.trim();
    if noun.is_empty() {
        Some("cards you own that aren't on the battlefield".to_string())
    } else {
        Some(format!(
            "{noun} cards you own that aren't on the battlefield"
        ))
    }
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
        union_surface: filter.union_surface,
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

/// Compact mirrored ownership/controller branches while preserving their
/// shared object constraints.
///
/// `(permanent owned by you) OR (permanent controlled by you)` is the typed
/// form of Oracle's "a permanent you own or control". The two branches must
/// otherwise be identical so genuinely distinct selectors remain expanded.
/// Oracle uses the possessive for the bare commander subject — "Whenever **your
/// commander** deals combat damage to a player" — and reserves the indefinite
/// "a commander you own" for the cross-zone value reference, e.g. Cloudkill's
/// "the greatest mana value of a commander you own on the battlefield or in the
/// command zone". Those value references are zone unions whose arms do not carry
/// `owner`, so requiring an explicit `owner: You` on a bare, otherwise-unqualified
/// commander filter separates the two without a scorer rewrite (a REWRITES pair
/// collapsing the two spellings damaged Cloudkill and Majestic Genesis, whose
/// oracle really does say "a commander you own").
fn describe_possessive_commander_subject(filter: &ObjectFilter) -> Option<String> {
    if !filter.is_commander
        || filter.owner != Some(PlayerFilter::You)
        || !matches!(filter.controller, None | Some(PlayerFilter::You))
        || !matches!(filter.zone, None | Some(Zone::Battlefield))
        || !filter.any_of.is_empty()
    {
        return None;
    }
    // Anything the possessive cannot express keeps the spelled-out form.
    if !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.all_subtypes.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.tagged_constraints.is_empty()
        || filter.name.is_some()
        || filter.specific.is_some()
        || filter.token
        || filter.nontoken
        || filter.other
        || filter.power.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.exact_mana_cost.is_some()
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.colors.is_some()
        || filter.attacking
        || filter.attacking_alone
        || filter.blocking
        || filter.tapped
        || filter.untapped
    {
        return None;
    }
    Some("your commander".to_string())
}

fn describe_you_own_or_control_union(filter: &ObjectFilter) -> Option<String> {
    if filter.union_connective() != ObjectFilterUnionConnective::Or {
        return None;
    }
    let mut outer = filter.clone();
    outer.any_of.clear();
    outer.union_surface = ObjectFilterUnionSurface::default();
    if outer != ObjectFilter::default() {
        return None;
    }
    let [first, second] = filter.any_of.as_slice() else {
        return None;
    };

    let (owned, controlled) = if first.owner == Some(PlayerFilter::You)
        && first.controller.is_none()
        && second.controller == Some(PlayerFilter::You)
        && second.owner.is_none()
    {
        (first, second)
    } else if second.owner == Some(PlayerFilter::You)
        && second.controller.is_none()
        && first.controller == Some(PlayerFilter::You)
        && first.owner.is_none()
    {
        (second, first)
    } else {
        return None;
    };

    let mut owned_base = owned.clone();
    owned_base.owner = None;
    let mut controlled_base = controlled.clone();
    controlled_base.controller = None;
    if owned_base != controlled_base {
        return None;
    }

    Some(format!(
        "{} you own or control",
        ensure_indefinite_article(owned_base.description())
    ))
}

fn relative_characteristic_selector_description(filter: &ObjectFilter) -> Option<String> {
    if !filter.any_of.is_empty() {
        return None;
    }
    let description = match (
        filter.card_types.as_slice(),
        filter.subtypes.as_slice(),
        filter.token,
    ) {
        ([card_type], [], false) => card_type.name().to_string(),
        ([], [subtype], false) => subtype.to_string(),
        ([], [], true) => "token".to_string(),
        _ => return None,
    };

    let mut remainder = filter.clone();
    remainder.card_types.clear();
    remainder.subtypes.clear();
    remainder.token = false;
    (remainder == ObjectFilter::default()).then_some(description)
}

pub fn describe_relative_characteristic_list_filter(filter: &ObjectFilter) -> Option<String> {
    if !filter.has_relative_characteristic_list_surface() {
        return None;
    }

    let mut base = filter.clone();
    let (selectors, negated) = if filter.any_of.len() >= 2 {
        let selectors = filter
            .any_of
            .iter()
            .map(relative_characteristic_selector_description)
            .collect::<Option<Vec<_>>>()?;
        base.any_of.clear();
        (selectors, false)
    } else if filter.subtypes.len() >= 2 && filter.excluded_subtypes.is_empty() {
        let selectors = filter
            .subtypes
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        base.subtypes.clear();
        (selectors, false)
    } else if filter.excluded_subtypes.len() >= 2 && filter.subtypes.is_empty() {
        let selectors = filter
            .excluded_subtypes
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        base.excluded_subtypes.clear();
        (selectors, true)
    } else if filter.type_or_subtype_union && filter.card_types.len() + filter.subtypes.len() >= 2 {
        let selectors = filter
            .card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .chain(filter.subtypes.iter().map(ToString::to_string))
            .collect::<Vec<_>>();
        base.card_types.clear();
        base.subtypes.clear();
        (selectors, false)
    } else {
        return None;
    };

    base.type_or_subtype_union = false;
    base.set_relative_characteristic_list_surface(false);
    let base_description = base.description();
    let selector_description = if filter.has_explicit_union_branch_articles() {
        describe_filter_union_list(
            selectors
                .into_iter()
                .map(ensure_indefinite_article)
                .collect(),
            filter.union_connective(),
            filter.has_serial_or_list_surface(),
        )
    } else {
        ensure_indefinite_article(describe_filter_union_list(
            selectors,
            filter.union_connective(),
            filter.has_serial_or_list_surface(),
        ))
    };
    let relation = if negated { "that isn't" } else { "that's" };
    Some(format!(
        "{base_description} {relation} {selector_description}"
    ))
}

pub fn describe_branch_scoped_card_type_union(filter: &ObjectFilter) -> Option<String> {
    if filter.any_of.len() < 2 {
        return None;
    }

    // A source exclusion can belong to only one independently nouned arm,
    // as in `another creature you control or a land you control`. Factoring
    // the shared controller onto a single suffix would render that semantic
    // distinction ambiguously as `another creature or land you control`.
    // Let the general union renderer materialize the common scope on each arm
    // so the authored branch-local determiner remains explicit.
    let first_other = filter.any_of.first()?.other;
    if !filter.has_conjunctive_set_surface()
        && filter
            .any_of
            .iter()
            .skip(1)
            .any(|branch| branch.other != first_other)
    {
        return None;
    }

    let mut outer = filter.clone();
    outer.any_of.clear();
    let mut branches = filter.any_of.clone();
    // Serial characteristic lists are sometimes parsed as a left-associated
    // union tree. When the authored outer surface is conjunctive and a nested
    // node carries no semantic predicate of its own, flatten that container
    // for rendering so shared outer scope can qualify every list member.
    // The executable filter remains unchanged.
    if outer.has_conjunctive_set_surface() {
        let mut flattened = Vec::new();
        for branch in branches {
            let mut container = branch.clone();
            container.any_of.clear();
            if !branch.any_of.is_empty() && container == ObjectFilter::default() {
                flattened.extend(branch.any_of);
            } else {
                flattened.push(branch);
            }
        }
        branches = flattened;
    }
    // The filter parser can distribute a shared state adjective over every
    // union arm while retaining controller/zone scope on the outer filter.
    // Factor those identical predicates back out before validating the arms
    // as characteristic selectors, preserving both semantics and the authored
    // shared surface ("untapped artifacts and/or creatures you control").
    if branches.iter().all(|branch| branch.tapped) {
        outer.tapped = true;
        for branch in &mut branches {
            branch.tapped = false;
        }
    }
    if branches.iter().all(|branch| branch.untapped) {
        outer.untapped = true;
        for branch in &mut branches {
            branch.untapped = false;
        }
    }
    if let Some(stack_kind) = branches
        .first()
        .and_then(|branch| branch.stack_kind)
        .filter(|stack_kind| {
            branches
                .iter()
                .all(|branch| branch.stack_kind == Some(*stack_kind))
        })
    {
        outer.stack_kind = Some(stack_kind);
        for branch in &mut branches {
            branch.stack_kind = None;
        }
    }
    if branches.iter().all(|branch| branch.has_mana_cost) {
        outer.has_mana_cost = true;
        for branch in &mut branches {
            branch.has_mana_cost = false;
        }
    }
    if branches
        .iter()
        .all(|branch| branch.has_phyrexian_mana_symbol)
    {
        outer.has_phyrexian_mana_symbol = true;
        for branch in &mut branches {
            branch.has_phyrexian_mana_symbol = false;
        }
    }

    let shared_card_noun = outer.has_explicit_card_noun()
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
        );
    let mut selectors = Vec::new();
    for branch in &branches {
        if !branch_scoped_union_arm_is_characteristic_selector(branch) {
            return None;
        }
        if shared_card_noun
            && branch.supertypes.len() == 1
            && branch.card_types.is_empty()
            && branch.subtypes.is_empty()
        {
            selectors.push(branch.supertypes[0].to_string());
        } else {
            selectors.push(branch.description());
        }
    }

    if !outer.card_types.is_empty()
        || !outer.all_card_types.is_empty()
        || !outer.excluded_card_types.is_empty()
        || !outer.subtypes.is_empty()
        || !outer.all_subtypes.is_empty()
        || outer.type_or_subtype_union
        || !outer.excluded_subtypes.is_empty()
        || !outer.supertypes.is_empty()
        || !outer.excluded_supertypes.is_empty()
        || outer.required_colors.is_some()
        || !outer.excluded_colors.is_empty()
        || outer.colorless
        || outer.multicolored
        || outer.monocolored
    {
        return None;
    }

    let selector = if filter.has_conjunctive_set_surface() {
        describe_conjunctive_filter_list(selectors)
    } else {
        describe_filter_union_list(selectors, filter.union_connective(), true)
    };
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
    let mut description =
        replace_first_description_word(&outer.description(), placeholder, &replacement)?;
    // A branch-local source exclusion already supplies the determiner
    // ("another Pest"). Do not retain the outer placeholder's article when
    // that selector becomes the coordinated list head.
    for doubled in ["a another ", "an another ", "a other ", "an other "] {
        if description.starts_with(doubled) {
            let determiner = doubled
                .split_once(' ')
                .map(|(_, remainder)| remainder)
                .unwrap_or(doubled);
            description.replace_range(..doubled.len(), determiner);
            break;
        }
    }
    // Re-evaluate the article against the complete rendered phrase. Shared
    // adjectives can precede the replacement noun ("a tapped artifact",
    // "a red instant"), so looking only at the first character of the
    // replacement incorrectly produced "an tapped" and "an red".
    Some(correct_leading_indefinite_article(description))
}

fn branch_scoped_union_arm_is_characteristic_selector(filter: &ObjectFilter) -> bool {
    let single_characteristic =
        filter.card_types.len() + filter.subtypes.len() + filter.supertypes.len() == 1
            && filter.all_card_types.is_empty();
    // The leaf parser currently preserves an intersecting phrase such as
    // `artifact creature` in both its permissive card-type list and its exact
    // all-types list. Treat that redundant representation as the same typed
    // selector instead of falling back to per-branch rendering, which loses
    // shared outer modifiers such as `nontoken`.
    let intersecting_card_types = filter.subtypes.is_empty()
        && filter.all_card_types.len() >= 2
        && (filter.card_types.is_empty() || filter.card_types == filter.all_card_types);
    if !filter.any_of.is_empty()
        || (!single_characteristic && !intersecting_card_types)
        || !filter.all_subtypes.is_empty()
        || filter.type_or_subtype_union
        || filter.zone.is_some()
        || filter.controller.is_some()
        || filter.owner.is_some()
    {
        return false;
    }

    let mut remainder = filter.clone();
    if remainder.tagged_constraints.iter().any(|constraint| {
        constraint.relation != TaggedOpbjectRelation::IsTaggedObject
            || !matches!(constraint.tag.as_str(), "enchanted" | "equipped")
    }) {
        return false;
    }
    remainder.card_types.clear();
    remainder.all_card_types.clear();
    remainder.subtypes.clear();
    remainder.all_subtypes.clear();
    remainder.supertypes.clear();
    remainder.excluded_card_types.clear();
    remainder.excluded_subtypes.clear();
    remainder.excluded_supertypes.clear();
    remainder.excluded_colors = ColorSet::new();
    remainder.zone = None;
    remainder.controller = None;
    remainder.owner = None;
    remainder.power = None;
    remainder.toughness = None;
    remainder.mana_value = None;
    // State adjectives may be scoped to just one arm while zone/controller
    // qualifiers live on the outer union (for example, "untapped artifacts
    // and/or creatures you control"). They are still valid characteristic
    // selectors and remain present in each arm's rendered description.
    remainder.tapped = false;
    remainder.untapped = false;
    // "Other" is a branch-local source exclusion, not a separate domain
    // predicate. Keep it on the rendered selector (for example,
    // "Skeletons you control and other Zombies you control") while allowing
    // the outer battlefield/controller scope to qualify the whole union.
    remainder.other = false;
    remainder.tagged_constraints.clear();
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

fn describe_distinct_source_threshold(sources: &ObjectFilter, minimum: u32) -> String {
    let sources = if let [subtype] = sources.subtypes.as_slice() {
        let mut remainder = sources.clone();
        remainder.subtypes.clear();
        if remainder.card_types == [CardType::Creature] {
            remainder.card_types.clear();
        }
        if remainder.zone == Some(Zone::Battlefield) {
            remainder.zone = None;
        }
        if remainder == ObjectFilter::default() {
            pluralize_count_terminal_word(&subtype.to_string())
        } else {
            let source = sources.description();
            let source = source
                .strip_prefix("an ")
                .or_else(|| source.strip_prefix("a "))
                .unwrap_or(&source);
            pluralize_count_terminal_word(source)
        }
    } else {
        let source = sources.description();
        let source = source
            .strip_prefix("an ")
            .or_else(|| source.strip_prefix("a "))
            .unwrap_or(&source);
        pluralize_count_terminal_word(source)
    };
    let minimum = crate::cardinal_word(minimum).unwrap_or_else(|| minimum.to_string());
    format!("{minimum} or more {sources}")
}

fn describe_possessive_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Any => "a player's".to_string(),
        PlayerFilter::You => "your".to_string(),
        PlayerFilter::NotYou => "a non-you player's".to_string(),
        PlayerFilter::Opponent => "an opponent's".to_string(),
        PlayerFilter::Teammate => "a teammate's".to_string(),
        PlayerFilter::PlayerToYourLeft => "the player to your left's".to_string(),
        PlayerFilter::PlayerToYourRight => "the player to your right's".to_string(),
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
        PlayerFilter::AttackedBySourceThisTurn => {
            "a player this creature attacked this turn's".to_string()
        }
        PlayerFilter::WasDealtDamageBySourceThisGame { base } => format!(
            "{} this source has dealt damage to this game's",
            describe_player_filter(base)
        ),
        PlayerFilter::WasDealtCombatDamageBySourcesThisGame { base, sources } => format!(
            "{} dealt combat damage this game by {}'s",
            describe_player_filter(base),
            sources.description()
        ),
        PlayerFilter::LostLifeThisTurn { base } => {
            format!("{} who lost life this turn's", describe_player_filter(base))
        }
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. } => {
            format!("{}'s", describe_player_filter(filter))
        }
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
        PlayerFilter::ControlsMost { .. } => format!("{}'s", filter.description()),
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
        PlayerFilter::PlayerToYourLeft => "the player to your left".to_string(),
        PlayerFilter::PlayerToYourRight => "the player to your right".to_string(),
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
        PlayerFilter::AttackedBySourceThisTurn => {
            "player this creature attacked this turn".to_string()
        }
        PlayerFilter::WasDealtDamageBySourceThisGame { base } => format!(
            "{} this source has dealt damage to this game",
            describe_player_filter(base)
        ),
        PlayerFilter::WasDealtCombatDamageBySourcesThisGame { base, sources } => format!(
            "{} dealt combat damage this game by {}",
            describe_player_filter(base),
            sources.description()
        ),
        PlayerFilter::LostLifeThisTurn { base } => {
            format!("{} who lost life this turn", describe_player_filter(base))
        }
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn {
            base,
            sources,
            minimum,
        } => format!(
            "{} who was dealt combat damage by {} this turn",
            describe_player_filter(base),
            describe_distinct_source_threshold(sources, *minimum)
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
        PlayerFilter::ControlsMost { filter } => format!(
            "the player who controls the most {}",
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

fn describe_counters_put_on_this_turn_constraint(
    constraint: &CountersPutOnThisTurnConstraint,
) -> String {
    let actor = match &constraint.source_controller {
        PlayerFilter::You => "you've put".to_string(),
        PlayerFilter::Opponent => "an opponent has put".to_string(),
        PlayerFilter::Any => "a player has put".to_string(),
        PlayerFilter::IteratedPlayer | PlayerFilter::AliasedTarget(_) => {
            "that player has put".to_string()
        }
        player => format!("{} has put", describe_player_filter(player)),
    };
    let quantity = match constraint.minimum {
        1 => "one or more".to_string(),
        minimum => format!("{minimum} or more"),
    };
    let counter = constraint
        .counter_type
        .map(|counter_type| counter_type.description().into_owned())
        .unwrap_or_else(|| "counter".to_string());
    format!("that {actor} {quantity} {counter} counters on this turn")
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
        CounterConstraint::AtLeast {
            counter_type,
            count,
        } => {
            let count = crate::cardinal_word(count).unwrap_or_else(|| count.to_string());
            match counter_type {
                Some(counter_type) => {
                    format!("{count} or more {} counters", counter_type.description())
                }
                None => format!("{count} or more counters"),
            }
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
        " defending player controls",
        " defending player owns",
        " attacking player controls",
        " active player controls",
        " its controller controls",
        " target player controls",
        " target opponent controls",
        " a teammate controls",
        " your team controls",
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
                if hints.contains(&crate::ValueSurfaceHint::MasculineSourcePossessive)
                    && matches!(value.unhinted(), Value::SourcePower)
                {
                    return "his power".to_string();
                }
                if hints.contains(&crate::ValueSurfaceHint::FeminineSourcePossessive)
                    && matches!(value.unhinted(), Value::SourcePower)
                {
                    return "her power".to_string();
                }
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
            Value::GreatestCount(filter) => format!(
                "the greatest number of {}",
                describe_count_filter_subject(filter)
            ),
            Value::GreatestSharedCreatureTypeCount(filter) => format!(
                "the greatest number of {} that have a creature type in common",
                describe_count_filter_subject(filter)
            ),
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
            Value::ColorPairsAmong(filter) => {
                format!(
                    "the number of different color pairs among {}",
                    filter.description()
                )
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
            Value::PlayerCounters(player, counter_type) => {
                let holder = match player {
                    PlayerFilter::You => "you have".to_string(),
                    PlayerFilter::Opponent => "an opponent has".to_string(),
                    PlayerFilter::Any => "a player has".to_string(),
                    PlayerFilter::Target(_)
                    | PlayerFilter::AliasedTarget(_)
                    | PlayerFilter::Specific(_) => "that player has".to_string(),
                    other => format!("{} has", other.description()),
                };
                format!(
                    "the number of {} counters {holder}",
                    counter_type.description()
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
            Value::Devotion { player, color } => format!(
                "{} devotion to {}",
                describe_possessive_player_filter(player),
                color.name()
            ),
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
                reference,
            } => {
                let mut source = source_filter.description();
                if *include_source_noun {
                    source.push_str(" source");
                }
                format!(
                    "the amount of mana from {source} spent to cast {}",
                    reference.text()
                )
            }
            Value::EffectMetric {
                metric: EffectMetric::OtherNumber,
                ..
            } => "the other result".to_string(),
            Value::BasicLandTypesAmong(filter) => {
                let among = if filter.card_types == [crate::types::CardType::Land]
                    && filter.controller == Some(PlayerFilter::You)
                {
                    "lands you control".to_string()
                } else {
                    filter.description()
                };
                format!("the number of basic land types among {among}")
            }
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
                || matches!(
                    value.unhinted(),
                    Value::EffectMetric {
                        metric: EffectMetric::OtherNumber,
                        ..
                    }
                )
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
                || matches!(
                    value.unhinted(),
                    Value::EffectMetric {
                        metric: EffectMetric::OtherNumber,
                        ..
                    }
                )
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
    use crate::{
        CardType, ColorSet, CounterConstraint, CounterType, ManaSymbol, ObjectId, Subtype,
        Supertype, TagKey, Value, ValueSurfaceHint, Zone,
    };

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
        assert!(!PlayerFilter::target_player().mentions_iterated_player());
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

        let experience_counters = Value::PlayerCounters(PlayerFilter::You, CounterType::Experience);
        assert_eq!(
            describe_comparison(&Comparison::GreaterThanExpr(Box::new(experience_counters))),
            "greater than the number of experience counters you have"
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
    fn excluded_name_surface_preserves_oracle_spelling_without_changing_filter_equality() {
        let normalized = ObjectFilter::default().not_named("staff of eden vaults key");
        let mut surfaced = normalized.clone();
        surfaced.set_excluded_name_surface("Staff of Eden, Vault's Key");

        assert_eq!(normalized, surfaced);
        assert_eq!(
            surfaced.description(),
            "permanent not named Staff of Eden, Vault's Key"
        );
    }

    #[test]
    fn controlled_but_not_owned_scope_renders_in_oracle_order() {
        let filter = ObjectFilter::default()
            .you_control()
            .owned_by(PlayerFilter::NotYou);

        assert_eq!(
            filter.description(),
            "a permanent you control but don't own"
        );
    }

    #[test]
    fn mirrored_owner_or_controller_branches_share_the_object_noun() {
        let permanent = ObjectFilter::permanent();
        let union = ObjectFilter {
            any_of: vec![
                permanent.clone().owned_by(PlayerFilter::You),
                permanent.controlled_by(PlayerFilter::You),
            ],
            ..ObjectFilter::default()
        };

        assert_eq!(union.description(), "a permanent you own or control");

        let distinct = ObjectFilter {
            any_of: vec![
                ObjectFilter::permanent().owned_by(PlayerFilter::You),
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
            ],
            ..ObjectFilter::default()
        };
        assert_eq!(
            distinct.description(),
            "a permanent you own or a creature you control"
        );
    }

    #[test]
    fn shared_color_and_controller_qualify_a_conjunctive_spell_type_union() {
        let mut filter = ObjectFilter::spell().you_control();
        filter.colors = Some(ColorSet::RED);
        filter.any_of = vec![
            ObjectFilter::default().with_type(CardType::Instant),
            ObjectFilter::default().with_type(CardType::Sorcery),
        ];
        filter.set_conjunctive_set_surface(true);

        assert_eq!(
            filter.description(),
            "a red instant and sorcery spell you control"
        );
    }

    #[test]
    fn flat_subtype_set_uses_its_conjunctive_list_surface() {
        let mut filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            controller: Some(PlayerFilter::You),
            subtypes: vec![Subtype::Spider, Subtype::Boar, Subtype::Bat],
            other: true,
            ..ObjectFilter::default()
        };
        filter.set_conjunctive_set_surface(true);

        assert_eq!(
            filter.description(),
            "another Spider, Boar, and Bat you control"
        );
    }

    #[test]
    fn shared_outer_controller_qualifies_union_with_spell_fields_on_branches() {
        let spell_branch = |card_type| ObjectFilter {
            stack_kind: Some(StackObjectKind::Spell),
            card_types: vec![card_type],
            has_mana_cost: true,
            ..ObjectFilter::default()
        };
        let mut filter = ObjectFilter {
            zone: Some(Zone::Stack),
            controller: Some(PlayerFilter::You),
            any_of: vec![
                spell_branch(CardType::Instant),
                spell_branch(CardType::Sorcery),
            ],
            ..ObjectFilter::default()
        };
        filter.set_conjunctive_set_surface(true);

        assert_eq!(
            filter.description(),
            "an instant and sorcery spell you control"
        );
    }

    #[test]
    fn outer_controller_materializes_on_mixed_domain_union_branches() {
        let mut creature_spell = ObjectFilter::spell().with_type(CardType::Creature);
        creature_spell.has_mana_cost = true;
        let mut filter = ObjectFilter {
            controller: Some(PlayerFilter::You),
            any_of: vec![ObjectFilter::creature(), creature_spell],
            ..ObjectFilter::default()
        };
        filter.set_conjunctive_set_surface(true);

        assert_eq!(
            filter.description(),
            "a creature you control and a creature spell you control"
        );
    }

    #[test]
    fn outer_other_materializes_on_mixed_domain_union_branches() {
        let mut filter = ObjectFilter {
            controller: Some(PlayerFilter::You),
            other: true,
            any_of: vec![
                ObjectFilter::default()
                    .with_type(CardType::Artifact)
                    .with_type(CardType::Creature)
                    .with_all_type(CardType::Artifact)
                    .with_all_type(CardType::Creature)
                    .nontoken(),
                ObjectFilter::default().with_subtype(Subtype::Vehicle),
            ],
            ..ObjectFilter::default()
        };
        filter.set_conjunctive_set_surface(true);

        assert_eq!(
            filter.description(),
            "another nontoken artifact creature you control and another Vehicle you control"
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
    fn counter_threshold_constraint_renders_its_exact_cardinal_minimum() {
        let mut filter = ObjectFilter::default()
            .with_type(CardType::Creature)
            .you_control();
        filter.with_counter = Some(CounterConstraint::AtLeast {
            counter_type: Some(CounterType::PlusOnePlusOne),
            count: 12,
        });
        filter.set_counter_requirement_surface(false, true, true);

        assert_eq!(
            filter.description(),
            "a creature you control with twelve or more +1/+1 counters on them"
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
    fn branch_scoped_characteristic_union_keeps_comparison_on_its_arm() {
        let mut filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            any_of: vec![
                ObjectFilter::creature().with_power(Comparison::LessThanOrEqual(2)),
                ObjectFilter::default().with_subtype(Subtype::Wall),
            ],
            ..Default::default()
        };
        for branch in &mut filter.any_of {
            branch.zone = None;
        }
        filter.set_union_connective(ObjectFilterUnionConnective::AndOr);

        assert_eq!(
            filter.description(),
            "creature with power 2 or less and/or Wall"
        );
    }

    #[test]
    fn branch_scoped_characteristic_union_factors_conjunctive_controller_scope() {
        let mut filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            controller: Some(PlayerFilter::You),
            any_of: vec![
                ObjectFilter::default().with_subtype(Subtype::Plant),
                ObjectFilter::default().with_subtype(Subtype::Treefolk),
            ],
            ..Default::default()
        };
        filter.set_conjunctive_set_surface(true);

        assert_eq!(filter.description(), "a Plant and Treefolk you control");
    }

    #[test]
    fn branch_scoped_conjunctive_union_accepts_intersecting_card_type_selector() {
        let mut filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            controller: Some(PlayerFilter::You),
            any_of: vec![
                ObjectFilter::default()
                    .with_all_type(CardType::Artifact)
                    .with_all_type(CardType::Creature),
                ObjectFilter::default().with_subtype(Subtype::Hero),
            ],
            ..ObjectFilter::default()
        };
        filter.set_conjunctive_set_surface(true);

        assert_eq!(
            filter.description(),
            "an artifact creature and Hero you control"
        );
    }

    #[test]
    fn branch_scoped_characteristic_union_keeps_other_on_one_arm() {
        let mut filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            controller: Some(PlayerFilter::You),
            any_of: vec![
                ObjectFilter::default().with_subtype(Subtype::Skeleton),
                ObjectFilter::default()
                    .with_subtype(Subtype::Zombie)
                    .other(),
            ],
            ..Default::default()
        };
        filter.set_conjunctive_set_surface(true);

        assert_eq!(
            filter.description(),
            "a Skeleton and another Zombie you control"
        );
    }

    #[test]
    fn branch_scoped_characteristic_union_flattens_a_serial_list_container() {
        let leading = ObjectFilter {
            any_of: vec![
                ObjectFilter::default().with_subtype(Subtype::Pest).other(),
                ObjectFilter::default().with_subtype(Subtype::Bat),
                ObjectFilter::default().with_subtype(Subtype::Insect),
                ObjectFilter::default().with_subtype(Subtype::Snake),
            ],
            ..ObjectFilter::default()
        };
        let mut filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            controller: Some(PlayerFilter::You),
            any_of: vec![
                leading,
                ObjectFilter::default().with_subtype(Subtype::Spider),
            ],
            ..ObjectFilter::default()
        };
        filter.set_conjunctive_set_surface(true);

        assert_eq!(
            filter.description(),
            "another Pest, Bat, Insect, Snake, and Spider you control"
        );
    }

    #[test]
    fn branch_scoped_type_union_factors_shared_tap_state_and_controller_scope() {
        for (tapped, untapped, expected) in [
            (true, false, "a tapped artifact and/or creature you control"),
            (
                false,
                true,
                "an untapped artifact and/or creature you control",
            ),
        ] {
            let mut artifact = ObjectFilter::default().with_type(CardType::Artifact);
            artifact.tapped = tapped;
            artifact.untapped = untapped;
            let mut creature = ObjectFilter::default().with_type(CardType::Creature);
            creature.tapped = tapped;
            creature.untapped = untapped;
            let filter = ObjectFilter {
                zone: Some(Zone::Battlefield),
                controller: Some(PlayerFilter::You),
                any_of: vec![artifact, creature],
                ..Default::default()
            }
            .with_union_connective(ObjectFilterUnionConnective::AndOr);

            assert_eq!(filter.description(), expected);
        }
    }

    #[test]
    fn branch_scoped_type_union_keeps_arm_tap_state_and_outer_controller_scope() {
        let mut artifact = ObjectFilter::default().with_type(CardType::Artifact);
        artifact.untapped = true;
        let filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            controller: Some(PlayerFilter::You),
            any_of: vec![
                artifact,
                ObjectFilter::default().with_type(CardType::Creature),
            ],
            ..Default::default()
        }
        .with_union_connective(ObjectFilterUnionConnective::AndOr);

        assert_eq!(
            filter.description(),
            "an untapped artifact and/or creature you control"
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
    fn branch_scoped_supertype_or_subtype_union_uses_one_shared_card_noun() {
        let filter = ObjectFilter {
            zone: Some(Zone::Graveyard),
            owner: Some(PlayerFilter::You),
            any_of: vec![
                ObjectFilter {
                    supertypes: vec![Supertype::Legendary],
                    ..Default::default()
                },
                ObjectFilter {
                    subtypes: vec![Subtype::Rat],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert_eq!(
            filter.description(),
            "legendary or Rat card in your graveyard"
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
    fn relative_and_controller_qualified_unions_keep_their_authored_shape() {
        let mut relative = ObjectFilter::creature()
            .you_control()
            .with_subtype(Subtype::Fungus)
            .with_subtype(Subtype::Saproling)
            .with_union_connective(ObjectFilterUnionConnective::AndOr);
        relative.set_relative_characteristic_list_surface(true);
        assert_eq!(
            relative.description(),
            "a creature you control that's a Fungus and/or Saproling"
        );

        let mut relative_with_repeated_articles = ObjectFilter::creature().you_control().other();
        relative_with_repeated_articles.any_of = vec![
            ObjectFilter::default().token(),
            ObjectFilter::default().with_subtype(Subtype::Rabbit),
        ];
        relative_with_repeated_articles.set_relative_characteristic_list_surface(true);
        relative_with_repeated_articles.set_explicit_union_branch_articles(true);
        assert_eq!(
            relative_with_repeated_articles.description(),
            "another creature you control that's a token or a Rabbit"
        );

        let mut controller_qualified = ObjectFilter::creature()
            .you_control()
            .with_subtype(Subtype::Vehicle)
            .with_union_connective(ObjectFilterUnionConnective::AndOr);
        controller_qualified.type_or_subtype_union = true;
        assert_eq!(
            controller_qualified.description(),
            "a creature and/or Vehicle you control"
        );

        let mut subtype_first = controller_qualified;
        subtype_first.set_subtype_before_card_type_union_surface(true);
        assert_eq!(
            subtype_first.description(),
            "a Vehicle and/or creature you control"
        );

        let mut shared_spell_noun = ObjectFilter {
            zone: Some(Zone::Stack),
            card_types: vec![CardType::Creature],
            subtypes: vec![Subtype::Aura],
            type_or_subtype_union: true,
            ..Default::default()
        };
        shared_spell_noun.set_union_connective(ObjectFilterUnionConnective::Or);
        shared_spell_noun.set_terminal_noun_after_type_subtype_union_surface(true);
        assert_eq!(shared_spell_noun.description(), "creature or Aura spell");
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
    fn shared_creature_type_comparison_domains_each_keep_an_article() {
        let filter = ObjectFilter::spell()
            .with_type(CardType::Creature)
            .sharing_no_creature_types_with(ObjectFilter::creature().you_control())
            .sharing_no_creature_types_with(
                ObjectFilter::default()
                    .with_type(CardType::Creature)
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You),
            );

        assert_eq!(
            filter.description(),
            "creature spell that doesn't share a creature type with a creature you control or a creature card in your graveyard"
        );
    }

    #[test]
    fn explicit_land_type_noun_is_surface_only_for_subtyped_lands() {
        let semantic_filter = ObjectFilter::land()
            .with_subtype(Subtype::Urzas)
            .you_control();
        let mut surfaced_filter = semantic_filter.clone();
        surfaced_filter.set_explicit_card_type_noun(Some(CardType::Land));

        assert_eq!(semantic_filter, surfaced_filter);
        assert_eq!(semantic_filter.description(), "an Urza's you control");
        assert_eq!(surfaced_filter.description(), "an Urza's land you control");
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

    #[test]
    fn attack_destination_constraint_does_not_repeat_attacking_as_an_adjective() {
        let mut filter = ObjectFilter::creature()
            .attacking_player_or_planeswalker_controlled_by(PlayerFilter::You);
        filter.attacking = true;

        assert_eq!(
            filter.description(),
            "creature that's attacking you or a planeswalker you control"
        );
    }

    #[test]
    fn attack_destination_constraint_uses_the_prior_player_as_pronoun_antecedent() {
        let filter = ObjectFilter::creature()
            .attacking_player_or_planeswalker_controlled_by(PlayerFilter::IteratedPlayer);

        assert_eq!(
            filter.description(),
            "creature that's attacking that player or a planeswalker they control"
        );
    }

    #[test]
    fn opponent_attack_destination_uses_authored_group_surface() {
        let filter = ObjectFilter::creature()
            .attacking_player_or_planeswalker_controlled_by(PlayerFilter::Opponent);

        assert_eq!(
            filter.description(),
            "creature attacking your opponents and/or planeswalkers they control"
        );
    }

    #[test]
    fn characteristic_or_mana_capability_union_factors_the_shared_domain() {
        let filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Land],
            any_of: vec![
                ObjectFilter {
                    supertypes: vec![Supertype::Snow],
                    ..Default::default()
                },
                ObjectFilter {
                    could_produce_mana: vec![ManaSymbol::Colorless],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert!(filter.uses_non_pt_battlefield_characteristics());
        assert_eq!(
            filter.description(),
            "land that is snow or could produce {C}"
        );
    }

    #[test]
    fn entered_battlefield_history_surface_preserves_explicit_zone_only_when_authored() {
        let mut abbreviated = ObjectFilter::creature().in_zone(Zone::Battlefield);
        abbreviated.entered_battlefield_this_turn = true;
        assert_eq!(abbreviated.description(), "creature that entered this turn");

        let mut explicit = abbreviated.clone();
        explicit.set_entered_battlefield_explicit_surface(true);
        assert_eq!(
            explicit.description(),
            "creature that entered the battlefield this turn"
        );
        assert_eq!(abbreviated, explicit);
    }

    #[test]
    fn owned_zone_then_counter_order_is_equality_transparent() {
        let semantic = ObjectFilter::creature()
            .in_zone(Zone::Exile)
            .owned_by(PlayerFilter::You)
            .with_counter_type(CounterType::Named("memory".into()));
        let mut surfaced = semantic.clone();
        surfaced.set_owner_before_zone_surface(true);
        surfaced.set_counter_requirement_after_zone_surface(true);
        surfaced.set_for_each_leading_then_surface(true);

        assert_eq!(semantic, surfaced);
        assert_eq!(
            surfaced.description(),
            "a creature card you own in exile with a memory counter on it"
        );
    }
}
