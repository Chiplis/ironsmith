use super::*;
use crate::runtime_backend::lexer::{
    find_token_word, token_slice_starts_with_at, token_slice_strip_any_word_prefix,
    token_slice_strip_word_prefix,
};
use crate::runtime_backend::util::parse_value;
use crate::runtime_backend::value_helpers::{
    parse_equal_to_aggregate_filter_value, parse_equal_to_number_of_filter_value,
};

const PAYMENT_FOR_EACH_PREFIXES: &[&[&str]] = &[&["for", "each"], &["each"]];
const UNTAP_ONLY_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["untap"], &["untaps"]]);
const UNTAP_RESTRICTION_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any & [&["untap"], &["untaps"]];
    contains_words & ["during"];
    contains_any_words & [&["step", "steps"]]
);
const CUMULATIVE_UPKEEP_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["cumulative", "upkeep"]);
const ADD_MANA_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["add"]);
const PAY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix_any & [&["pay"], &["pays"]]);
const MANA_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["mana"]);
const WHERE_X_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["where", "x", "is"], &["where", "x", "equals"]]);
const SAME_NAME_AS_TRIGGERING_SPELL_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["same", "name", "as", "the", "spell"],
            &["same", "name", "as", "that", "spell"],
        ]];
    contains_any_words & [&["graveyard", "graveyards"]]
);
const CREW_SORCERY_SPEED_REMINDER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["activate", "only", "as", "a", "sorcery"]]);
const ONCE_PER_TURN_REMINDER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["activate", "only", "once", "each", "turn"],
            &["activate", "only", "once", "per", "turn"],
        ]]
);
const AURA_SWAP_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["aura", "swap"]);
const EMERGE_FROM_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["emerge", "from"]);
const UMBRA_ARMOR_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["umbra", "armor"]);
const JOB_SELECT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["job", "select"]);
const TOXIC_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["toxic"]);
const FIRST_STRIKE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["first", "strike"]);
const DOUBLE_STRIKE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["double", "strike"]);
const PROTECTION_FROM_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["protection", "from"]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["and"]);
const AND_EXACT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const OR_EXACT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const CAN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["can"]);
const YOU_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const T_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["t"]);
const VE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["ve"]);
const CANT_CONTRACTION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["cannot"], &["can't"]]);
const YOUVE_CONTRACTION_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you've"]);
const COST_OR_COSTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["cost"], &["costs"]]);
const SUNBURST_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["sunburst"]);
const IT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["it"]);
const DEAL_OR_DEALS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["deal"], &["deals"]]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["or"]);
const AND_OR_CONNECTOR_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["or"]]);
const LIFE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["life"]);
const ENCHANTED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["enchanted"]);
const CONTROLLER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["controller"]);
const ATTACHED_CONTROLLER_OBJECT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["creature"],
            &["creatures"],
            &["permanent"],
            &["permanents"],
            &["artifact"],
            &["artifacts"],
            &["enchantment"],
            &["enchantments"],
            &["land"],
            &["lands"],
        ]
);
const DISCARD_OR_DISCARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["discard", "discards"]]);
const CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["card", "cards"]]);
const SPELL_OR_SPELLS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["spell", "spells"]]);
const EVERYTHING_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["everything"]);
const CANT_BE_BLOCKED_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["cant", "be", "blocked"], &["cannot", "be", "blocked"]]);

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
    ("demonstrate", KeywordAction::Demonstrate),
    ("rebound", KeywordAction::Rebound),
    ("ascend", KeywordAction::Ascend),
    ("compleated", KeywordAction::Marker("compleated")),
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

pub(crate) fn target_ast_to_object_filter(target: TargetAst) -> Option<ObjectFilter> {
    match target {
        TargetAst::Source(_) => Some(ObjectFilter::source()),
        TargetAst::Object(filter, _, _) => Some(filter),
        TargetAst::Spell(_) => Some(ObjectFilter::spell()),
        TargetAst::Tagged(tag, _) => Some(ObjectFilter::tagged(tag)),
        TargetAst::AnyOtherTarget(_) => {
            Some(ObjectFilter::default().not_tagged(TagKey::from(IT_TAG)))
        }
        TargetAst::WithCount(inner, _) => target_ast_to_object_filter(*inner),
        _ => None,
    }
}

pub(crate) fn is_supported_untap_restriction_tail(words: &[&str]) -> bool {
    if UNTAP_ONLY_TAIL_PATTERN.matches_words(words) {
        return true;
    }
    if !UNTAP_RESTRICTION_TAIL_PATTERN.matches_words(words) {
        return false;
    }

    let allowed = [
        "untap",
        "untaps",
        "during",
        "its",
        "their",
        "your",
        "controllers",
        "controller",
        "untap",
        "step",
        "steps",
        "next",
        "the",
    ];
    if words.iter().any(|word| !slice_contains(&allowed, word)) {
        return false;
    }

    true
}

