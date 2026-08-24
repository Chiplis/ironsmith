use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, token_word_refs};
use super::super::super::super::util::{
    parse_filter_keyword_constraint_words, starts_filter_keyword_list_continuation_words,
};
use super::super::super::{leaf, primitives};
use super::AndPreservation;
use super::verbs::find_chain_verb_tokens;

const EACH_PLAYER_OR_OPPONENT_PREFIXES: &[&[&str]] = &[
    &["each", "player"],
    &["each", "players"],
    &["each", "opponent"],
    &["each", "opponents"],
    &["for", "each", "player"],
    &["for", "each", "players"],
    &["for", "each", "opponent"],
    &["for", "each", "opponents"],
];
const INLINE_TOKEN_RULES_TAIL_PREFIXES: &[&[&str]] = &[
    &["when"],
    &["whenever"],
    &["when", "this", "token"],
    &["whenever", "this", "token"],
    &["this", "token"],
    &["that", "token"],
    &["those", "tokens"],
    &["except", "it"],
    &["except", "they"],
    &["except", "its"],
    &["except", "their"],
    &["this", "creature"],
    &["that", "creature"],
    &["at", "the", "beginning"],
    &["at", "beginning"],
    &["sacrifice", "this", "token"],
    &["sacrifice", "that", "token"],
    &["sacrifice", "this", "permanent"],
    &["sacrifice", "that", "permanent"],
    &["sacrifice", "it"],
    &["sacrifice", "them"],
    &["it", "has"],
    &["it", "gains"],
    &["they", "have"],
    &["they", "gain"],
    &["equip"],
    &["equipped", "creature"],
    &["enchanted", "creature"],
    &["r"],
    &["t"],
];
const INLINE_CONTINUATION_WORDS: &[&str] = &[
    "it",
    "they",
    "that",
    "those",
    "this",
    "gain",
    "gains",
    "draw",
    "draws",
    "add",
    "deal",
    "deals",
    "destroy",
    "destroys",
    "exile",
    "exiles",
    "return",
    "returns",
    "tap",
    "untap",
    "sacrifice",
    "create",
    "put",
    "fights",
    "fight",
];
const CARD_TYPE_WORDS: &[&str] = &[
    "artifact",
    "artifacts",
    "battle",
    "battles",
    "creature",
    "creatures",
    "enchantment",
    "enchantments",
    "instant",
    "instants",
    "land",
    "lands",
    "planeswalker",
    "planeswalkers",
    "sorcery",
    "sorceries",
    "kindred",
];
const CARD_TYPE_LIST_NOUNS: &[&str] = &[
    "card",
    "cards",
    "spell",
    "spells",
    "permanent",
    "permanents",
];
const NONVERB_EFFECT_HEAD_WORDS: &[&str] = &[
    "double",
    "distribute",
    "copy",
    "copies",
    "support",
    "bolster",
    "adapt",
    "open",
    "cloak",
    "manifest",
    "populate",
    "connive",
    "endure",
    "endures",
    "explore",
    "explores",
    "earthbend",
    "harness",
    "harnesses",
];
const KEYWORD_ACTION_WORDS: &[&str] = &[
    "adapt",
    "adapts",
    "bolster",
    "bolsters",
    "connive",
    "connives",
    "earthbend",
    "earthbends",
    "harness",
    "harnesses",
    "endure",
    "endures",
    "explore",
    "explores",
    "cloak",
    "cloaks",
    "manifest",
    "manifests",
    "open",
    "opens",
    "support",
    "supports",
];
const PLAYER_MAY_PREFIXES: &[&[&str]] = &[
    &["you", "may"],
    &["they", "may"],
    &["the", "player", "may"],
    &["the", "players", "may"],
    &["that", "player", "may"],
    &["that", "players", "may"],
    &["that", "opponent", "may"],
    &["that", "opponents", "may"],
    &["target", "player", "may"],
    &["target", "players", "may"],
    &["target", "opponent", "may"],
    &["target", "opponents", "may"],
    &["defending", "player", "may"],
    &["attacking", "player", "may"],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ThenFollowupFacts {
    has_effect_head: bool,
    has_back_reference: bool,
    continues_inline_consult: bool,
    allow_back_reference: bool,
    allow_clash: bool,
    allow_attach: bool,
    allow_that_many: bool,
    allow_life_equal: bool,
    allow_damage_equal: bool,
    allow_that_much_damage: bool,
    allow_total_mana_value_damage: bool,
    allow_for_each_damage: bool,
    allow_source_deals_x_damage: bool,
    allow_dynamic_target_phase_out: bool,
    allow_target_pump: bool,
    allow_return_counter: bool,
    allow_return_attached: bool,
    allow_put_counter: bool,
    allow_put_hand: bool,
    allow_put_battlefield: bool,
    allow_put_back: bool,
    allow_exile_graveyard: bool,
}

impl ThenFollowupFacts {
    pub(super) fn should_split(self, ability_head: bool) -> bool {
        let has_effect_head =
            self.has_effect_head || ability_head || self.allow_dynamic_target_phase_out;
        if !has_effect_head {
            return false;
        }
        (!self.continues_inline_consult && (!self.has_back_reference || self.allow_back_reference))
            || self.allow_clash
            || self.allow_attach
            || self.allow_that_many
            || self.allow_life_equal
            || self.allow_damage_equal
            || self.allow_that_much_damage
            || self.allow_total_mana_value_damage
            || self.allow_for_each_damage
            || self.allow_source_deals_x_damage
            || self.allow_dynamic_target_phase_out
            || self.allow_target_pump
            || self.allow_return_counter
            || self.allow_return_attached
            || self.allow_put_counter
            || self.allow_put_hand
            || self.allow_put_battlefield
            || self.allow_put_back
            || self.allow_exile_graveyard
    }
}

pub(super) struct CommaBoundaryFacts {
    pub(super) before_has_verb: bool,
    pub(super) after_starts_effect: bool,
    pub(super) preserve_boundary: bool,
}

pub fn starts_with_inline_token_rules_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    let tokens = if primitives::parse_prefix(tokens, primitives::quote().void()).is_some() {
        tokens.get(1..).unwrap_or_default()
    } else {
        tokens
    };
    starts_any(tokens, INLINE_TOKEN_RULES_TAIL_PREFIXES)
}

