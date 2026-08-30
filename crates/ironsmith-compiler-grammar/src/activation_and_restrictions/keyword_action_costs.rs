use super::*;
use crate::grammar::keyword_action_costs::{
    KeywordAbilityHead, KeywordCumulativeUpkeepCostSurface, KeywordDamageSubjectKind,
    KeywordDynamicManaTail, KeywordDynamicPaymentShape, KeywordGraveyardBottomPaymentScope,
    SpecialAbilityPhraseKind, parse_cumulative_upkeep_cost_surface_tokens,
    parse_dynamic_soulshift_words, parse_keyword_ability_surface_tokens,
    parse_keyword_cost_action_surface_tokens, parse_keyword_damage_subject_split_tokens,
    parse_keyword_dynamic_mana_tail_tokens, parse_keyword_dynamic_payment_tokens,
    parse_keyword_payment_lead_tokens, parse_keyword_untap_restriction_words,
    parse_normalized_keyword_words_tokens, parse_payment_alternative_split_tokens,
    parse_single_graveyard_bottom_payment_tokens, parse_special_ability_phrase_words,
};
use crate::grammar::leaf::{LeafManaCostPrefix, parse_leaf_mana_cost_prefix_tokens};
use crate::grammar::shared_util::value_semantics::{
    parse_equal_to_aggregate_filter_value, parse_equal_to_number_of_filter_value,
};
use crate::util::parse_value;

const SIMPLE_HEAD_KEYWORD_ACTIONS: &[(&str, KeywordAction)] = &[
    ("evolve", KeywordAction::Evolve),
    ("mentor", KeywordAction::Mentor),
    ("training", KeywordAction::Training),
    ("soulbond", KeywordAction::Soulbond),
    ("sunburst", KeywordAction::Sunburst),
    ("cascade", KeywordAction::Cascade),
    ("demonstrate", KeywordAction::Demonstrate),
];

#[derive(Debug, Clone, Copy)]
enum KeywordAmountKind {
    Afterlife,
    Backup,
    Fabricate,
    Firebending,
    Fading,
    Graft,
    Modular,
    Renown,
    Soulshift,
    Vanishing,
    Ward,
}

const NUMERIC_KEYWORD_ACTIONS: &[(&str, KeywordAmountKind)] = &[
    ("afterlife", KeywordAmountKind::Afterlife),
    ("backup", KeywordAmountKind::Backup),
    ("fabricate", KeywordAmountKind::Fabricate),
    ("firebending", KeywordAmountKind::Firebending),
    ("fading", KeywordAmountKind::Fading),
    ("graft", KeywordAmountKind::Graft),
    ("modular", KeywordAmountKind::Modular),
    ("renown", KeywordAmountKind::Renown),
    ("soulshift", KeywordAmountKind::Soulshift),
    ("vanishing", KeywordAmountKind::Vanishing),
    ("ward", KeywordAmountKind::Ward),
];

const MARKER_KEYWORD_FALLBACK_HEADS: &[&str] = &[
    "fabricate",
    "foretell",
    "bestow",
    "dash",
    "overload",
    "cleave",
    "soulshift",
    "adapt",
    "bolster",
    "disturb",
    "embalm",
    "emerge",
    "echo",
    "modular",
    "ninjutsu",
    "outlast",
    "suspend",
    "vanishing",
    "offering",
    "specialize",
    "spectacle",
    "graft",
    "backup",
    "fading",
    "fuse",
    "plot",
    "disguise",
    "tribute",
    "buyback",
    "flashback",
];
const MARKER_KEYWORD_IDS: &[&str] = &[
    "banding",
    "fabricate",
    "foretell",
    "bestow",
    "dash",
    "overload",
    "cleave",
    "soulshift",
    "adapt",
    "bolster",
    "disturb",
    "embalm",
    "emerge",
    "echo",
    "modular",
    "ninjutsu",
    "outlast",
    "scavenge",
    "suspend",
    "vanishing",
    "offering",
    "soulbond",
    "unearth",
    "specialize",
    "squad",
    "spectacle",
    "graft",
    "backup",
    "saddle",
    "fading",
    "fuse",
    "plot",
    "disguise",
    "tribute",
    "buyback",
    "flashback",
    "rebound",
];
const AMOUNT_MARKER_KEYWORDS: &[&str] = &[
    "soulshift",
    "adapt",
    "bolster",
    "modular",
    "vanishing",
    "backup",
    "saddle",
    "fading",
    "graft",
    "tribute",
];
const COST_MARKER_KEYWORDS: &[&str] = &[
    "bestow",
    "dash",
    "disturb",
    "embalm",
    "emerge",
    "ninjutsu",
    "outlast",
    "scavenge",
    "unearth",
    "specialize",
    "spectacle",
    "plot",
    "disguise",
    "flashback",
    "foretell",
    "overload",
    "cleave",
    "recover",
];
const ECHO_MARKER_KEYWORD: &str = "echo";
const BUYBACK_MARKER_KEYWORD: &str = "buyback";
const SUSPEND_MARKER_KEYWORD: &str = "suspend";
const REBOUND_MARKER_KEYWORD: &str = "rebound";
const SQUAD_MARKER_KEYWORD: &str = "squad";
const SINGLE_WORD_KEYWORD_ACTIONS: &[(&str, KeywordAction)] = &[
    ("flying", KeywordAction::Flying),
    ("menace", KeywordAction::Menace),
    ("banding", KeywordAction::Banding),
    ("hexproof", KeywordAction::Hexproof),
    ("haste", KeywordAction::Haste),
    ("improvise", KeywordAction::Improvise),
    ("convoke", KeywordAction::Convoke),
    ("delve", KeywordAction::Delve),
    ("deathtouch", KeywordAction::Deathtouch),
    ("lifelink", KeywordAction::Lifelink),
    ("vigilance", KeywordAction::Vigilance),
    ("trample", KeywordAction::Trample),
    ("reach", KeywordAction::Reach),
    ("defender", KeywordAction::Defender),
    ("decayed", KeywordAction::Decayed),
    ("flash", KeywordAction::Flash),
    ("phasing", KeywordAction::Phasing),
    ("indestructible", KeywordAction::Indestructible),
    ("shroud", KeywordAction::Shroud),
    ("assist", KeywordAction::Assist),
    ("backup", KeywordAction::Marker("backup")),
    ("cipher", KeywordAction::Cipher),
    ("devoid", KeywordAction::Devoid),
    ("dethrone", KeywordAction::Dethrone),
    ("enlist", KeywordAction::Enlist),
    ("evolve", KeywordAction::Evolve),
    ("extort", KeywordAction::Extort),
    ("haunt", KeywordAction::Haunt),
    ("ingest", KeywordAction::Ingest),
    ("mentor", KeywordAction::Mentor),
    ("melee", KeywordAction::Melee),
    ("training", KeywordAction::Training),
    ("myriad", KeywordAction::Myriad),
    ("partner", KeywordAction::Partner),
    ("provoke", KeywordAction::Provoke),
    ("ravenous", KeywordAction::Ravenous),
    ("riot", KeywordAction::Riot),
    ("skulk", KeywordAction::Skulk),
    ("sunburst", KeywordAction::Sunburst),
    ("undaunted", KeywordAction::Undaunted),
    ("unleash", KeywordAction::Unleash),
    ("wither", KeywordAction::Wither),
    ("infect", KeywordAction::Infect),
    ("undying", KeywordAction::Undying),
    ("persist", KeywordAction::Persist),
    ("prowess", KeywordAction::Prowess),
    ("exalted", KeywordAction::Exalted),
    ("cascade", KeywordAction::Cascade),
    ("storm", KeywordAction::Storm),
    ("gravestorm", KeywordAction::Gravestorm),
    ("demonstrate", KeywordAction::Demonstrate),
    ("rebound", KeywordAction::Rebound),
    ("ascend", KeywordAction::Ascend),
    ("fuse", KeywordAction::Fuse),
    ("compleated", KeywordAction::StaticMarker("compleated")),
    ("daybound", KeywordAction::Daybound),
    ("nightbound", KeywordAction::Nightbound),
    (
        "islandwalk",
        KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::Subtype {
            subtype: Subtype::Island,
            snow: false,
        }),
    ),
    (
        "swampwalk",
        KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::Subtype {
            subtype: Subtype::Swamp,
            snow: false,
        }),
    ),
    (
        "mountainwalk",
        KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::Subtype {
            subtype: Subtype::Mountain,
            snow: false,
        }),
    ),
    (
        "forestwalk",
        KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::Subtype {
            subtype: Subtype::Forest,
            snow: false,
        }),
    ),
    (
        "plainswalk",
        KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::Subtype {
            subtype: Subtype::Plains,
            snow: false,
        }),
    ),
    ("fear", KeywordAction::Fear),
    ("intimidate", KeywordAction::Intimidate),
    ("shadow", KeywordAction::Shadow),
    ("horsemanship", KeywordAction::Horsemanship),
    ("flanking", KeywordAction::Flanking),
    ("changeling", KeywordAction::Changeling),
];