pub(crate) fn normalize_cant_words(tokens: &[OwnedLexToken]) -> Vec<String> {
    let words = ActivationRestrictionCompatWords::new(tokens).to_word_refs();
    let mut normalized = Vec::with_capacity(words.len());
    let mut idx = 0;
    while idx < words.len() {
        if CANT_CONTRACTION_WORD_PATTERN.matches_word_at(&words, idx) {
            normalized.push("cant".to_string());
            idx += 1;
        } else if CAN_WORD_PATTERN.matches_word_at(&words, idx)
            && T_WORD_PATTERN.matches_word_at(&words, idx + 1)
        {
            normalized.push("cant".to_string());
            idx += 2;
        } else if YOUVE_CONTRACTION_WORD_PATTERN.matches_word_at(&words, idx) {
            normalized.push("youve".to_string());
            idx += 1;
        } else if YOU_WORD_PATTERN.matches_word_at(&words, idx)
            && VE_WORD_PATTERN.matches_word_at(&words, idx + 1)
        {
            normalized.push("youve".to_string());
            idx += 2;
        } else {
            normalized.push(words[idx].to_string());
            idx += 1;
        }
    }
    normalized
}

pub(crate) fn keyword_title(keyword: &str) -> String {
    let mut words = keyword.split_whitespace();
    let Some(first) = words.next() else {
        return String::new();
    };
    let mut out = String::new();
    let mut first_chars = first.chars();
    if let Some(ch) = first_chars.next() {
        out.push(ch.to_ascii_uppercase());
        out.push_str(first_chars.as_str());
    }
    for word in words {
        out.push(' ');
        out.push_str(word);
    }
    out
}

pub(crate) fn leading_mana_symbols_to_oracle(words: &[&str]) -> Option<(String, usize)> {
    if words.is_empty() {
        return None;
    }
    let mut pips = Vec::new();
    let mut consumed = 0usize;
    for word in words {
        let Ok(symbol) = parse_mana_symbol(word) else {
            break;
        };
        pips.push(vec![symbol]);
        consumed += 1;
    }
    if consumed == 0 {
        return None;
    }
    Some((ManaCost::from_pips(pips).to_oracle(), consumed))
}

fn cumulative_upkeep_text(words: &[&str]) -> String {
    let mut text = "Cumulative upkeep".to_string();
    let tail = words.get(2..).unwrap_or_default();
    if tail.is_empty() {
        return text;
    }

    if ADD_MANA_PREFIX_PATTERN.matches_words(tail)
        && let Some((cost, consumed)) = leading_mana_symbols_to_oracle(&tail[1..])
        && consumed + 1 == tail.len()
    {
        return format!("Cumulative upkeep—Add {cost}");
    }
    if let Some((cost, consumed)) = leading_mana_symbols_to_oracle(tail)
        && consumed == tail.len()
    {
        return format!("Cumulative upkeep {cost}");
    }
    if tail.len() == 3
        && OR_EXACT_WORD_PATTERN.matches_word(tail[1])
        && let (Some((left, 1)), Some((right, 1))) = (
            leading_mana_symbols_to_oracle(&tail[..1]),
            leading_mana_symbols_to_oracle(&tail[2..3]),
        )
    {
        return format!("Cumulative upkeep {left} or {right}");
    }

    let mut tail_text = tail.join(" ");
    if let Some(first) = tail_text.chars().next() {
        let upper = first.to_ascii_uppercase().to_string();
        let rest = &tail_text[first.len_utf8()..];
        tail_text = format!("{upper}{rest}");
    }
    text = format!("Cumulative upkeep—{tail_text}");
    text
}

fn strip_leading_keyword_cost_separator(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    while start < tokens.len() && matches!(tokens[start].kind, TokenKind::Dash | TokenKind::EmDash)
    {
        start += 1;
    }
    &tokens[start..]
}

fn echo_text(total_cost: &TotalCost, cost_tokens: &[OwnedLexToken]) -> String {
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
) -> Result<Option<Vec<Effect>>, CardTextError> {
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
        return Ok(Some(vec![Effect::unless_action(
            left,
            right,
            PlayerFilter::You,
        )]));
    }

    if let Ok(total_cost) = parse_activation_cost(&trimmed) {
        let effects = crate::costs::total_cost_to_payment_effects(&total_cost);
        if !effects.is_empty() {
            return Ok(Some(effects));
        }
    }

    let ast = match parse_effect_sentences_lexed(&trimmed) {
        Ok(ast) => ast,
        Err(_) => return Ok(None),
    };
    let mut ctx = crate::runtime_backend::EffectLoweringContext::new();
    let (effects, choices) =
        match crate::runtime_backend::compile_support::compile_effects(&ast, &mut ctx) {
            Ok(compiled) => compiled,
            Err(_) => return Ok(None),
        };
    if choices.is_empty() && !effects.is_empty() {
        Ok(Some(effects))
    } else {
        Ok(None)
    }
}

