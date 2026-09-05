//! The readings of one "where X is ..." value binding clause: the typed
//! bound values (source stats, life gained or lost this turn, known values,
//! aggregates, sums and offsets of counts, hand and party sizes, mana-symbol
//! aggregates, turn history, the counted filter, a value expression). Formerly
//! a first-match ladder in `etb_static_lines`; every reading runs; two different
//! readings of one input are an ambiguity error.

use super::*;
use crate::recognition::{ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct BindingClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) clause: &'a LexedClause<'a>,
    pub(super) words: &'a [&'a str],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl BindingClause<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// A reading's outcome.
    fn outcome(&self, read: Option<Value>) -> ParseOutcome<Value> {
        match read {
            Some(value) => ParseOutcome::matched(value, crate::util::span_from_tokens(self.tokens)),
            None => ParseOutcome::NoMatch,
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&BindingClause<'_>) -> bool,
    read: fn(&BindingClause<'_>) -> ParseOutcome<Value>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("where-x-binding-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("where-x-source-stat-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_source_stat_value(input)),
    },
    Reading {
        id: RuleId::new("players-who-control-more-than-you-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_players_who_control_more_than_you_value(input)),
    },
    Reading {
        id: RuleId::new("where-x-life-gained-this-turn-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_life_gained_this_turn_value(input)),
    },
    Reading {
        id: RuleId::new("where-x-life-lost-this-turn-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_life_lost_this_turn_value(input)),
    },
    Reading {
        id: RuleId::new("where-x-opponents-dealt-combat-damage-this-turn-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_where_x_opponents_dealt_combat_damage_this_turn_value(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("where-x-noncombat-damage-to-opponents-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_noncombat_damage_to_opponents_value(input)),
    },
    Reading {
        id: RuleId::new("where-x-known-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_known_value(input)),
    },
    Reading {
        id: RuleId::new("difference-value-expression"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_difference_value_expression(input)),
    },
    Reading {
        id: RuleId::new("where-x-is-aggregate-filter-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_is_aggregate_filter_value(input)),
    },
    Reading {
        id: RuleId::new("players-with-cards-in-hand-at-least"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_players_with_cards_in_hand_at_least(input)),
    },
    Reading {
        id: RuleId::new("devotion-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_devotion_value(input)),
    },
    Reading {
        id: RuleId::new("all-players-hand-count"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_all_players_hand_count(input)),
    },
    Reading {
        id: RuleId::new("same-name-as-triggering-spell-graveyard-count"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_same_name_as_triggering_spell_graveyard_count(input)),
    },
    Reading {
        id: RuleId::new("where-x-is-fixed-plus-number-of-filter-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_is_fixed_plus_number_of_filter_value(input)),
    },
    Reading {
        id: RuleId::new("where-x-is-sum-of-number-of-filter-values"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_is_sum_of_number_of_filter_values(input)),
    },
    Reading {
        id: RuleId::new("where-x-is-fixed-plus-reference-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_is_fixed_plus_reference_value(input)),
    },
    Reading {
        id: RuleId::new("where-x-is-number-of-filter-plus-or-minus-fixed-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_where_x_is_number_of_filter_plus_or_minus_fixed_value(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("tagged-mana-value-reference"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_tagged_mana_value_reference(input)),
    },
    Reading {
        id: RuleId::new("your-hand-count"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("players-with-cards-in-hand-at-least")
                // Readings ranked above this one that read the input read it.
                && !input.read_by("where-x-is-number-of-filter-plus-or-minus-fixed-value")
        },
        read: |input| input.outcome(read_your_hand_count(input)),
    },
    Reading {
        id: RuleId::new("your-party-size"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("where-x-is-fixed-plus-number-of-filter-value")
        },
        read: |input| input.outcome(read_your_party_size(input)),
    },
    Reading {
        id: RuleId::new("where-x-is-number-of-differently-named-filter-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_where_x_is_number_of_differently_named_filter_value(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("where-x-is-number-of-different-powers-filter-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_where_x_is_number_of_different_powers_filter_value(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("where-x-is-greatest-number-of-filter-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_is_greatest_number_of_filter_value(input)),
    },
    Reading {
        id: RuleId::new("counters-on-reference"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_counters_on_reference(input)),
    },
    Reading {
        id: RuleId::new("where-x-is-colored-mana-symbols-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_where_x_is_colored_mana_symbols_value(input)),
    },
    Reading {
        id: RuleId::new("attractions-visited-this-turn"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_attractions_visited_this_turn(input)),
    },
    Reading {
        id: RuleId::new("where-x-is-number-of-filter-value"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("counters-on-reference")
                && !input.read_by("same-name-as-triggering-spell-graveyard-count")
                && !input.read_by("your-hand-count")
        },
        read: |input| input.outcome(read_where_x_is_number_of_filter_value(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &BindingClause<'_>) -> ParseOutcome<RuleMatch<Value>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS {
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        match (reading.read)(input).within(reading.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            )),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<Value>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{REGISTRY}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_where_x_source_stat_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    if let Some(value) = parse_where_x_source_stat_value(tokens) {
        return Some(value);
    }
    None
}
fn read_players_who_control_more_than_you_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    if let Some(value) =
        crate::grammar::values::parse_players_who_control_more_than_you_value_lexed(tokens)
    {
        return Some(value);
    }
    None
}
fn read_where_x_life_gained_this_turn_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    if let Some(value) = parse_where_x_life_gained_this_turn_value(tokens) {
        return Some(value);
    }
    None
}
fn read_where_x_life_lost_this_turn_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    if let Some(value) = parse_where_x_life_lost_this_turn_value(tokens) {
        return Some(value);
    }
    None
}
fn read_where_x_opponents_dealt_combat_damage_this_turn_value(
    input: &BindingClause<'_>,
) -> Option<Value> {
    let tokens = input.tokens;
    if let Some(value) = parse_where_x_opponents_dealt_combat_damage_this_turn_value(tokens) {
        return Some(value);
    }
    None
}
fn read_where_x_noncombat_damage_to_opponents_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    if let Some(value) = parse_where_x_noncombat_damage_to_opponents_value(tokens) {
        return Some(value);
    }
    None
}
fn read_where_x_known_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    if let Some(value) = etb_grammar::parse_where_x_known_value_tokens(tokens) {
        return Some(match value {
            WhereXKnownValue::ThisAbilityResolvedThisTurnCount => {
                Value::ThisAbilityResolvedThisTurnCount
            }
            WhereXKnownValue::YourLifeTotal => Value::LifeTotal(PlayerFilter::You),
            WhereXKnownValue::HalfYourLifeTotalRoundedUp => {
                Value::HalfLifeTotalRoundedUp(PlayerFilter::You)
            }
            WhereXKnownValue::HalfYourLifeTotalRoundedDown => {
                Value::HalfLifeTotalRoundedDown(PlayerFilter::You)
            }
            WhereXKnownValue::YourSpeed => Value::Speed(PlayerFilter::You),
            WhereXKnownValue::EventDamageAmount => Value::EventValue(EventValueSpec::Amount),
            WhereXKnownValue::OpponentCount => Value::CountPlayers(PlayerFilter::Opponent),
            WhereXKnownValue::PlayersBeingAttacked => Value::PlayersBeingAttacked,
            WhereXKnownValue::TargetPlayerLifeTotal | WhereXKnownValue::ThatPlayerLifeTotal => {
                Value::LifeTotal(PlayerFilter::target_player())
            }
            WhereXKnownValue::TargetPlayersLifeTotalDifference => {
                Value::LifeTotalDifference(PlayerFilter::target_player())
            }
            WhereXKnownValue::ThatPlayerSpeed => Value::Speed(PlayerFilter::target_player()),
            WhereXKnownValue::DiscardedCardManaValue => Value::ManaValueOf(Box::new(
                ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::DiscardedCost.bind()),
            )),
            WhereXKnownValue::RevealedCardsTotalManaValue => Value::TotalManaValue(
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::PublicRevealed.bind()),
            ),
            WhereXKnownValue::DraftNotedHighestNumber { card_name_tokens } => {
                Value::DraftNotedHighestNumber {
                    card_name: parser_token_word_refs(card_name_tokens).join(" "),
                }
                .with_surface_hint(ValueSurfaceHint::WhereXIs)
            }
        });
    }
    None
}
fn read_difference_value_expression(input: &BindingClause<'_>) -> Option<Value> {
    let words = input.words;
    // A complete arithmetic value expression owns relationship-aware zone
    // scopes such as `7 minus the number of cards in that creature's
    // controller's hand`. Parse that exact typed difference before the broad
    // where-X object-count families can reinterpret `that creature` as a
    // characteristic of the cards being counted.
    if let Some(tail) = words.get(3..)
        && let Some((value, used)) = parse_value_expr_words(tail)
        && used == tail.len()
        && value.has_surface_hint(ValueSurfaceHint::Difference)
    {
        return Some(value);
    }
    None
}
fn read_where_x_is_aggregate_filter_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    if let Some(value) = parse_where_x_is_aggregate_filter_value(tokens) {
        return Some(value);
    }
    None
}
fn read_players_with_cards_in_hand_at_least(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    // A qualified participant count can contain both "players" and "cards in
    // hand", but it counts the players satisfying the hand-size predicate.
    // Recognize that typed shape before the broad all-players-hand heuristic
    // below can collapse it into a count of cards.
    if let Some(captured) = etb_grammar::parse_where_x_number_of_filter_tokens(tokens)
        && let Some((players, minimum)) =
            crate::grammar::shared_util::value_semantics::parse_players_with_cards_in_hand_at_least(
                captured.filter_tokens,
            )
    {
        return Some(scale_where_x_number_value(
            Value::CountPlayersWithCardsInHandAtLeast(players, minimum),
            captured.multiplier,
        ));
    }
    None
}
fn read_devotion_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    // where X is your devotion to black
    if etb_grammar::etb_tokens_have_devotion_value_marker(tokens)
        && let Ok(Some(value)) = parse_devotion_value_from_add_clause(tokens)
    {
        return Some(value);
    }
    None
}
fn read_all_players_hand_count(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    // where X is the total number of cards in all players' hands
    if etb_grammar::etb_tokens_have_all_players_hand_count_value(tokens) {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Hand);
        return Some(Value::Count(filter));
    }
    None
}
fn read_same_name_as_triggering_spell_graveyard_count(input: &BindingClause<'_>) -> Option<Value> {
    let clause = input.clause;
    if clause.after_words(3).is_some_and(|tail| {
        etb_grammar::parse_same_name_as_triggering_spell_graveyard_value_tokens(tail.tokens())
    }) {
        return Some(Value::Count(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .match_tagged(
                    crate::tag::CompilerReferenceTag::Triggering.bind(),
                    crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
                ),
        ));
    }
    None
}
fn read_where_x_is_fixed_plus_number_of_filter_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    // where X is N plus the number of <objects>
    if let Some(value) = parse_where_x_is_fixed_plus_number_of_filter_value(tokens) {
        return Some(value);
    }
    None
}
fn read_where_x_is_sum_of_number_of_filter_values(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    // where X is the number of <objects> plus the number of <other objects>
    if let Some(value) = parse_where_x_is_sum_of_number_of_filter_values(tokens) {
        return Some(value);
    }
    None
}
fn read_where_x_is_fixed_plus_reference_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    // where X is N plus the sacrificed creature's mana value / power / toughness
    if let Some(value) = parse_where_x_is_fixed_plus_reference_value(tokens) {
        return Some(value);
    }
    None
}
fn read_where_x_is_number_of_filter_plus_or_minus_fixed_value(
    input: &BindingClause<'_>,
) -> Option<Value> {
    let tokens = input.tokens;
    // where X is the number of <objects> plus/minus N
    if let Some(value) = parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(tokens) {
        return Some(value);
    }
    None
}
fn read_tagged_mana_value_reference(input: &BindingClause<'_>) -> Option<Value> {
    let clause = input.clause;
    if let Some(reference) = clause
        .after_words(3)
        .and_then(|tail| etb_grammar::parse_tagged_mana_value_reference_tokens(tail.tokens()))
    {
        let tag = match reference {
            EtbTaggedManaValueReference::ExiledCard | EtbTaggedManaValueReference::ThatCard => {
                crate::tag::CompilerReferenceTag::It
            }
            EtbTaggedManaValueReference::TriggeringSpell => {
                crate::tag::CompilerReferenceTag::Triggering
            }
        };
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(tag.bind()))));
    }
    None
}
fn read_your_hand_count(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    // where X is the number of cards in your hand
    if etb_grammar::etb_tokens_have_your_hand_count_value(tokens) {
        return Some(Value::CardsInHand(PlayerFilter::You));
    }
    None
}
fn read_your_party_size(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    // where X is the number of creatures in your party
    if etb_grammar::etb_tokens_have_your_party_size_value(tokens) {
        return Some(Value::PartySize(PlayerFilter::You));
    }
    None
}
fn read_where_x_is_number_of_differently_named_filter_value(
    input: &BindingClause<'_>,
) -> Option<Value> {
    let tokens = input.tokens;
    // where X is the number of differently named <objects>
    if let Some(value) = parse_where_x_is_number_of_differently_named_filter_value(tokens) {
        return Some(value);
    }
    None
}
fn read_where_x_is_number_of_different_powers_filter_value(
    input: &BindingClause<'_>,
) -> Option<Value> {
    let tokens = input.tokens;
    // where X is the number of different powers among <objects>
    if let Some(value) = parse_where_x_is_number_of_different_powers_filter_value(tokens) {
        return Some(value);
    }
    None
}
fn read_where_x_is_greatest_number_of_filter_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    // where X is the greatest number of <objects> <player> controls
    if let Some(value) = parse_where_x_is_greatest_number_of_filter_value(tokens) {
        return Some(value);
    }
    None
}
fn read_counters_on_reference(input: &BindingClause<'_>) -> Option<Value> {
    let clause = input.clause;
    // where X is the number of counters on that creature
    if let Some(tail) = clause.after_words(3) {
        let mut equal_prefixed = Vec::with_capacity(tail.tokens().len() + 2);
        equal_prefixed.push(OwnedLexToken::word(
            "equal".to_string(),
            TextSpan::synthetic(),
        ));
        equal_prefixed.push(OwnedLexToken::word("to".to_string(), TextSpan::synthetic()));
        equal_prefixed.extend(tail.tokens().iter().cloned());
        if let Some(value) = parse_equal_to_number_of_counters_on_reference_value(&equal_prefixed) {
            return Some(value);
        }
    }
    None
}
fn read_where_x_is_colored_mana_symbols_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    // Parse mana-symbol aggregates before the generic "number of <objects>" form.
    // Otherwise the leading color adjective can be mistaken for an object filter.
    if let Some(value) = parse_where_x_is_colored_mana_symbols_value(tokens) {
        return Some(value);
    }
    None
}
fn read_attractions_visited_this_turn(input: &BindingClause<'_>) -> Option<Value> {
    let words = input.words;
    // Preserve turn-history quantities before the broad number-of-object
    // fallback. For example, "Attractions you've visited this turn" is not a
    // battlefield Attraction filter; Attractions that have left still count.
    if let Some(tail) = words.get(3..)
        && let Some((value, used)) = parse_value_expr_words(tail)
        && used == tail.len()
        && matches!(value.unhinted(), Value::AttractionsVisitedThisTurn(_))
    {
        return Some(value);
    }
    None
}
fn read_where_x_is_number_of_filter_value(input: &BindingClause<'_>) -> Option<Value> {
    let tokens = input.tokens;
    // where X is the number of <objects>
    if let Some(value) = parse_where_x_is_number_of_filter_value(tokens) {
        return Some(value);
    }
    None
}
pub(super) fn read_value_expression(input: &BindingClause<'_>) -> Option<Value> {
    let words = input.words;
    if let Some(tail) = words.get(3..)
        && let Some((value, used)) = parse_value_expr_words(tail)
        && used == tail.len()
    {
        return Some(value);
    }
    None
}