pub fn is_token_creation_context_tokens(tokens: &[OwnedLexToken]) -> bool {
    contains_any(tokens, &["token", "tokens"])
        && find_chain_verb_tokens(tokens)
            .is_some_and(|verb| verb.kind == super::ChainVerbKind::Create)
}

pub fn starts_with_player_may_tokens(tokens: &[OwnedLexToken]) -> bool {
    starts_any(tokens, PLAYER_MAY_PREFIXES)
}

pub fn strip_leading_instead_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(tokens, primitives::kw("instead").void())?;
    if starts_any(rest, &[&["of"], &["if"]]) {
        return None;
    }
    let rest = super::super::super::super::lexer::trim_lexed_commas(rest);
    (!rest.is_empty()).then_some(rest)
}

pub fn has_basic_effect_head_tokens(tokens: &[OwnedLexToken]) -> bool {
    exact_any(
        tokens,
        &[
            &["repeat", "this", "process"],
            &["and", "repeat", "this", "process"],
        ],
    ) || starts_with_nonverb_effect_head(tokens)
        || is_cant_restriction(tokens)
        || is_life_total_change_restriction(tokens)
        || super::super::parse_persistent_no_maximum_hand_size_player_lexed(tokens).is_some()
}

pub fn has_extended_effect_head_tokens(tokens: &[OwnedLexToken]) -> bool {
    has_basic_effect_head_tokens(tokens)
        || parse_prevent_next_damage(tokens)
        || parse_prevent_all_damage(tokens)
        || is_can_attack_as_though(tokens)
        || is_attack_or_block_if_able(tokens)
        || is_attack_if_able(tokens)
        || is_must_block_if_able(tokens)
        || is_phase_clause(tokens)
        || is_choose_target_prelude(tokens)
}