fn find_payment_alternative_or(tokens: &[OwnedLexToken]) -> Option<usize> {
    find_index_with(tokens, |idx, token| {
        OR_EXACT_WORD_PATTERN.matches_token(token) && !is_comparison_or_delimiter(tokens, idx)
    })
}

pub(crate) fn parse_payment_clause_as_total_cost(
    tokens: &[OwnedLexToken],
) -> Result<Option<TotalCost>, CardTextError> {
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
        return Ok(Some(TotalCost::one_of(vec![left, right])));
    }

    if let Some(dynamic_cost) = parse_dynamic_payment_clause_as_total_cost(&trimmed)? {
        return Ok(Some(dynamic_cost));
    }

    if let Ok(total_cost) = parse_activation_cost(&trimmed)
        && !total_cost.is_free()
    {
        return Ok(Some(total_cost));
    }

    let Some(effects) = parse_payment_clause_as_effects(&trimmed)? else {
        return Ok(None);
    };
    crate::costs::payment_effects_to_total_cost(effects)
        .map(Some)
        .map_err(CardTextError::ParseError)
}

fn parse_dynamic_payment_clause_as_total_cost(
    tokens: &[OwnedLexToken],
) -> Result<Option<TotalCost>, CardTextError> {
    let words_before_pay_strip = words(tokens);
    let tokens = if PAY_PREFIX_PATTERN.matches_words(&words_before_pay_strip) {
        &tokens[1..]
    } else {
        tokens
    };
    let tokens = trim_edge_punctuation(&trim_commas(tokens));
    if tokens.is_empty() {
        return Ok(None);
    }
    let token_words = words(&tokens);
    if MANA_PREFIX_PATTERN.matches_words(&token_words)
        && let Some(value) = parse_equal_to_aggregate_filter_value(&tokens)
            .or_else(|| parse_equal_to_number_of_filter_value(&tokens))
    {
        return Ok(Some(TotalCost::from_cost(
            crate::costs::Cost::dynamic_mana(ironsmith_core::DynamicManaCost::new(
                ManaCost::new(),
                None,
                Some(value),
                None,
                ironsmith_core::DynamicManaDisplayHint::ManaEqualTo,
            )),
        )));
    }

    let mut mana = Vec::new();
    let mut consumed = 0usize;
    for token in tokens.iter() {
        if let Some(group) = mana_pips_from_token(token) {
            mana.extend(group);
            consumed += 1;
            continue;
        }
        let Some(word) = token.as_word() else {
            break;
        };
        match parse_mana_symbol(word) {
            Ok(symbol) => {
                mana.push(symbol);
                consumed += 1;
            }
            Err(_) => break,
        }
    }
    if mana.is_empty() {
        return Ok(None);
    }
    let trailing = trim_edge_punctuation(&trim_commas(&tokens[consumed..]));
    if trailing.is_empty() {
        return Ok(None);
    }

    let mana_cost = ManaCost::from_symbols(mana.clone());
    let mut x_value = None;
    let mut additional_generic = None;
    let mut multiplier = None;
    let trailing_words = words(&trailing);
    if token_slice_first_is(&trailing, "and") {
        let life_tokens = trim_edge_punctuation(&trim_commas(&trailing[1..]));
        if let Some((amount, used)) = parse_value(&life_tokens)
            && life_tokens
                .get(used)
                .is_some_and(|token| LIFE_WORD_PATTERN.matches_token(token))
            && trim_edge_punctuation(&trim_commas(&life_tokens[used + 1..])).is_empty()
        {
            return Ok(Some(TotalCost::from_costs(vec![
                crate::costs::Cost::mana(mana_cost),
                crate::costs::Cost::life(amount),
            ])));
        }
        return Ok(None);
    } else if WHERE_X_PREFIX_PATTERN.matches_words(&trailing_words) {
        if !mana_cost.has_x() {
            return Err(CardTextError::ParseError(format!(
                "where-X payment clause has no X mana symbol (clause: '{}')",
                words(&tokens).join(" ")
            )));
        }
        x_value = parse_value_binding_clause(&trailing).or_else(|| {
            if SAME_NAME_AS_TRIGGERING_SPELL_GRAVEYARD_PATTERN.matches_words(&trailing_words) {
                Some(Value::Count(
                    ObjectFilter::default()
                        .in_zone(Zone::Graveyard)
                        .match_tagged(
                            TagKey::from("triggering"),
                            crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
                        ),
                ))
            } else {
                None
            }
        });
        if x_value.is_none() {
            return Err(CardTextError::ParseError(format!(
                "unsupported where-X payment clause (clause: '{}')",
                words(&tokens).join(" ")
            )));
        }
    } else if grammar::words_match_any_prefix(&trailing, PAYMENT_FOR_EACH_PREFIXES).is_some() {
        multiplier = parse_dynamic_cost_modifier_value(&trailing)?;
    } else if let Some(value) = parse_dynamic_cost_modifier_value(&trailing)? {
        additional_generic = Some(value);
    } else {
        return Ok(None);
    }

    Ok(Some(TotalCost::from_cost(
        crate::costs::Cost::dynamic_mana(ironsmith_core::DynamicManaCost::new(
            mana_cost,
            x_value,
            additional_generic,
            multiplier,
            ironsmith_core::DynamicManaDisplayHint::Default,
        )),
    )))
}