fn keyword_head_is(head: &str, keyword: &str) -> bool {
    head == keyword
}

fn simple_keyword_action_for_head(head: &str) -> Option<KeywordAction> {
    SIMPLE_HEAD_KEYWORD_ACTIONS
        .iter()
        .find_map(|(keyword, action)| keyword_head_is(head, keyword).then(|| action.clone()))
}

fn numeric_keyword_action(head: &str, amount: &str) -> Option<KeywordAction> {
    let value = parse_named_number(amount)?;
    NUMERIC_KEYWORD_ACTIONS
        .iter()
        .find_map(|(keyword, kind)| keyword_head_is(head, keyword).then_some(*kind))
        .map(|kind| match kind {
            KeywordAmountKind::Afterlife => KeywordAction::Afterlife(value),
            KeywordAmountKind::Backup => KeywordAction::Backup(value),
            KeywordAmountKind::Fabricate => KeywordAction::Fabricate(value),
            KeywordAmountKind::Firebending => KeywordAction::Firebending(value),
            KeywordAmountKind::Fading => KeywordAction::Fading(value),
            KeywordAmountKind::Graft => KeywordAction::Graft(value),
            KeywordAmountKind::Modular => KeywordAction::Modular(value),
            KeywordAmountKind::Renown => KeywordAction::Renown(value),
            KeywordAmountKind::Soulshift => KeywordAction::Soulshift(value),
            KeywordAmountKind::Vanishing => KeywordAction::Vanishing(value),
            KeywordAmountKind::Ward => KeywordAction::Ward(value),
        })
}

fn is_marker_keyword_fallback_head(head: &str) -> bool {
    MARKER_KEYWORD_FALLBACK_HEADS
        .iter()
        .any(|keyword| keyword_head_is(head, keyword))
}

fn marker_keyword_set_contains(set: &[&str], keyword: &str) -> bool {
    set.iter()
        .any(|candidate| keyword_head_is(keyword, candidate))
}

pub fn target_ast_to_object_filter(target: TargetAst) -> Option<ObjectFilter> {
    match target {
        TargetAst::Source(_) => Some(ObjectFilter::source()),
        TargetAst::Object(filter, _, _) => Some(filter),
        TargetAst::Spell(_) => Some(ObjectFilter::spell()),
        TargetAst::Tagged(tag, _) => Some(ObjectFilter::tagged(tag)),
        TargetAst::AnyOtherTarget(_) => {
            Some(ObjectFilter::default().not_tagged(crate::tag::CompilerReferenceTag::It.key()))
        }
        TargetAst::WithCount(inner, _) => target_ast_to_object_filter(*inner),
        _ => None,
    }
}

pub fn is_supported_untap_restriction_tail(words: &[&str]) -> bool {
    parse_keyword_untap_restriction_words(words).is_some()
}

pub fn normalize_cant_words(tokens: &[OwnedLexToken]) -> Vec<String> {
    parse_normalized_keyword_words_tokens(tokens).words
}

pub fn keyword_title(keyword: &str) -> String {
    let mut out = String::new();
    let mut saw_word = false;
    let mut at_word_start = true;
    for ch in keyword.trim().chars() {
        if ch.is_whitespace() {
            if saw_word && !out.ends_with(' ') {
                out.push(' ');
            }
            at_word_start = true;
            continue;
        }
        if !saw_word {
            out.extend(ch.to_uppercase());
            saw_word = true;
        } else {
            out.push(ch);
        }
        at_word_start = false;
    }
    if at_word_start {
        out.pop();
    }
    out
}

fn keyword_mana_cost_prefix(tokens: &[OwnedLexToken], start: usize) -> Option<LeafManaCostPrefix> {
    let tail = strip_leading_keyword_cost_separator(tokens.get(start..)?);
    parse_leaf_mana_cost_prefix_tokens(tail)
}

fn cumulative_upkeep_text(cost_tokens: &[OwnedLexToken]) -> String {
    match parse_cumulative_upkeep_cost_surface_tokens(cost_tokens) {
        KeywordCumulativeUpkeepCostSurface::Empty => return "Cumulative upkeep".to_string(),
        KeywordCumulativeUpkeepCostSurface::AddMana(cost) => {
            return format!("Cumulative upkeep—Add {}", cost.to_oracle());
        }
        KeywordCumulativeUpkeepCostSurface::Mana(cost) => {
            return format!("Cumulative upkeep {}", cost.to_oracle());
        }
        KeywordCumulativeUpkeepCostSurface::ManaOrMana { left, right } => {
            return format!(
                "Cumulative upkeep {} or {}",
                left.to_oracle(),
                right.to_oracle()
            );
        }
        KeywordCumulativeUpkeepCostSurface::Text => {}
    }

    let mut tail_text = ActivationRestrictionCompatWords::new(cost_tokens).join(" ");
    if let Some(first) = tail_text.chars().next() {
        let upper = first.to_ascii_uppercase().to_string();
        let rest = &tail_text[first.len_utf8()..];
        tail_text = format!("{upper}{rest}");
    }
    format!("Cumulative upkeep—{tail_text}")
}