pub fn preserve_and_reason(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
    extended: bool,
) -> Option<AndPreservation> {
    if current.is_empty() || remaining.is_empty() {
        return None;
    }
    if color_pair_boundary(current, remaining) {
        return Some(AndPreservation::ColorPair);
    }
    // `tapped and attacking` is one token-entry modifier. At this boundary
    // the token noun is necessarily in the right-hand slice, so the generic
    // "current clause contains token" guard below cannot recognize it yet.
    if starts_any(current, &[&["create"], &["creates"]])
        && ends_any(current, &[&["tapped"]])
        && starts_any(remaining, &[&["attacking"]])
        && (contains_any(current, &["token", "tokens"])
            || contains_any(remaining, &["token", "tokens"]))
    {
        return Some(AndPreservation::TokenRules);
    }
    if (is_token_creation_context_tokens(current) || has_inline_token_rules_context(current))
        && starts_with_inline_token_rules_tail_tokens(remaining)
    {
        return Some(AndPreservation::TokenRules);
    }
    if is_token_creation_context_tokens(current)
        && primitives::contains_word(current, "with")
        && parse_filter_keyword_constraint_words(&token_word_refs(remaining)).is_some()
    {
        // In `a token with flying and haste`, the second keyword is still
        // part of the token blueprint. It is not an independent granted
        // ability or a second action in the resolution chain.
        return Some(AndPreservation::TokenRules);
    }
    // A token-copy exception owns all of its characteristic modifiers.
    // Splitting `... except it has haste and loses soulbond` at the final
    // conjunction sends `loses soulbond` through ordinary ability-removal
    // parsing, where it must be rejected because marker removal is not
    // executable. Keep this narrow copy-only modifier inside the create
    // clause so copy lowering can set its typed `loses_soulbond` flag.
    if is_token_creation_context_tokens(current)
        && contains_any(current, &["copy", "copies"])
        && primitives::contains_word(current, "except")
        && starts_any(remaining, &[&["lose", "soulbond"], &["loses", "soulbond"]])
    {
        return Some(AndPreservation::TokenRules);
    }
    if is_token_creation_context_tokens(current)
        && contains_any(current, &["copy", "copies"])
        && primitives::contains_word(current, "except")
        && contains_all(current, &["half", "power"])
        && starts_any(
            remaining,
            &[&["their", "base", "toughness"], &["their", "toughness"]],
        )
    {
        return Some(AndPreservation::TokenRules);
    }
    if starts_any(
        current,
        &[
            &["destroy", "all"],
            &["exile", "all"],
            &["gain", "control", "of", "all"],
        ],
    ) && starts_any(
        remaining,
        &[
            &["aura"],
            &["auras"],
            &["equipment"],
            &["equipments"],
            &["enchantment"],
            &["enchantments"],
            &["artifact"],
            &["artifacts"],
        ],
    ) && primitives::contains_word(remaining, "attached")
    {
        return Some(AndPreservation::AttachmentList);
    }
    if starts_with_each_player_or_opponent(current)
        && primitives::contains_word(current, "may")
        && !starts_any(remaining, &[&["for", "each"], &["each"]])
    {
        return Some(AndPreservation::SharedPlayerMay);
    }
    if starts_any(remaining, &[&["the", "rest"], &["rest"]])
        && contains_all(current, &["put", "into", "hand"])
    {
        return Some(AndPreservation::PutRemainder);
    }
    if ends_any(current, &[&["as", "steps"]]) && starts_any(remaining, &[&["phases", "end"]]) {
        return Some(AndPreservation::StepAndPhase);
    }
    if starts_any(current, &[&["exchange"]])
        && has_zone_word(current)
        && first_word(remaining).is_some_and(is_zone_word)
    {
        return Some(AndPreservation::ExchangeZones);
    }
    if is_card_type_list_boundary(current, remaining) {
        return Some(AndPreservation::CardTypeList);
    }
    if is_creature_subtype_subject_list_boundary(current, remaining) {
        return Some(AndPreservation::CreatureSubtypeList);
    }
    if extended
        && ends_any(
            current,
            &[&["power"], &["total", "power"], &["base", "power"]],
        )
        && starts_any(remaining, &[&["toughness"]])
    {
        return Some(AndPreservation::PowerToughnessAxis);
    }
    if (contains_all(current, &["becomes", "with"]) || contains_all(current, &["emblem", "with"]))
        && (remaining
            .first()
            .is_some_and(|token| token.kind == TokenKind::Quote)
            || starts_with_inline_token_rules_tail_tokens(remaining))
    {
        return Some(AndPreservation::QuotedAbility);
    }
    if remaining
        .first()
        .is_some_and(|token| token.kind == TokenKind::Quote)
        && contains_any(current, &["gain", "gains", "has", "have", "lose", "loses"])
    {
        return Some(AndPreservation::QuotedAbility);
    }
    if extended
        && contains_any(current, &["get", "gets", "become", "becomes"])
        && first_word(remaining).is_some_and(|word| {
            matches!(word, "gain" | "gains" | "has" | "have" | "lose" | "loses")
        })
    {
        return Some(AndPreservation::SharedSubject);
    }
    None
}