pub(crate) fn marker_keyword_id(keyword: &str) -> Option<&'static str> {
    MARKER_KEYWORD_IDS
        .iter()
        .copied()
        .find(|candidate| keyword_head_is(keyword, candidate))
}

pub(crate) fn marker_keyword_display(words: &[&str]) -> Option<String> {
    let keyword = words.first().copied()?;
    let title = keyword_title(keyword);

    if marker_keyword_set_contains(AMOUNT_MARKER_KEYWORDS, keyword) {
        let amount = words.get(1).and_then(|word| parse_named_number(word))?;
        return Some(format!("{title} {amount}"));
    }
    if marker_keyword_set_contains(COST_MARKER_KEYWORDS, keyword) {
        let (cost, _) = leading_mana_symbols_to_oracle(&words[1..])?;
        return Some(format!("{title} {cost}"));
    }
    if keyword_head_is(keyword, ECHO_MARKER_KEYWORD) {
        return echo_marker_keyword_display(words);
    }
    if keyword_head_is(keyword, BUYBACK_MARKER_KEYWORD) {
        return buyback_marker_keyword_display(words);
    }
    if keyword_head_is(keyword, SUSPEND_MARKER_KEYWORD) {
        let time = words.get(1).and_then(|word| parse_named_number(word))?;
        let (cost, _) = leading_mana_symbols_to_oracle(&words[2..])?;
        return Some(format!("Suspend {time}—{cost}"));
    }
    if keyword_head_is(keyword, REBOUND_MARKER_KEYWORD) {
        return Some("Rebound".to_string());
    }
    if keyword_head_is(keyword, SQUAD_MARKER_KEYWORD) {
        let (cost, _) = leading_mana_symbols_to_oracle(&words[1..])?;
        return Some(format!("Squad {cost}"));
    }
    None
}