fn strip_leading_keyword_cost_separator(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    while start < tokens.len() && matches!(tokens[start].kind, TokenKind::Dash | TokenKind::EmDash)
    {
        start += 1;
    }
    &tokens[start..]
}

fn echo_text(
    total_cost: &ironsmith_core::TotalCost<crate::model::CompilerCost>,
    cost_tokens: &[OwnedLexToken],
) -> String {
    if let Some(cost) = total_cost.mana_cost()
        && !total_cost.has_non_mana_costs()
    {
        return format!("Echo {}", cost.to_oracle());
    }

    let payload = cost_tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>()
        .join(" ");
    if payload.is_empty() {
        return "Echo".to_string();
    }

    let mut chars = payload.chars();
    let first = chars.next().expect("payload is not empty");
    let mut normalized = String::new();
    normalized.push(first.to_ascii_uppercase());
    normalized.push_str(chars.as_str());
    format!("Echo—{normalized}")
}

fn parse_payment_clause_as_effects(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let trimmed = trim_edge_punctuation(&trim_commas(tokens));
    if trimmed.is_empty() {
        return Ok(None);
    }

    if let Some(or_idx) = find_payment_alternative_or(&trimmed) {
        let left = parse_payment_clause_as_effects(&trimmed[..or_idx])?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported payment cost before 'or' (clause: '{}')",
                words(&trimmed[..or_idx]).join(" ")
            ))
        })?;
        let right = parse_payment_clause_as_effects(&trimmed[or_idx + 1..])?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported payment cost after 'or' (clause: '{}')",
                words(&trimmed[or_idx + 1..]).join(" ")
            ))
        })?;
        return Ok(Some(vec![EffectAst::UnlessAction {
            effects: left,
            alternative: right,
            player: PlayerAst::You,
        }]));
    }

    let ast = match parse_effect_sentences_lexed(&trimmed) {
        Ok(ast) => ast,
        Err(_) => return Ok(None),
    };
    Ok((!ast.is_empty()).then_some(ast))
}

pub fn find_payment_alternative_or(tokens: &[OwnedLexToken]) -> Option<usize> {
    parse_payment_alternative_split_tokens(tokens).map(|split| split.delimiter)
}

pub fn parse_single_graveyard_bottom_library_compiler_payment(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::TotalCost<crate::model::CompilerCost>> {
    let payment = parse_single_graveyard_bottom_payment_tokens(tokens)?;
    let filter = match payment.scope {
        KeywordGraveyardBottomPaymentScope::SingleOwner => ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .single_graveyard(),
        KeywordGraveyardBottomPaymentScope::Yours => ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You),
    };
    Some(ironsmith_core::TotalCost::from_costs(vec![
        crate::model::CompilerCost::MoveChosenToLibraryBottom {
            count: payment.count,
            filter,
        },
    ]))
}

/// Parse `<payment> and <payment>` or `<payment>, <payment>` as one payment of
/// both components.
///
/// A compound cost may conjoin a mana payment with an action ("pay {1} and
/// return a basic land you control to its owner's hand", "Ward—{2}, Pay 2
/// life"). Cost components are already paid together, so the halves become
/// siblings of one total cost. A split is taken only when both halves are
/// themselves complete payments, which keeps commas inside one payment's own
/// filter out of the separator role.
fn parse_conjoined_payment_clause_as_total_cost(
    tokens: &[OwnedLexToken],
) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    for (index, token) in tokens.iter().enumerate() {
        if !token.is_word("and") && !token.is_comma() {
            continue;
        }
        let left_tokens = trim_edge_punctuation(&trim_commas(&tokens[..index]));
        let right_tokens = trim_edge_punctuation(&trim_commas(&tokens[index + 1..]));
        if left_tokens.is_empty() || right_tokens.is_empty() {
            continue;
        }
        let (Ok(Some(left)), Ok(Some(right))) = (
            parse_payment_clause_as_total_cost(&left_tokens),
            parse_payment_clause_as_total_cost(&right_tokens),
        ) else {
            continue;
        };
        let (Some(left), Some(right)) = (left.as_all(), right.as_all()) else {
            continue;
        };
        let mut components = left.to_vec();
        components.extend(right.iter().cloned());
        return Ok(Some(ironsmith_core::TotalCost::from_costs(components)));
    }
    Ok(None)
}

pub fn parse_payment_clause_as_total_cost(
    tokens: &[OwnedLexToken],
) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    let trimmed = trim_edge_punctuation(&trim_commas(tokens));
    if trimmed.is_empty() {
        return Ok(None);
    }

    if let Some(or_idx) = find_payment_alternative_or(&trimmed) {
        let left = parse_payment_clause_as_total_cost(&trimmed[..or_idx])?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported payment cost before 'or' (clause: '{}')",
                words(&trimmed[..or_idx]).join(" ")
            ))
        })?;
        let right =
            parse_payment_clause_as_total_cost(&trimmed[or_idx + 1..])?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported payment cost after 'or' (clause: '{}')",
                    words(&trimmed[or_idx + 1..]).join(" ")
                ))
            })?;
        return Ok(Some(ironsmith_core::TotalCost::one_of(vec![left, right])));
    }

    match parse_dynamic_payment_clause_as_total_cost(&trimmed)? {
        DynamicPaymentParse::Parsed(dynamic_cost) => return Ok(Some(dynamic_cost)),
        DynamicPaymentParse::Rejected => {
            // A dynamic-payment head can appear inside a conjoined payment
            // ("pay {1} and return a basic land ..."), where the rejected
            // reading covers only the first half.
            return parse_conjoined_payment_clause_as_total_cost(&trimmed);
        }
        DynamicPaymentParse::NotRecognized => {}
    }

    if let Some(effect_cost) = parse_single_graveyard_bottom_library_compiler_payment(&trimmed) {
        return Ok(Some(effect_cost));
    }

    if let Ok(total_cost) = parse_activation_cost(&trimmed)
        && !total_cost.is_free()
    {
        return Ok(Some(total_cost));
    }

    if let Some(total_cost) = parse_conjoined_payment_clause_as_total_cost(&trimmed)? {
        return Ok(Some(total_cost));
    }

    let Some(effects) = parse_payment_clause_as_effects(&trimmed)? else {
        return Ok(None);
    };
    Ok(Some(ironsmith_core::TotalCost::from_costs(
        effects
            .into_iter()
            .map(|effect| crate::model::CompilerCost::ValidatedEffect(Box::new(effect)))
            .collect(),
    )))
}

enum DynamicPaymentParse {
    NotRecognized,
    Rejected,
    Parsed(ironsmith_core::TotalCost<crate::model::CompilerCost>),
}