pub(super) fn then_followup_facts(
    before: &[OwnedLexToken],
    after: &[OwnedLexToken],
    starts_with_for_each: bool,
) -> ThenFollowupFacts {
    let has_back_reference = contains_any(after, &["that", "it", "them", "its"]);
    let has_effect_head = find_chain_verb_tokens(after).is_some()
        || starts_with_nonverb_effect_head(after)
        || starts_with_player_may_tokens(after);
    let sacrifice_unless_payment = starts_any(after, &[&["sacrifice"], &["sacrifices"]])
        && primitives::contains_word(after, "unless")
        && contains_any(after, &["pay", "pays"]);
    let allow_back_reference = has_back_reference
        && ((starts_any(after, &[&["put"], &["double"]])
            && contains_any(after, &["counter", "counters"]))
            || starts_any(after, &[&["copy"], &["copies"]])
            || (starts_any(after, &[&["return"], &["returns"]])
                && contains_any(
                    after,
                    &[
                        "hand",
                        "hands",
                        "battlefield",
                        "graveyard",
                        "graveyards",
                        "library",
                        "libraries",
                        "exile",
                    ],
                ))
            || starts_with_player_may_tokens(after)
            || starts_any(
                after,
                &[&["transform"], &["transforms"], &["convert"], &["converts"]],
            )
            || sacrifice_unless_payment);
    let allow_clash = starts_any(before, &[&["clash"], &["clashes"]]);
    let allow_attach = starts_any(after, &[&["attach"], &["attaches"]]);
    let allow_that_many = !starts_with_for_each
        && has_back_reference
        && starts_any(
            after,
            &[
                &["draw", "that", "many"],
                &["draws", "that", "many"],
                &["discard", "that", "many"],
                &["discards", "that", "many"],
                &["create", "that", "many"],
                &["creates", "that", "many"],
            ],
        );
    let allow_life_equal =
        !starts_with_for_each && has_back_reference && life_equal_followup(after);
    let allow_damage_equal =
        !starts_with_for_each && has_back_reference && damage_equal_followup(after);
    let allow_that_much_damage = !starts_with_for_each
        && has_back_reference
        && find_chain_verb_tokens(after)
            .is_some_and(|found| found.kind == super::ChainVerbKind::Deal)
        && primitives::has_phrase(after, &["that", "much", "damage"]);
    let allow_total_mana_value_damage = !starts_with_for_each
        && has_back_reference
        && contains_any(after, &["deal", "deals"])
        && contains_all(after, &["damage", "equal", "total", "mana", "value"]);
    let allow_for_each_damage = has_back_reference
        && starts_any(after, &[&["for", "each"], &["each"]])
        && contains_any(after, &["deal", "deals"])
        && primitives::contains_word(after, "damage");
    // A singular source pronoun followed by a complete dynamic-damage
    // clause is an executable follow-up, not inline rules text belonging to
    // the preceding action. Keep this exact shape separable so the where-X
    // binding pass can type the amount after both actions have been parsed.
    let allow_source_deals_x_damage = !starts_with_for_each
        && starts_any(after, &[&["it", "deals", "x"]])
        && primitives::contains_word(after, "damage");
    let allow_dynamic_target_phase_out = !starts_with_for_each
        && starts_any(after, &[&["up", "to", "that", "many"], &["that", "many"]])
        && primitives::contains_word(after, "target")
        && primitives::has_phrase(after, &["phase", "out"]);
    let allow_target_pump = has_back_reference
        && !starts_with_for_each
        && starts_any(after, &[&["target"], &["up", "to"]])
        && contains_any(after, &["get", "gets", "become", "becomes"])
        && primitives::has_phrase(after, &["that", "player", "controls"]);
    let allow_return_counter = !starts_with_for_each
        && has_back_reference
        && starts_any(after, &[&["return"]])
        && contains_any(after, &["counter", "counters"])
        && has_any_phrase(after, &[&["on", "it"], &["on", "them"]]);
    let allow_return_attached = !starts_with_for_each
        && has_back_reference
        && starts_any(after, &[&["return"]])
        && primitives::contains_word(after, "battlefield")
        && has_any_phrase(
            after,
            &[
                &["attached", "to", "it"],
                &["attached", "to", "them"],
                &["attached", "to", "that", "card"],
                &["attached", "to", "that", "creature"],
                &["attached", "to", "that", "object"],
                &["attached", "to", "that", "permanent"],
                &["attached", "to", "those", "cards"],
                &["attached", "to", "those", "creatures"],
                &["attached", "to", "those", "objects"],
                &["attached", "to", "those", "permanents"],
            ],
        );
    let allow_put_counter = !starts_with_for_each
        && has_back_reference
        && starts_any(after, &[&["put"], &["puts"]])
        && primitives::contains_word(after, "battlefield")
        && contains_any(after, &["counter", "counters"])
        && has_any_phrase(after, &[&["on", "it"], &["on", "them"]]);
    let allow_put_hand = has_back_reference
        && starts_any(after, &[&["put"], &["puts"]])
        && contains_all(after, &["into", "hand"]);
    let allow_put_battlefield = has_back_reference
        && starts_any(after, &[&["put"], &["puts"]])
        && (primitives::has_phrase(after, &["onto", "the", "battlefield"])
            || primitives::has_phrase(after, &["onto", "battlefield"]));
    let allow_put_back = has_back_reference
        && starts_any(
            after,
            &[
                &["put", "it", "back"],
                &["put", "them", "back"],
                &["puts", "it", "back"],
                &["puts", "them", "back"],
            ],
        )
        && contains_all(after, &["any", "order"]);
    let allow_exile_graveyard = has_back_reference
        && starts_any(
            after,
            &[
                &["exile", "that", "player", "graveyard"],
                &["exile", "that", "players", "graveyard"],
                &["exile", "that", "player's", "graveyard"],
            ],
        )
        && contains_any(after, &["graveyard", "graveyards"]);
    let continues_inline_consult = starts_any(after, &[&["put"], &["puts"]])
        && contains_all(after, &["rest", "bottom", "library"])
        && contains_all(before, &["reveal", "top", "library"]);
    ThenFollowupFacts {
        has_effect_head,
        has_back_reference,
        continues_inline_consult,
        allow_back_reference,
        allow_clash,
        allow_attach,
        allow_that_many,
        allow_life_equal,
        allow_damage_equal,
        allow_that_much_damage,
        allow_total_mana_value_damage,
        allow_for_each_damage,
        allow_source_deals_x_damage,
        allow_dynamic_target_phase_out,
        allow_target_pump,
        allow_return_counter,
        allow_return_attached,
        allow_put_counter,
        allow_put_hand,
        allow_put_battlefield,
        allow_put_back,
        allow_exile_graveyard,
    }
}