fn echo_marker_keyword_display(words: &[&str]) -> Option<String> {
    if let Some((cost, _)) = leading_mana_symbols_to_oracle(&words[1..]) {
        return Some(format!("Echo {cost}"));
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

fn buyback_marker_keyword_display(words: &[&str]) -> Option<String> {
    if let Some((cost, _)) = leading_mana_symbols_to_oracle(&words[1..]) {
        Some(format!("Buyback {cost}"))
    } else if words.len() > 1 {
        Some(format!("Buyback—{}", words[1..].join(" ")))
    } else {
        Some("Buyback".to_string())
    }
}

pub(crate) fn marker_text_from_words(words: &[&str]) -> Option<String> {
    let first = words.first().copied()?;
    let mut text = keyword_title(first);
    if words.len() > 1 {
        text.push(' ');
        text.push_str(&words[1..].join(" "));
    }
    Some(text)
}

pub(crate) fn parse_numeric_keyword_action<F>(
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

pub(crate) enum KeywordCostFallback {
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

pub(crate) fn parse_cost_keyword_action<F>(
    words: &[&str],
    keyword: &'static str,
    fallback: KeywordCostFallback,
    build: F,
) -> Option<KeywordAction>
where
    F: FnOnce(ManaCost) -> KeywordAction,
{
    if words.first().copied() != Some(keyword) {
        return None;
    }
    if COST_OR_COSTS_WORD_PATTERN.matches_word_at(words, 1) {
        return None;
    }
    if let Some((cost_text, _consumed)) = leading_mana_symbols_to_oracle(&words[1..])
        && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
    {
        return Some(build(cost));
    }
    if fallback.allows_marker_text() && words.len() > 1 {
        if let Some(display) = marker_keyword_display(words) {
            return Some(KeywordAction::MarkerText(display));
        }
    }
    Some(KeywordAction::Marker(keyword))
}

pub(crate) fn parse_single_word_keyword_action(word: &str) -> Option<KeywordAction> {
    SINGLE_WORD_KEYWORD_ACTIONS
        .iter()
        .find_map(|(keyword, action)| keyword_head_is(word, keyword).then(|| action.clone()))
}

#[derive(Clone, Copy)]
enum SpecialAbilityPhrase {
    VariableCasualtyPlaneswalkerCopy,
    StartYourEngines,
    AnyLandwalk,
    NonbasicLandwalk,
    ArtifactLandwalk,
}

const VARIABLE_CASUALTY_PLANESWALKER_COPY_PREFIX: &[&str] = &[
    "casualty",
    "x",
    "the",
    "copy",
    "isnt",
    "legendary",
    "and",
    "has",
    "starting",
    "loyalty",
    "x",
];

const EXACT_SPECIAL_ABILITY_PHRASES: &[(&[&str], SpecialAbilityPhrase)] = &[
    (
        &["start", "your", "engines"],
        SpecialAbilityPhrase::StartYourEngines,
    ),
    (&["landwalk"], SpecialAbilityPhrase::AnyLandwalk),
    (
        &["nonbasic", "landwalk"],
        SpecialAbilityPhrase::NonbasicLandwalk,
    ),
    (
        &["artifact", "landwalk"],
        SpecialAbilityPhrase::ArtifactLandwalk,
    ),
];

fn special_ability_phrase_action(kind: SpecialAbilityPhrase) -> KeywordAction {
    match kind {
        SpecialAbilityPhrase::VariableCasualtyPlaneswalkerCopy => {
            KeywordAction::VariableCasualtyPlaneswalkerCopy
        }
        SpecialAbilityPhrase::StartYourEngines => KeywordAction::StartYourEngines,
        SpecialAbilityPhrase::AnyLandwalk => {
            KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::AnyLand)
        }
        SpecialAbilityPhrase::NonbasicLandwalk => {
            KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::NonbasicLand)
        }
        SpecialAbilityPhrase::ArtifactLandwalk => {
            KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::ArtifactLand)
        }
    }
}

fn parse_special_ability_phrase(words: &[&str]) -> Option<KeywordAction> {
    if words.starts_with(VARIABLE_CASUALTY_PLANESWALKER_COPY_PREFIX) {
        return Some(special_ability_phrase_action(
            SpecialAbilityPhrase::VariableCasualtyPlaneswalkerCopy,
        ));
    }
    EXACT_SPECIAL_ABILITY_PHRASES
        .iter()
        .find_map(|(phrase, kind)| (*phrase == words).then(|| special_ability_phrase_action(*kind)))
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
    EXACT_ABILITY_PHRASES
        .iter()
        .find_map(|(phrase, kind)| (*phrase == words).then(|| exact_ability_phrase_action(*kind)))
}

pub(crate) fn parse_ability_phrase(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
    let mut phrase_tokens = tokens;
    if phrase_tokens
        .first()
        .is_some_and(|token| AND_EXACT_WORD_PATTERN.matches_token(token))
    {
        phrase_tokens = &phrase_tokens[1..];
    }

    let word_view = ActivationRestrictionCompatWords::new(phrase_tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return None;
    }

    let (head, second) = lexed_head_words(phrase_tokens).unwrap_or(("", None));

    if let Some(action) =
        parse_special_ability_phrase(&words).or_else(|| parse_snow_landwalk_phrase(&words))
    {
        return Some(action);
    }

    if CUMULATIVE_UPKEEP_PREFIX_PATTERN.matches_words(&words) {
        let reminder_start =
            find_index(phrase_tokens, |token| token.is_period()).unwrap_or(phrase_tokens.len());
        let cost_tokens = trim_commas(&phrase_tokens[2..reminder_start]).to_vec();
        let text = cumulative_upkeep_text(&words);

        match parse_payment_clause_as_total_cost(&cost_tokens) {
            Ok(Some(total_cost)) => {
                return Some(KeywordAction::CumulativeUpkeep { total_cost, text });
            }
            Ok(None) | Err(_) => {
                return None;
            }
        }
    }

    if let Some(action) = parse_numeric_keyword_action(&words, "bushido", KeywordAction::Bushido) {
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
        && parse_named_number(amount).is_some()
    {
        return Some(KeywordAction::MarkerText(format!("Dredge {amount}")));
    }

    // Crew appears as "Crew N" and is often followed by inline restrictions/reminder text.
    if keyword_head_is(head, "crew") {
        if words.len() >= 2
            && let Some(amount) = parse_named_number(words[1])
        {
            let has_sorcery_speed = CREW_SORCERY_SPEED_REMINDER_PATTERN.matches_words(&words);
            let has_once_per_turn = ONCE_PER_TURN_REMINDER_PATTERN.matches_words(&words);

            let mut additional_restrictions = Vec::new();
            let timing = if has_sorcery_speed {
                if has_once_per_turn {
                    additional_restrictions.push("Activate only once each turn.".to_string());
                }
                ActivationTiming::SorcerySpeed
            } else if has_once_per_turn {
                ActivationTiming::OncePerTurn
            } else {
                ActivationTiming::AnyTime
            };

            return Some(KeywordAction::Crew {
                amount,
                timing,
                additional_restrictions,
            });
        }
        // Fallback: preserve unsupported crew variants as marker text.
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("crew"));
    }

    // Saddle appears as "Saddle N" and is often followed by reminder text.
    // Per CR 702.171a, Saddle can be activated only as a sorcery.
    if keyword_head_is(head, "saddle") {
        if words.len() >= 2
            && let Some(amount) = parse_named_number(words[1])
        {
            let has_once_per_turn = ONCE_PER_TURN_REMINDER_PATTERN.matches_words(&words);

            let mut additional_restrictions = Vec::new();
            let timing = ActivationTiming::SorcerySpeed;
            if has_once_per_turn {
                additional_restrictions.push("Activate only once each turn.".to_string());
            }

            return Some(KeywordAction::Saddle {
                amount,
                timing,
                additional_restrictions,
            });
        }
        // Fallback: preserve unsupported saddle variants as marker text.
        if let Some(display) = marker_keyword_display(&words) {
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
    if let Some(action) =
        parse_numeric_keyword_action(&words, "soulshift", KeywordAction::Soulshift)
    {
        return Some(action);
    }

    if AURA_SWAP_PREFIX_PATTERN.matches_words(&words)
        && let Some((cost_text, _consumed)) = leading_mana_symbols_to_oracle(&words[2..])
        && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
    {
        return Some(KeywordAction::AuraSwap(cost));
    }

    if keyword_head_is(head, "awaken")
        && let Some(amount_word) = words.get(1)
        && let Some(amount) = parse_named_number(amount_word)
        && let Some((cost_text, _consumed)) = leading_mana_symbols_to_oracle(&words[2..])
        && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
    {
        return Some(KeywordAction::Awaken { amount, cost });
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "outlast",
        KeywordCostFallback::MarkerOnly,
        KeywordAction::Outlast,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "scavenge",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Scavenge,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "unearth",
        KeywordCostFallback::MarkerOnly,
        KeywordAction::Unearth,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "embalm",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Embalm,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "eternalize",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Eternalize,
    ) {
        return Some(action);
    }

    if !(keyword_head_is(head, "emerge") && second == Some("from"))
        && let Some(action) = parse_cost_keyword_action(
            &words,
            "emerge",
            KeywordCostFallback::MarkerOrText,
            KeywordAction::Emerge,
        )
    {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "ninjutsu",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Ninjutsu,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "dash",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Dash,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "blitz",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Blitz,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "warp",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Warp,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "plot",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Plot,
    ) {
        return Some(action);
    }

    if keyword_head_is(head, "suspend") {
        if let Some(time_word) = words.get(1)
            && let Some(time) = parse_named_number(time_word)
            && let Some((cost_text, _consumed)) = leading_mana_symbols_to_oracle(&words[2..])
            && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
        {
            return Some(KeywordAction::Suspend { time, cost });
        }
        if words.len() == 1 {
            return Some(KeywordAction::Marker("suspend"));
        }
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("suspend"));
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "disturb",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Disturb,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "foretell",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Foretell,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
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

    if EMERGE_FROM_PREFIX_PATTERN.matches_words(&words) {
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }
    if JOB_SELECT_PREFIX_PATTERN.matches_words(&words) {
        return Some(KeywordAction::MarkerText("Job select".to_string()));
    }
    if UMBRA_ARMOR_PREFIX_PATTERN.matches_words(&words) {
        return Some(KeywordAction::UmbraArmor);
    }

    if keyword_head_is(head, "exert") {
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if keyword_head_is(head, "airbend") {
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "overload",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Overload,
    ) {
        return Some(action);
    }

    if keyword_head_is(head, "echo") {
        let reminder_start = find_index(phrase_tokens, |token| {
            token.is_period() || token.kind == TokenKind::LParen
        })
        .or_else(|| find_token_word(&phrase_tokens[1..], "at").map(|idx| idx + 1))
        .unwrap_or(phrase_tokens.len());
        let raw_cost_tokens = trim_commas(&phrase_tokens[1..reminder_start]);
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
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("echo"));
    }

    if keyword_head_is(head, "modular") {
        if SUNBURST_WORD_PATTERN.matches_word_at(&words, 1) {
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
    if let Some((matched_phrase, _)) = token_slice_strip_any_word_prefix(
        phrase_tokens,
        &[
            &["for", "mirrodin"],
            &["living", "weapon"],
            &["battle", "cry"],
            &["split", "second"],
            &["read", "ahead"],
            &["doctor", "companion"],
        ],
    ) {
        return Some(match matched_phrase {
            ["for", "mirrodin"] => KeywordAction::ForMirrodin,
            ["living", "weapon"] => KeywordAction::LivingWeapon,
            ["battle", "cry"] => KeywordAction::BattleCry,
            ["split", "second"] => KeywordAction::SplitSecond,
            ["read", "ahead"] => KeywordAction::ReadAhead,
            ["doctor", "companion"] => KeywordAction::Marker("doctor companion"),
            _ => unreachable!("matched phrase must be one of the declared keyword heads"),
        });
    }
    if let Some(action) = simple_keyword_action_for_head(head) {
        return Some(action);
    }

    // Casualty N - "as you cast this spell, you may sacrifice a creature with power N or greater"
    if keyword_head_is(head, "casualty") {
        if words.len() == 2 {
            if let Some(power) = parse_named_number(words[1]) {
                return Some(KeywordAction::Casualty(power));
            }
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
        if words.len() == 2 {
            if let Some(amount) = parse_named_number(words[1]) {
                return Some(KeywordAction::Amplify(amount));
            }
        }
        if words.len() == 1 {
            return Some(KeywordAction::Amplify(1));
        }
        return None;
    }

    // Devour N - "as this enters, you may sacrifice any number of creatures..."
    if keyword_head_is(head, "devour") {
        if words.len() == 2 {
            if let Some(multiplier) = parse_named_number(words[1]) {
                return Some(KeywordAction::Devour(multiplier));
            }
        }
        if words.len() == 1 {
            return Some(KeywordAction::Devour(1));
        }
        return None;
    }

    if let Some(first) = (!head.is_empty()).then_some(head)
        && is_marker_keyword_fallback_head(first)
    {
        if let Some(display) = marker_keyword_display(&words) {
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

    if words.len() == 3 && PROTECTION_FROM_PREFIX_PATTERN.matches_words(&words) {
        let value = words[2];
        return if let Some(color) = parse_color(value) {
            Some(KeywordAction::ProtectionFrom(color))
        } else if EVERYTHING_PATTERN.matches_words(&[value]) {
            Some(KeywordAction::ProtectionFromEverything)
        } else {
            parse_card_type(value)
                .map(KeywordAction::ProtectionFromCardType)
                .or_else(|| parse_subtype_flexible(value).map(KeywordAction::ProtectionFromSubtype))
        };
    }

    // "toxic N" needs exactly 2 words
    if words.len() == 2 && TOXIC_PREFIX_PATTERN.matches_words(&words) {
        let amount = parse_named_number(words[1]).unwrap_or(1);
        return Some(KeywordAction::Toxic(amount));
    }
    if words.len() >= 2 {
        if FIRST_STRIKE_PREFIX_PATTERN.matches_words(&words) {
            if words.len() > 2 && AND_WORD_PATTERN.matches_words(&words) {
                return None;
            }
            return Some(KeywordAction::FirstStrike);
        }
        if DOUBLE_STRIKE_PREFIX_PATTERN.matches_words(&words) {
            if words.len() > 2 && AND_WORD_PATTERN.matches_words(&words) {
                return None;
            }
            return Some(KeywordAction::DoubleStrike);
        }
    }
    if CANT_BE_BLOCKED_SUFFIX_PATTERN.matches_words(&words) {
        return Some(KeywordAction::Unblockable);
    }
    None
}

pub(crate) fn rewrite_attached_controller_trigger_effect_tokens(
    trigger_tokens: &[OwnedLexToken],
    effects_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let trigger_words = crate::runtime_backend::token_word_refs(trigger_tokens);
    let references_enchanted_controller = find_window_by(&trigger_words, 3, |window| {
        ENCHANTED_WORD_PATTERN.matches_word(window[0])
            && ATTACHED_CONTROLLER_OBJECT_WORD_PATTERN.matches_word(window[1])
            && CONTROLLER_WORD_PATTERN.matches_word(window[2])
    })
    .is_some();
    if !references_enchanted_controller {
        return effects_tokens.to_vec();
    }

    let mut rewritten = Vec::with_capacity(effects_tokens.len());
    let mut idx = 0usize;
    while idx < effects_tokens.len() {
        if token_slice_starts_with_at(effects_tokens, idx, &["that", "creature"]) {
            let first_span = effects_tokens[idx].span();
            let second_span = effects_tokens[idx + 1].span();
            rewritten.push(OwnedLexToken::word("enchanted".to_string(), first_span));
            rewritten.push(OwnedLexToken::word("creature".to_string(), second_span));
            idx += 2;
            continue;
        }
        if token_slice_starts_with_at(effects_tokens, idx, &["that", "permanent"]) {
            let first_span = effects_tokens[idx].span();
            let second_span = effects_tokens[idx + 1].span();
            rewritten.push(OwnedLexToken::word("enchanted".to_string(), first_span));
            rewritten.push(OwnedLexToken::word("permanent".to_string(), second_span));
            idx += 2;
            continue;
        }
        rewritten.push(effects_tokens[idx].clone());
        idx += 1;
    }

    rewritten
}

pub(crate) fn maybe_strip_leading_damage_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if tokens.len() < 2 {
        return None;
    }

    if IT_WORD_PATTERN.matches_word_at(&words, 0)
        && DEAL_OR_DEALS_WORD_PATTERN.matches_word_at(&words, 1)
    {
        return Some(&tokens[1..]);
    }

    for subject_len in 1..tokens.len() {
        if !DEAL_OR_DEALS_WORD_PATTERN.matches_word_at(&words, subject_len) {
            continue;
        }
        if crate::runtime_backend::front_end::shared::util::is_source_reference_words(
            &words[..subject_len],
        ) {
            return Some(&tokens[subject_len..]);
        }
    }
    None
}

pub(crate) fn looks_like_trigger_object_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }

    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }

    let starts_with_or = OR_EXACT_WORD_PATTERN.matches_word_at(&words, 0);
    let first_candidate = if starts_with_or {
        words.get(1).copied()
    } else {
        words.first().copied()
    };
    let Some(first_word) = first_candidate else {
        return false;
    };

    let type_like = parse_card_type(first_word).is_some()
        || parse_subtype_word(first_word).is_some()
        || str_strip_suffix(first_word, "s").is_some_and(|stem| {
            parse_card_type(stem).is_some() || parse_subtype_word(stem).is_some()
        });
    if !type_like {
        return false;
    }

    contains_token_kind(tokens, TokenKind::Comma)
}

pub(crate) fn looks_like_trigger_discard_qualifier_tail(
    trigger_prefix_tokens: &[OwnedLexToken],
    tail_tokens: &[OwnedLexToken],
) -> bool {
    if tail_tokens.is_empty() {
        return false;
    }

    let prefix_words = crate::runtime_backend::token_word_refs(trigger_prefix_tokens);
    if !DISCARD_OR_DISCARDS_WORD_PATTERN.matches_words(&prefix_words) {
        return false;
    }

    let tail_words = crate::runtime_backend::token_word_refs(tail_tokens);
    if tail_words.is_empty() {
        return false;
    }

    let Some(first_word) = tail_words.first().copied() else {
        return false;
    };
    let typeish = parse_card_type(first_word).is_some()
        || parse_non_type(first_word).is_some()
        || AND_OR_CONNECTOR_PATTERN.matches_word(first_word);
    if !typeish {
        return false;
    }

    find_index(tail_tokens, |token| token.is_comma()).is_some_and(|comma_idx| {
        let before_words = crate::runtime_backend::token_word_refs(&tail_tokens[..comma_idx]);
        CARD_OR_CARDS_WORD_PATTERN.matches_words(&before_words)
    })
}

pub(crate) fn looks_like_trigger_type_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }
    let first_is_card_type = parse_card_type(words[0]).is_some()
        || parse_subtype_word(words[0]).is_some()
        || str_strip_suffix(words[0], "s").is_some_and(|word| {
            parse_card_type(word).is_some() || parse_subtype_word(word).is_some()
        });
    first_is_card_type
        && SPELL_OR_SPELLS_WORD_PATTERN.matches_words(&words)
        && OR_WORD_PATTERN.matches_words(&words)
        && contains_token_kind(tokens, TokenKind::Comma)
}

pub(crate) fn looks_like_trigger_color_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }
    is_basic_color_word(words[0])
        && OR_WORD_PATTERN.matches_words(&words)
        && contains_token_kind(tokens, TokenKind::Comma)
}

pub(crate) fn looks_like_trigger_numeric_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 3 {
        return false;
    }
    if words[0].parse::<i32>().is_err() {
        return false;
    }
    let has_second_number = words.iter().skip(1).any(|word| word.parse::<i32>().is_ok());
    has_second_number && OR_WORD_PATTERN.matches_words(&words)
}

pub(crate) fn is_trigger_objectish_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || parse_subtype_word(word).is_some()
        || str_strip_suffix(word, "s").is_some_and(|stem| {
            parse_card_type(stem).is_some() || parse_subtype_word(stem).is_some()
        })
}