fn parse_dynamic_payment_clause_as_total_cost(
    tokens: &[OwnedLexToken],
) -> Result<DynamicPaymentParse, CardTextError> {
    let lead = parse_keyword_payment_lead_tokens(tokens);
    let tokens = &tokens[lead.payload_first..];
    let tokens = trim_edge_punctuation(&trim_commas(tokens));
    if tokens.is_empty() {
        return Ok(DynamicPaymentParse::NotRecognized);
    }
    let payment_words = crate::lexer::parser_token_word_refs(&tokens);
    if crate::word_primitives::parse_choice_sequence_complete(
        &payment_words,
        &[&["discard"], &["x"], &["card", "cards"]],
    ) {
        return Ok(DynamicPaymentParse::Parsed(
            ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::ValidatedEffect(
                Box::new(EffectAst::subject_verb_discard(
                    PlayerAst::You,
                    Value::X,
                    false,
                    false,
                    None,
                    None,
                )),
            )),
        ));
    }
    let Some(shape) = parse_keyword_dynamic_payment_tokens(&tokens) else {
        return Ok(DynamicPaymentParse::NotRecognized);
    };
    match shape {
        KeywordDynamicPaymentShape::Energy { value } => {
            let value_tokens = trim_edge_punctuation(&trim_commas(&tokens[value]));
            let Some((value, used)) = parse_value(&value_tokens) else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported dynamic energy payment amount (clause: '{}')",
                    words(&tokens).join(" ")
                )));
            };
            if !trim_edge_punctuation(&trim_commas(&value_tokens[used..])).is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing dynamic energy payment text (clause: '{}')",
                    words(&tokens).join(" ")
                )));
            }
            Ok(DynamicPaymentParse::Parsed(
                ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::ValidatedEffect(
                    Box::new(EffectAst::subject_verb_pay_energy(PlayerAst::You, value)),
                )),
            ))
        }
        KeywordDynamicPaymentShape::ManaAmountEqual => {
            let Some(value) = parse_equal_to_aggregate_filter_value(&tokens)
                .or_else(|| parse_equal_to_number_of_filter_value(&tokens))
            else {
                return Ok(DynamicPaymentParse::Rejected);
            };
            Ok(DynamicPaymentParse::Parsed(
                ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::DynamicMana(
                    ironsmith_core::DynamicManaCost::new(
                        ManaCost::new(),
                        None,
                        Some(value),
                        None,
                        ironsmith_core::DynamicManaDisplayHint::ManaEqualTo,
                    ),
                )),
            ))
        }
        KeywordDynamicPaymentShape::Mana {
            cost: mana_cost,
            trailing_first,
        } => {
            let trailing = trim_edge_punctuation(&trim_commas(&tokens[trailing_first..]));
            if trailing.is_empty() {
                return Ok(DynamicPaymentParse::Rejected);
            }
            let tail = parse_keyword_dynamic_mana_tail_tokens(&trailing);
            if let KeywordDynamicManaTail::Life { value } = tail {
                let Some(value) = value else {
                    return Ok(DynamicPaymentParse::Rejected);
                };
                let life_tokens = &trailing[value];
                if let Some((amount, used)) = parse_value(life_tokens)
                    && used == life_tokens.len()
                {
                    return Ok(DynamicPaymentParse::Parsed(
                        ironsmith_core::TotalCost::from_costs(vec![
                            crate::model::CompilerCost::Mana(mana_cost),
                            crate::model::CompilerCost::Life(amount),
                        ]),
                    ));
                }
                return Ok(DynamicPaymentParse::Rejected);
            }

            let mut x_value = None;
            let mut additional_generic = None;
            let mut multiplier = None;
            match tail {
                KeywordDynamicManaTail::WhereX {
                    same_name_in_graveyard,
                } => {
                    if !mana_cost.has_x() {
                        return Err(CardTextError::ParseError(format!(
                            "where-X payment clause has no X mana symbol (clause: '{}')",
                            words(&tokens).join(" ")
                        )));
                    }
                    x_value = parse_value_binding_clause(&trailing).or_else(|| {
                        same_name_in_graveyard.then(|| {
                            Value::Count(
                                ObjectFilter::default()
                                    .in_zone(Zone::Graveyard)
                                    .match_tagged(
                                        crate::tag::CompilerReferenceTag::Triggering.key(),
                                        crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
                                    ),
                            )
                        })
                    });
                    if x_value.is_none() {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported where-X payment clause (clause: '{}')",
                            words(&tokens).join(" ")
                        )));
                    }
                }
                KeywordDynamicManaTail::ForEach => {
                    multiplier = parse_dynamic_cost_modifier_value(&trailing)?;
                }
                KeywordDynamicManaTail::Modifier => {
                    let Some(value) = parse_dynamic_cost_modifier_value(&trailing)? else {
                        return Ok(DynamicPaymentParse::Rejected);
                    };
                    additional_generic = Some(value);
                }
                KeywordDynamicManaTail::Life { .. } => unreachable!("handled above"),
            }

            Ok(DynamicPaymentParse::Parsed(
                ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::DynamicMana(
                    ironsmith_core::DynamicManaCost::new(
                        mana_cost,
                        x_value,
                        additional_generic,
                        multiplier,
                        ironsmith_core::DynamicManaDisplayHint::Default,
                    ),
                )),
            ))
        }
    }
}

pub fn marker_keyword_id(keyword: &str) -> Option<&'static str> {
    MARKER_KEYWORD_IDS
        .iter()
        .copied()
        .find(|candidate| keyword_head_is(keyword, candidate))
}

pub fn marker_keyword_display(tokens: &[OwnedLexToken]) -> Option<String> {
    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let words = word_view.to_word_refs();
    let keyword = words.first().copied()?;
    let title = keyword_title(keyword);

    if marker_keyword_set_contains(AMOUNT_MARKER_KEYWORDS, keyword) {
        let amount = words.get(1).and_then(|word| parse_named_number(word))?;
        return Some(format!("{title} {amount}"));
    }
    if marker_keyword_set_contains(COST_MARKER_KEYWORDS, keyword) {
        let cost = keyword_mana_cost_prefix(tokens, 1)?.cost;
        return Some(format!("{title} {}", cost.to_oracle()));
    }
    if keyword_head_is(keyword, ECHO_MARKER_KEYWORD) {
        return echo_marker_keyword_display(tokens, &words);
    }
    if keyword_head_is(keyword, BUYBACK_MARKER_KEYWORD) {
        return buyback_marker_keyword_display(tokens, &words);
    }
    if keyword_head_is(keyword, SUSPEND_MARKER_KEYWORD) {
        let time = words.get(1).and_then(|word| parse_named_number(word))?;
        let cost = keyword_mana_cost_prefix(tokens, 2)?.cost;
        return Some(format!("Suspend {time}—{}", cost.to_oracle()));
    }
    if keyword_head_is(keyword, REBOUND_MARKER_KEYWORD) {
        return Some("Rebound".to_string());
    }
    if keyword_head_is(keyword, SQUAD_MARKER_KEYWORD) {
        let cost = keyword_mana_cost_prefix(tokens, 1)?.cost;
        return Some(format!("Squad {}", cost.to_oracle()));
    }
    None
}

fn echo_marker_keyword_display(tokens: &[OwnedLexToken], words: &[&str]) -> Option<String> {
    if let Some(prefix) = keyword_mana_cost_prefix(tokens, 1) {
        return Some(format!("Echo {}", prefix.cost.to_oracle()));
    }
    if words.len() > 1 {
        let payload = words[1..].join(" ");
        let mut chars = payload.chars();
        let Some(first) = chars.next() else {
            return Some("Echo".to_string());
        };
        let mut normalized = String::new();
        normalized.push(first.to_ascii_uppercase());
        normalized.push_str(chars.as_str());
        return Some(format!("Echo—{normalized}"));
    }
    Some("Echo".to_string())
}