pub(super) fn comma_boundary_facts(
    before: &[OwnedLexToken],
    after: &[OwnedLexToken],
) -> CommaBoundaryFacts {
    let before_has_verb = find_chain_verb_tokens(before).is_some();
    let after_verb = find_chain_verb_tokens(after);
    let explicit_subject_action = after_verb.is_some_and(|found| found.word_index > 0)
        && starts_any(
            after,
            &[
                &["you"],
                &["they"],
                &["it"],
                &["that", "player"],
                &["that", "opponent"],
                &["target", "player"],
                &["target", "opponent"],
                &["each", "player"],
                &["each", "opponent"],
                &["defending", "player"],
            ],
        );
    let after_starts_effect = after_verb.is_some_and(|found| found.word_index == 0)
        || explicit_subject_action
        || has_extended_effect_head_tokens(after);
    let duration_trigger = starts_any(before, &[&["until"], &["during"]])
        && (contains_any(before, &["whenever", "when"])
            || primitives::has_phrase(before, &["at", "the"]));
    let target_card_type_list = primitives::contains_word(before, "target")
        && (first_word(after).is_some_and(is_card_type_word)
            || starts_any(after, &[&["or"]]) && nth_word(after, 1).is_some_and(is_card_type_word))
        && !is_cant_restriction(after);
    let inline_token_rules = (is_token_creation_context_tokens(before)
        || has_inline_token_rules_context(before))
        && (starts_with_inline_token_rules_tail_tokens(after)
            || first_word(after).is_some_and(|word| {
                crate::slice_primitives::contains(INLINE_CONTINUATION_WORDS, &word)
            }));
    let token_copy_exception = contains_any(before, &["create", "creates"])
        && contains_any(before, &["token", "tokens"])
        && contains_any(before, &["copy", "copies"])
        && starts_any(
            after,
            &[&["except", "it"], &["except", "its"], &["except", "their"]],
        );
    let named_token_appositive =
        is_create_named_token_prefix(before) && starts_like_named_token_appositive(after);
    let filter_keyword_list =
        starts_filter_keyword_list_continuation_words(&token_word_refs(after));
    CommaBoundaryFacts {
        before_has_verb,
        after_starts_effect,
        preserve_boundary: starts_any(before, &[&["unless"]])
            || duration_trigger
            || contains_all(before, &["search", "library"])
            || target_card_type_list
            || filter_keyword_list
            || inline_token_rules
            || token_copy_exception
            || named_token_appositive,
    }
}