fn buyback_marker_keyword_display(tokens: &[OwnedLexToken], words: &[&str]) -> Option<String> {
    if let Some(prefix) = keyword_mana_cost_prefix(tokens, 1) {
        Some(format!("Buyback {}", prefix.cost.to_oracle()))
    } else if words.len() > 1 {
        Some(format!("Buyback—{}", words[1..].join(" ")))
    } else {
        Some("Buyback".to_string())
    }
}

pub fn marker_text_from_words(words: &[&str]) -> Option<String> {
    let first = words.first().copied()?;
    let mut text = keyword_title(first);
    if words.len() > 1 {
        text.push(' ');
        text.push_str(&words[1..].join(" "));
    }
    Some(text)
}

pub fn parse_numeric_keyword_action<F>(
    words: &[&str],
    keyword: &'static str,
    build: F,
) -> Option<KeywordAction>
where
    F: FnOnce(u32) -> KeywordAction,
{
    if words.first().copied() != Some(keyword) {
        return None;
    }
    if let Some(amount) = words.get(1).and_then(|word| parse_named_number(word)) {
        return Some(build(amount));
    }
    Some(KeywordAction::Marker(keyword))
}

pub fn parse_dynamic_soulshift_keyword_action(words: &[&str]) -> Option<KeywordAction> {
    let parsed = parse_dynamic_soulshift_words(words)?;
    Some(KeywordAction::SoulshiftValue(crate::effect::Value::Count(
        parsed.count_filter,
    )))
}

pub enum KeywordCostFallback {
    MarkerOnly,
    MarkerOrText,
}

impl KeywordCostFallback {
    fn allows_marker_text(self) -> bool {
        match self {
            Self::MarkerOnly => false,
            Self::MarkerOrText => true,
        }
    }
}

pub fn parse_cost_keyword_action<F>(
    tokens: &[OwnedLexToken],
    keyword: &'static str,
    fallback: KeywordCostFallback,
    build: F,
) -> Option<KeywordAction>
where
    F: FnOnce(ManaCost) -> KeywordAction,
{
    let surface = parse_keyword_cost_action_surface_tokens(tokens, keyword)?;
    if let Some(cost) = surface.mana_cost {
        return Some(build(cost));
    }
    if fallback.allows_marker_text()
        && surface.has_payload
        && let Some(display) = marker_keyword_display(tokens)
    {
        return Some(KeywordAction::MarkerText(display));
    }
    Some(KeywordAction::Marker(keyword))
}

pub fn parse_single_word_keyword_action(word: &str) -> Option<KeywordAction> {
    SINGLE_WORD_KEYWORD_ACTIONS
        .iter()
        .find_map(|(keyword, action)| keyword_head_is(word, keyword).then(|| action.clone()))
}

pub fn is_known_keyword_action_head(word: &str) -> bool {
    let word = word.to_ascii_lowercase();
    let word = word.as_str();
    SIMPLE_HEAD_KEYWORD_ACTIONS
        .iter()
        .any(|(keyword, _)| keyword_head_is(word, keyword))
        || NUMERIC_KEYWORD_ACTIONS
            .iter()
            .any(|(keyword, _)| keyword_head_is(word, keyword))
        || MARKER_KEYWORD_FALLBACK_HEADS
            .iter()
            .any(|keyword| keyword_head_is(word, keyword))
        || MARKER_KEYWORD_IDS
            .iter()
            .any(|keyword| keyword_head_is(word, keyword))
        || COST_MARKER_KEYWORDS
            .iter()
            .any(|keyword| keyword_head_is(word, keyword))
        || matches!(
            word,
            "annihilator"
                | "afflict"
                | "awaken"
                | "bloodthirst"
                | "bushido"
                | "casualty"
                | "crew"
                | "dredge"
                | "devour"
                | "frenzy"
                | "poisonous"
                | "rampage"
                | "saddle"
                | "toxic"
        )
}

fn special_ability_phrase_action(kind: SpecialAbilityPhraseKind) -> KeywordAction {
    match kind {
        SpecialAbilityPhraseKind::VariableCasualtyPlaneswalkerCopy => {
            KeywordAction::VariableCasualtyPlaneswalkerCopy
        }
        SpecialAbilityPhraseKind::StartYourEngines => KeywordAction::StartYourEngines,
        SpecialAbilityPhraseKind::AnyLandwalk => {
            KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::AnyLand)
        }
        SpecialAbilityPhraseKind::NonbasicLandwalk => {
            KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::NonbasicLand)
        }
        SpecialAbilityPhraseKind::ArtifactLandwalk => {
            KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::ArtifactLand)
        }
    }
}

fn parse_special_ability_phrase(words: &[&str]) -> Option<KeywordAction> {
    parse_special_ability_phrase_words(words).map(special_ability_phrase_action)
}

fn parse_snow_landwalk_phrase(words: &[&str]) -> Option<KeywordAction> {
    let ["snow", subtype_walk] = words else {
        return None;
    };
    let action = parse_single_word_keyword_action(subtype_walk)?;
    let KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::Subtype { subtype, .. }) =
        action
    else {
        return None;
    };
    Some(KeywordAction::Landwalk(
        crate::static_abilities::LandwalkKind::Subtype {
            subtype,
            snow: true,
        },
    ))
}

#[derive(Clone, Copy)]
enum ExactAbilityPhrase {
    AffinityForArtifacts,
    FirstStrike,
    DoubleStrike,
    ForMirrodin,
    LivingWeapon,
    ModularSunburst,
    ProtectionFromAllColors,
    ProtectionFromColorless,
    ProtectionFromEverything,
    ProtectionFromColoredSpells,
}

const EXACT_ABILITY_PHRASES: &[(&[&str], ExactAbilityPhrase)] = &[
    (
        &["affinity", "for", "artifacts"],
        ExactAbilityPhrase::AffinityForArtifacts,
    ),
    (&["first", "strike"], ExactAbilityPhrase::FirstStrike),
    (&["double", "strike"], ExactAbilityPhrase::DoubleStrike),
    (&["for", "mirrodin"], ExactAbilityPhrase::ForMirrodin),
    (&["living", "weapon"], ExactAbilityPhrase::LivingWeapon),
    (
        &["modular", "sunburst"],
        ExactAbilityPhrase::ModularSunburst,
    ),
    (
        &["protection", "from", "all", "colors"],
        ExactAbilityPhrase::ProtectionFromAllColors,
    ),
    (
        &["protection", "from", "all", "color"],
        ExactAbilityPhrase::ProtectionFromAllColors,
    ),
    (
        &["protection", "from", "colorless"],
        ExactAbilityPhrase::ProtectionFromColorless,
    ),
    (
        &["protection", "from", "everything"],
        ExactAbilityPhrase::ProtectionFromEverything,
    ),
    (
        &[
            "protection",
            "from",
            "spells",
            "that",
            "are",
            "one",
            "or",
            "more",
            "colors",
        ],
        ExactAbilityPhrase::ProtectionFromColoredSpells,
    ),
];