pub(super) fn starts_with_each_player_or_opponent(tokens: &[OwnedLexToken]) -> bool {
    starts_any(tokens, EACH_PLAYER_OR_OPPONENT_PREFIXES)
}

fn parse_prevent_next_damage(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            primitives::kw("prevent"),
            opt(primitives::kw("the")),
            primitives::kw("next"),
            any,
            primitives::kw("damage"),
            primitives::phrase(&["that", "would", "be", "dealt", "to"]),
            repeat_till::<_, _, (), _, _, _, _>(
                1..,
                any.void(),
                peek(primitives::phrase(&["this", "turn"])),
            ),
            primitives::phrase(&["this", "turn"]),
            primitives::sentence_end(),
        )
            .void(),
        "prevent next damage chain head",
    )
    .is_ok()
}

fn parse_prevent_all_damage(tokens: &[OwnedLexToken]) -> bool {
    fn target<'a>(input: &mut LexStream<'a>) -> WResult<()> {
        repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek(alt((primitives::phrase(&["this", "turn"]), eof.void()))),
        )
        .void()
        .parse_next(input)
    }
    let duration_first = (
        primitives::phrase(&[
            "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
        ]),
        target,
        primitives::sentence_end(),
    )
        .void();
    let target_first = (
        primitives::phrase(&[
            "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
        ]),
        target,
        primitives::phrase(&["this", "turn"]),
        primitives::sentence_end(),
    )
        .void();
    primitives::parse_all(
        tokens,
        alt((duration_first, target_first)),
        "prevent all damage chain head",
    )
    .is_ok()
}