fn exact_ability_phrase_action(kind: ExactAbilityPhrase) -> KeywordAction {
    match kind {
        ExactAbilityPhrase::AffinityForArtifacts => KeywordAction::AffinityForArtifacts,
        ExactAbilityPhrase::FirstStrike => KeywordAction::FirstStrike,
        ExactAbilityPhrase::DoubleStrike => KeywordAction::DoubleStrike,
        ExactAbilityPhrase::ForMirrodin => KeywordAction::ForMirrodin,
        ExactAbilityPhrase::LivingWeapon => KeywordAction::LivingWeapon,
        ExactAbilityPhrase::ModularSunburst => KeywordAction::ModularSunburst,
        ExactAbilityPhrase::ProtectionFromAllColors => KeywordAction::ProtectionFromAllColors,
        ExactAbilityPhrase::ProtectionFromColorless => KeywordAction::ProtectionFromColorless,
        ExactAbilityPhrase::ProtectionFromEverything => KeywordAction::ProtectionFromEverything,
        ExactAbilityPhrase::ProtectionFromColoredSpells => {
            let all_colors = crate::color::ColorSet::WHITE
                .union(crate::color::ColorSet::BLUE)
                .union(crate::color::ColorSet::BLACK)
                .union(crate::color::ColorSet::RED)
                .union(crate::color::ColorSet::GREEN);
            let mut filter = ObjectFilter::spell();
            filter.colors = Some(all_colors);
            KeywordAction::ProtectionFromFilter(filter)
        }
    }
}

fn parse_exact_ability_phrase(words: &[&str]) -> Option<KeywordAction> {
    crate::word_primitives::matching_value(words, EXACT_ABILITY_PHRASES)
        .map(exact_ability_phrase_action)
}

pub fn parse_ability_phrase(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
    // "can't be blocked by more than N creature(s)" — a grantable blocking
    // restriction that rides in keyword lists ("trample and can't be blocked
    // by more than one creature").
    {
        let words = crate::lexer::token_word_refs(tokens);
        let tail = crate::word_primitives::strip_any_prefix(
            &words,
            &[
                &["cant", "be", "blocked", "by", "more", "than"],
                &["can", "t", "be", "blocked", "by", "more", "than"],
            ],
        )
        .map(|(_, tail)| tail);
        if let Some(tail) = tail
            && tail.len() == 2
            && crate::word_primitives::at_is_any(tail, 1, &["creature", "creatures"])
            && let Some(count) = match tail.first().copied()? {
                "one" | "1" => Some(1u32),
                "two" | "2" => Some(2),
                "three" | "3" => Some(3),
                _ => None,
            }
        {
            return Some(KeywordAction::CantBeBlockedByMoreThan(count));
        }
    }
    let surface = parse_keyword_ability_surface_tokens(tokens)?;
    let phrase_tokens = &tokens[surface.phrase_first..];

    let word_view = ActivationRestrictionCompatWords::new(phrase_tokens);
    let words = word_view.to_word_refs();
    if surface.word_count == 0 {
        return None;
    }

    let (head, second) = lexed_head_words(phrase_tokens).unwrap_or(("", None));

    if let Some(action) =
        parse_special_ability_phrase(&words).or_else(|| parse_snow_landwalk_phrase(&words))
    {
        return Some(action);
    }

    if let KeywordAbilityHead::CumulativeUpkeep { cost } = &surface.head {
        let cost_tokens =
            strip_leading_keyword_cost_separator(&trim_commas(&tokens[cost.clone()])).to_vec();
        let text = cumulative_upkeep_text(&cost_tokens);

        match parse_compiler_activation_cost(&cost_tokens) {
            Ok(total_cost) => {
                return Some(KeywordAction::CumulativeUpkeep { total_cost, text });
            }
            Err(_) => {
                return None;
            }
        }
    }

    match surface.head {
        KeywordAbilityHead::Fuse => return Some(KeywordAction::Fuse),
        KeywordAbilityHead::Bolster {
            amount: Some(amount),
        } => return Some(KeywordAction::Bolster(amount)),
        KeywordAbilityHead::Bolster { amount: None } => {
            return Some(KeywordAction::Marker("bolster"));
        }
        _ => {}
    }

    if let Some(action) = parse_numeric_keyword_action(&words, "bushido", KeywordAction::Bushido) {
        return Some(action);
    }
    if let Some(action) = parse_numeric_keyword_action(&words, "frenzy", KeywordAction::Frenzy) {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "poisonous", KeywordAction::Poisonous)
    {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "bloodthirst", KeywordAction::Bloodthirst)
    {
        return Some(action);
    }
    if let Some(action) = parse_numeric_keyword_action(&words, "tribute", KeywordAction::Tribute) {
        return Some(action);
    }
    if let Some(action) = parse_numeric_keyword_action(&words, "afflict", KeywordAction::Afflict) {
        return Some(action);
    }
    if let Some(action) = parse_numeric_keyword_action(&words, "backup", KeywordAction::Backup) {
        return Some(action);
    }
    if let Some(action) = parse_numeric_keyword_action(&words, "rampage", KeywordAction::Rampage) {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "annihilator", KeywordAction::Annihilator)
    {
        return Some(action);
    }
    if keyword_head_is(head, "dredge")
        && let Some(amount) = second
        && let Some(amount) = parse_named_number(amount)
    {
        return Some(KeywordAction::Dredge(amount));
    }

    // Crew appears as "Crew N" and is often followed by inline restrictions/reminder text.
    if matches!(&surface.head, KeywordAbilityHead::Crew) {
        if words.len() >= 2
            && let Some(amount) = parse_named_number(words[1])
        {
            let has_sorcery_speed = surface.sorcery_speed_reminder;
            let has_once_per_turn = surface.once_per_turn_reminder;

            let timing = if has_sorcery_speed {
                ActivationTiming::SorcerySpeed
            } else if has_once_per_turn {
                ActivationTiming::OncePerTurn
            } else {
                ActivationTiming::AnyTime
            };

            return Some(KeywordAction::Crew {
                amount,
                timing,
                once_per_turn: has_once_per_turn,
            });
        }
        // Fallback: preserve unsupported crew variants as marker text.
        if let Some(display) = marker_keyword_display(phrase_tokens) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("crew"));
    }

    // Saddle appears as "Saddle N" and is often followed by reminder text.
    // Per CR 702.171a, Saddle can be activated only as a sorcery.
    if matches!(&surface.head, KeywordAbilityHead::Saddle) {
        if words.len() >= 2
            && let Some(amount) = parse_named_number(words[1])
        {
            let has_once_per_turn = surface.once_per_turn_reminder;

            let timing = ActivationTiming::SorcerySpeed;

            return Some(KeywordAction::Saddle {
                amount,
                timing,
                once_per_turn: has_once_per_turn,
            });
        }
        // Fallback: preserve unsupported saddle variants as marker text.
        if let Some(display) = marker_keyword_display(phrase_tokens) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("saddle"));
    }

    if let Some(action) =
        parse_numeric_keyword_action(&words, "afterlife", KeywordAction::Afterlife)
    {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "fabricate", KeywordAction::Fabricate)
    {
        return Some(action);
    }

    if let Some(action) = simple_keyword_action_for_head(head) {
        return Some(action);
    }

    if let Some(action) = parse_numeric_keyword_action(&words, "renown", KeywordAction::Renown) {
        return Some(action);
    }
    if let Some(action) = parse_dynamic_soulshift_keyword_action(&words) {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "soulshift", KeywordAction::Soulshift)
    {
        return Some(action);
    }

    if matches!(&surface.head, KeywordAbilityHead::AuraSwap)
        && let Some(prefix) = keyword_mana_cost_prefix(phrase_tokens, 2)
    {
        return Some(KeywordAction::AuraSwap(prefix.cost));
    }

    if keyword_head_is(head, "awaken")
        && let Some(amount_word) = words.get(1)
        && let Some(amount) = parse_named_number(amount_word)
        && let Some(prefix) = keyword_mana_cost_prefix(phrase_tokens, 2)
    {
        return Some(KeywordAction::Awaken {
            amount,
            cost: prefix.cost,
        });
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "outlast",
        KeywordCostFallback::MarkerOnly,
        KeywordAction::Outlast,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "scavenge",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Scavenge,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "unearth",
        KeywordCostFallback::MarkerOnly,
        KeywordAction::Unearth,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "recover",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Recover,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "embalm",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Embalm,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "eternalize",
        KeywordCostFallback::MarkerOrText,
        |cost| {
            KeywordAction::Eternalize(
                ironsmith_core::TotalCost::<crate::model::CompilerCost>::mana(cost),
            )
        },
    ) {
        return Some(action);
    }

    if !(keyword_head_is(head, "emerge") && second == Some("from"))
        && let Some(action) = parse_cost_keyword_action(
            phrase_tokens,
            "emerge",
            KeywordCostFallback::MarkerOrText,
            KeywordAction::Emerge,
        )
    {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "ninjutsu",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Ninjutsu,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "dash",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Dash,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "blitz",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Blitz,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "warp",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Warp,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "plot",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Plot,
    ) {
        return Some(action);
    }

    if keyword_head_is(head, "suspend") {
        if let Some(time_word) = words.get(1)
            && let Some(time) = parse_named_number(time_word)
            && let Some(prefix) = keyword_mana_cost_prefix(phrase_tokens, 2)
        {
            return Some(KeywordAction::Suspend {
                time,
                cost: prefix.cost,
            });
        }
        if words.len() == 1 {
            return Some(KeywordAction::Marker("suspend"));
        }
        if let Some(display) = marker_keyword_display(phrase_tokens) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("suspend"));
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "disturb",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Disturb,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "foretell",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Foretell,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "spectacle",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Spectacle,
    ) {
        return Some(action);
    }

    if keyword_head_is(head, "hideaway") {
        if words.len() == 1 {
            return Some(KeywordAction::MarkerText("Hideaway".to_string()));
        }
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if keyword_head_is(head, "mobilize") {
        if let Some(amount_word) = words.get(1)
            && let Some(amount) = parse_named_number(amount_word)
        {
            return Some(KeywordAction::Mobilize(amount));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Marker("mobilize"));
        }
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if keyword_head_is(head, "impending") {
        if words.len() == 1 {
            return Some(KeywordAction::MarkerText("Impending".to_string()));
        }
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if matches!(&surface.head, KeywordAbilityHead::EmergeFrom) {
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }
    if matches!(&surface.head, KeywordAbilityHead::JobSelect) {
        return Some(KeywordAction::MarkerText("Job select".to_string()));
    }
    if matches!(&surface.head, KeywordAbilityHead::UmbraArmor) {
        return Some(KeywordAction::UmbraArmor);
    }

    if keyword_head_is(head, "exert") {
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if keyword_head_is(head, "airbend") {
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "overload",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Overload,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        phrase_tokens,
        "cleave",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Cleave,
    ) {
        return Some(action);
    }

    if let KeywordAbilityHead::Echo { cost } = &surface.head {
        let raw_cost_tokens = trim_commas(&tokens[cost.clone()]);
        let cost_tokens = strip_leading_keyword_cost_separator(&raw_cost_tokens).to_vec();

        if !cost_tokens.is_empty() {
            match parse_payment_clause_as_total_cost(&cost_tokens) {
                Ok(Some(total_cost)) => {
                    let text = echo_text(&total_cost, &cost_tokens);
                    return Some(KeywordAction::Echo { total_cost, text });
                }
                Ok(None) | Err(_) => {
                    return None;
                }
            }
        }

        if words.len() == 1 {
            return Some(KeywordAction::Marker("echo"));
        }
        if let Some(display) = marker_keyword_display(phrase_tokens) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("echo"));
    }

    if let KeywordAbilityHead::Modular { sunburst } = &surface.head {
        if *sunburst {
            return Some(KeywordAction::ModularSunburst);
        }
        if words.len() >= 2
            && let Some(amount) = parse_named_number(words[1])
        {
            return Some(KeywordAction::Modular(amount));
        }
        return Some(KeywordAction::Marker("modular"));
    }

    if keyword_head_is(head, "graft") {
        if words.len() >= 2
            && let Some(amount) = parse_named_number(words[1])
        {
            return Some(KeywordAction::Graft(amount));
        }
        return Some(KeywordAction::Marker("graft"));
    }

    if keyword_head_is(head, "fading") {
        if words.len() >= 2
            && let Some(amount) = parse_named_number(words[1])
        {
            return Some(KeywordAction::Fading(amount));
        }
        return Some(KeywordAction::Marker("fading"));
    }

    if keyword_head_is(head, "vanishing") {
        if words.len() >= 2
            && let Some(amount) = parse_named_number(words[1])
        {
            return Some(KeywordAction::Vanishing(amount));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Vanishing(0));
        }
        return Some(KeywordAction::Marker("vanishing"));
    }

    if keyword_head_is(head, "harness") {
        if words.len() > 1 {
            return Some(KeywordAction::MarkerText(format!(
                "Harness {}",
                words[1..].join(" ")
            )));
        }
        return Some(KeywordAction::MarkerText("Harness".to_string()));
    }

    if let Some(action) = simple_keyword_action_for_head(head) {
        return Some(action);
    }
    if let Some(action) = match &surface.head {
        KeywordAbilityHead::ForMirrodin => Some(KeywordAction::ForMirrodin),
        KeywordAbilityHead::LivingWeapon => Some(KeywordAction::LivingWeapon),
        KeywordAbilityHead::BattleCry => Some(KeywordAction::BattleCry),
        KeywordAbilityHead::SplitSecond => Some(KeywordAction::SplitSecond),
        KeywordAbilityHead::ReadAhead => Some(KeywordAction::ReadAhead),
        KeywordAbilityHead::DoctorCompanion => Some(KeywordAction::Marker("doctor companion")),
        _ => None,
    } {
        return Some(action);
    }
    if let Some(action) = simple_keyword_action_for_head(head) {
        return Some(action);
    }

    // Casualty N - "as you cast this spell, you may sacrifice a creature with power N or greater"
    if keyword_head_is(head, "casualty") {
        if words.len() == 2
            && let Some(power) = parse_named_number(words[1])
        {
            return Some(KeywordAction::Casualty(power));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Casualty(1));
        }
        return None;
    }

    // Conspire - "as you cast this spell, you may tap two untapped creatures..."
    if keyword_head_is(head, "conspire") && words.len() == 1 {
        return Some(KeywordAction::Conspire);
    }

    // Amplify N - "as this enters, reveal any number of matching creature-type cards..."
    if keyword_head_is(head, "amplify") {
        if words.len() == 2
            && let Some(amount) = parse_named_number(words[1])
        {
            return Some(KeywordAction::Amplify(amount));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Amplify(1));
        }
        return None;
    }

    // Devour N - "as this enters, you may sacrifice any number of creatures..."
    if keyword_head_is(head, "devour") {
        if words.len() == 2
            && let Some(multiplier) = parse_named_number(words[1])
        {
            return Some(KeywordAction::Devour(multiplier));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Devour(1));
        }
        return None;
    }

    if let Some(first) = (!head.is_empty()).then_some(head)
        && is_marker_keyword_fallback_head(first)
    {
        if let Some(display) = marker_keyword_display(phrase_tokens) {
            return Some(KeywordAction::MarkerText(display));
        }
        if words.len() > 1 {
            return None;
        }
        return Some(KeywordAction::Marker(
            marker_keyword_id(first).expect("marker keyword id must exist for matched keyword"),
        ));
    }

    if words.len() == 1
        && let Some(action) = parse_single_word_keyword_action(words[0])
    {
        return Some(action);
    }

    if let Some(action) = parse_exact_ability_phrase(&words) {
        return Some(action);
    }

    if words.len() == 2
        && words.first().copied() == Some("outlast")
        && let Some(cost) = words.get(1)
    {
        let parsed_cost = parse_scryfall_mana_cost(cost).ok()?;
        return Some(KeywordAction::Outlast(parsed_cost));
    }

    if words.len() == 2
        && let (Some(keyword), Some(amount)) = (words.first(), words.get(1))
        && let Some(action) = numeric_keyword_action(keyword, amount)
    {
        return Some(action);
    }

    if words.len() == 3 && matches!(&surface.head, KeywordAbilityHead::ProtectionFrom) {
        let value = words[2];
        return if let Some(color) = parse_color(value) {
            Some(KeywordAction::ProtectionFrom(color))
        } else if value == "everything" {
            Some(KeywordAction::ProtectionFromEverything)
        } else {
            parse_card_type(value)
                .map(KeywordAction::ProtectionFromCardType)
                .or_else(|| parse_subtype_flexible(value).map(KeywordAction::ProtectionFromSubtype))
        };
    }

    // "toxic N" needs exactly 2 words
    if words.len() == 2 && matches!(&surface.head, KeywordAbilityHead::Toxic) {
        let amount = parse_named_number(words[1]).unwrap_or(1);
        return Some(KeywordAction::Toxic(amount));
    }
    if words.len() >= 2 {
        if matches!(&surface.head, KeywordAbilityHead::FirstStrike) {
            if words.len() > 2 && surface.conjoined {
                return None;
            }
            return Some(KeywordAction::FirstStrike);
        }
        if matches!(&surface.head, KeywordAbilityHead::DoubleStrike) {
            if words.len() > 2 && surface.conjoined {
                return None;
            }
            return Some(KeywordAction::DoubleStrike);
        }
    }
    if surface.unblockable_tail {
        return Some(KeywordAction::Unblockable);
    }
    None
}