fn is_can_attack_as_though(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(&["can", "attack"])).is_some()
        && ends_any(tokens, &[&["defender"]])
        && primitives::has_phrase(tokens, &["as", "though"])
        && contains_all(tokens, &["turn", "have"])
}

fn is_attack_or_block_if_able(tokens: &[OwnedLexToken]) -> bool {
    exact_tail_from_any_word(
        tokens,
        &["attack", "attacks"],
        &[
            &["attack", "or", "block", "this", "turn", "if", "able"],
            &["attacks", "or", "blocks", "this", "turn", "if", "able"],
            &["attacks", "or", "block", "this", "turn", "if", "able"],
            &["attack", "or", "blocks", "this", "turn", "if", "able"],
        ],
    )
}

fn is_attack_if_able(tokens: &[OwnedLexToken]) -> bool {
    exact_tail_from_any_word(
        tokens,
        &["attack", "attacks"],
        &[
            &["attack", "this", "turn", "if", "able"],
            &["attacks", "this", "turn", "if", "able"],
        ],
    )
}

fn is_must_block_if_able(tokens: &[OwnedLexToken]) -> bool {
    if starts_any(tokens, &[&["all", "creatures", "able", "to", "block"]])
        && ends_any(tokens, &[&["do", "so"]])
    {
        return true;
    }
    let Some((idx, _, _)) = find_any_word(tokens, &["block", "blocks"]) else {
        return false;
    };
    idx > 0
        && (exact_any(
            &tokens[idx..],
            &[
                &["block", "this", "turn", "if", "able"],
                &["blocks", "this", "turn", "if", "able"],
            ],
        ) || ends_any(&tokens[idx..], &[&["if", "able"]]))
}

fn is_phase_clause(tokens: &[OwnedLexToken]) -> bool {
    token_word_refs(tokens).len() >= 3
        && ends_any(
            tokens,
            &[
                &["phase", "out"],
                &["phases", "out"],
                &["phase", "in"],
                &["phases", "in"],
            ],
        )
}

fn is_choose_target_prelude(tokens: &[OwnedLexToken]) -> bool {
    starts_any(tokens, &[&["choose"], &["chooses"]]) && primitives::contains_word(tokens, "target")
}

fn starts_with_nonverb_effect_head(tokens: &[OwnedLexToken]) -> bool {
    starts_any(
        tokens,
        &[
            &["choose"],
            &["chooses"],
            &["you", "choose"],
            &["you", "chooses"],
            &["that", "player", "choose"],
            &["that", "player", "chooses"],
            &["that", "players", "choose"],
            &["that", "players", "chooses"],
            &["that", "opponent", "choose"],
            &["that", "opponent", "chooses"],
            &["that", "opponents", "choose"],
            &["that", "opponents", "chooses"],
            &["the", "voter", "choose"],
            &["the", "voter", "chooses"],
            &["target", "player", "choose"],
            &["target", "player", "chooses"],
            &["target", "players", "choose"],
            &["target", "players", "chooses"],
            &["target", "opponent", "choose"],
            &["target", "opponent", "chooses"],
            &["target", "opponents", "choose"],
            &["target", "opponents", "chooses"],
            &["after", "this", "phase"],
            &["after", "this", "main", "phase"],
        ],
    ) || first_word(tokens)
        .is_some_and(|word| crate::slice_primitives::contains(NONVERB_EFFECT_HEAD_WORDS, &word))
        || contains_any(tokens, KEYWORD_ACTION_WORDS)
}

fn is_cant_restriction(tokens: &[OwnedLexToken]) -> bool {
    contains_any(tokens, &["cant", "can't", "cannot"])
        && (contains_any(tokens, &["attack", "attacks"])
            || contains_any(tokens, &["block", "blocks", "blocked"]))
}

fn is_life_total_change_restriction(tokens: &[OwnedLexToken]) -> bool {
    contains_any(tokens, &["cant", "can't", "cannot"])
        && contains_any(tokens, &["total", "totals"])
        && primitives::contains_word(tokens, "life")
        && contains_any(tokens, &["change", "changed"])
}