pub fn maybe_strip_leading_damage_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let split = parse_keyword_damage_subject_split_tokens(tokens)?;
    match split.subject {
        KeywordDamageSubjectKind::It => Some(&tokens[split.action_first..]),
        KeywordDamageSubjectKind::SourceCandidate { word_count } => {
            let normalized = parse_normalized_keyword_words_tokens(tokens);
            let words = normalized
                .words
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            crate::util::is_source_reference_words(&words[..word_count])
                .then_some(&tokens[split.action_first..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        crate::lexer::lex_line(raw, 0).expect("test text should lex")
    }

    #[test]
    fn cumulative_upkeep_accepts_single_graveyard_bottom_library_payment() {
        let tokens = crate::lexer::lex_line(
            "Cumulative upkeep—Put two cards from a single graveyard on the bottom of their owner's library. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)",
            0,
        )
        .expect("line should lex");

        let action = parse_ability_phrase(&tokens).expect("cumulative upkeep should parse");
        let KeywordAction::CumulativeUpkeep { total_cost, .. } = action else {
            panic!("expected cumulative upkeep action, got {action:?}");
        };
        assert!(matches!(
            total_cost.costs(),
            [crate::model::CompilerCost::MoveChosenToLibraryBottom {
                count: 2,
                filter,
            }] if filter.zone == Some(Zone::Graveyard) && filter.single_graveyard
        ));

        let actions = crate::clause_support::parse_ability_line_lexed(&tokens)
            .expect("cumulative upkeep line should parse through ability-line facade");
        assert!(
            matches!(actions.as_slice(), [KeywordAction::CumulativeUpkeep { .. }]),
            "{actions:?}"
        );
    }

    #[test]
    fn activation_cost_accepts_owned_graveyard_bottom_library_payment() {
        let total_cost = parse_payment_clause_as_total_cost(&lex(
            "Put three cards from your graveyard on the bottom of your library",
        ))
        .unwrap()
        .expect("owned graveyard payment should parse");
        let [crate::model::CompilerCost::MoveChosenToLibraryBottom { count, filter }] =
            total_cost.costs()
        else {
            panic!("expected one typed graveyard move cost, got {total_cost:#?}");
        };
        assert_eq!(*count, 3);
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert!(!filter.single_graveyard);
    }

    #[test]
    fn dynamic_mana_and_life_payment_requires_a_complete_life_tail() {
        let total_cost = parse_payment_clause_as_total_cost(&lex("{2} and three life"))
            .unwrap()
            .expect("mana-and-life payment should parse");
        assert!(matches!(
            total_cost.costs(),
            [
                crate::model::CompilerCost::Mana(_),
                crate::model::CompilerCost::Life(Value::Fixed(3))
            ]
        ));

        assert!(
            parse_payment_clause_as_total_cost(&lex("{2} and three life quickly"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn typed_keyword_heads_drive_cost_and_damage_subject_parsing() {
        let action =
            parse_ability_phrase(&lex("Unearth {2}{B}")).expect("typed cost keyword should parse");
        let KeywordAction::Unearth(cost) = action else {
            panic!("expected unearth action, got {action:?}");
        };
        assert_eq!(cost.to_oracle(), "{2}{B}");

        let damage = lex("This creature deals three damage");
        let stripped = maybe_strip_leading_damage_subject_tokens(&damage)
            .expect("typed source subject should be stripped");
        assert!(stripped.first().is_some_and(|token| token.is_word("deals")));
    }
}