fn has_inline_token_rules_context(tokens: &[OwnedLexToken]) -> bool {
    has_any_phrase(
        tokens,
        &[
            &["when", "this", "token"],
            &["whenever", "this", "token"],
            &["at", "the", "beginning", "of"],
        ],
    ) || contains_all(tokens, &["except", "copy", "token"])
}

fn is_create_named_token_prefix(tokens: &[OwnedLexToken]) -> bool {
    starts_any(tokens, &[&["create"], &["creates"]])
        && token_word_refs(tokens).len() > 1
        && !contains_any(tokens, &["token", "tokens"])
}

fn starts_like_named_token_appositive(tokens: &[OwnedLexToken]) -> bool {
    starts_any(tokens, &[&["a"], &["an"], &["the"]]) && contains_any(tokens, &["token", "tokens"])
}

fn color_pair_boundary(current: &[OwnedLexToken], remaining: &[OwnedLexToken]) -> bool {
    let Some(left) = last_word(current) else {
        return false;
    };
    let Some(right) = first_word(remaining) else {
        return false;
    };
    is_color_word(left) && is_color_word(right)
}

fn is_card_type_list_boundary(current: &[OwnedLexToken], remaining: &[OwnedLexToken]) -> bool {
    if !first_word(remaining).is_some_and(is_card_type_word)
        || !contains_any(remaining, CARD_TYPE_LIST_NOUNS)
    {
        return false;
    }
    let current_last_type = last_non_quantifier_word(current).is_some_and(is_card_type_word);
    current_last_type
        && contains_any(current, CARD_TYPE_WORDS)
        && (primitives::find_prefix(current, || primitives::comma().void()).is_some()
            || contains_any(current, &["or", "and/or"]))
}

/// Preserve the final conjunction in a serial creature-subtype subject.
///
/// The ordinary chain splitter searches the entire right-hand slice for a
/// verb, so `Birds, Frogs, Otters, and Rats you control get ...` otherwise
/// looks like a completed left action followed by `Rats ... get ...`. Require
/// every word on the left to be a known creature subtype and require the
/// right-hand slice to begin with another subtype before a later verb. This
/// keeps the subject union atomic without swallowing a real coordinated
/// action whose left side already contains a verb.
pub fn is_creature_subtype_subject_list_boundary(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if find_chain_verb_tokens(current).is_some() {
        return false;
    }

    let current_words = token_word_refs(current);
    if current_words.is_empty()
        || !current_words.iter().all(|word| {
            *word == "other"
                || leaf::parse_leaf_subtype_flexible_complete(word)
                    .is_ok_and(|subtype| subtype.is_creature_type())
        })
    {
        return false;
    }

    let Some(first_remaining) = first_word(remaining) else {
        return false;
    };
    leaf::parse_leaf_subtype_flexible_complete(first_remaining)
        .is_ok_and(|subtype| subtype.is_creature_type())
        && find_chain_verb_tokens(remaining).is_some_and(|found| found.word_index > 0)
}

#[cfg(test)]
#[path = "recognition_inline_tests.rs"]
mod tests;

#[path = "recognition/core_programs.rs"]
mod core_programs;
use core_programs::{
    contains_all, contains_any, ends_any, exact_any, exact_tail_from_any_word, find_any_word,
    first_word, has_any_phrase, is_color_word, last_word, nth_word, starts_any,
};
#[path = "recognition/library_programs.rs"]
mod library_programs;
use library_programs::is_card_type_word;
#[path = "recognition/zone_programs.rs"]
mod zone_programs;
use zone_programs::{has_zone_word, is_zone_word};
#[path = "recognition/condition_programs.rs"]
mod condition_programs;
use condition_programs::last_non_quantifier_word;
#[path = "recognition/combat_programs.rs"]
mod combat_programs;
use combat_programs::damage_equal_followup;
#[path = "recognition/resource_programs.rs"]
mod resource_programs;
use resource_programs::life_equal_followup;
