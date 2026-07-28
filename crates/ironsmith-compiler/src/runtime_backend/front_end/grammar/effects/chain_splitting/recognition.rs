use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, token_word_refs};
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

pub(crate) fn starts_with_inline_token_rules_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    let tokens = if primitives::parse_prefix(tokens, primitives::quote().void()).is_some() {
        tokens.get(1..).unwrap_or_default()
    } else {
        tokens
    };
    starts_any(tokens, INLINE_TOKEN_RULES_TAIL_PREFIXES)
}

pub(crate) fn is_token_creation_context_tokens(tokens: &[OwnedLexToken]) -> bool {
    starts_any(tokens, &[&["create"]]) && contains_any(tokens, &["token", "tokens"])
}

pub(crate) fn starts_with_player_may_tokens(tokens: &[OwnedLexToken]) -> bool {
    starts_any(tokens, PLAYER_MAY_PREFIXES)
}

pub(crate) fn strip_leading_instead_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(tokens, primitives::kw("instead").void())?;
    if starts_any(rest, &[&["of"], &["if"]]) {
        return None;
    }
    let rest = super::super::super::super::lexer::trim_lexed_commas(rest);
    (!rest.is_empty()).then_some(rest)
}

pub(crate) fn has_basic_effect_head_tokens(tokens: &[OwnedLexToken]) -> bool {
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

pub(crate) fn has_extended_effect_head_tokens(tokens: &[OwnedLexToken]) -> bool {
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

pub(super) fn preserve_and_reason(
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
    if (is_token_creation_context_tokens(current) || has_inline_token_rules_context(current))
        && starts_with_inline_token_rules_tail_tokens(remaining)
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
            ));
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
                INLINE_CONTINUATION_WORDS
                    .iter()
                    .any(|expected| word == *expected)
            }));
    let named_token_appositive =
        is_create_named_token_prefix(before) && starts_like_named_token_appositive(after);
    CommaBoundaryFacts {
        before_has_verb,
        after_starts_effect,
        preserve_boundary: starts_any(before, &[&["unless"]])
            || duration_trigger
            || contains_all(before, &["search", "library"])
            || target_card_type_list
            || inline_token_rules
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
    ) || first_word(tokens).is_some_and(|word| {
        NONVERB_EFFECT_HEAD_WORDS
            .iter()
            .any(|expected| word == *expected)
    }) || contains_any(tokens, KEYWORD_ACTION_WORDS)
}

fn is_cant_restriction(tokens: &[OwnedLexToken]) -> bool {
    contains_any(tokens, &["cant", "can't", "cannot"])
        && (contains_any(tokens, &["attack", "attacks"])
            || contains_any(tokens, &["block", "blocks"]))
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

fn life_equal_followup(tokens: &[OwnedLexToken]) -> bool {
    starts_any(
        tokens,
        &[
            &["you", "gain", "life", "equal", "to", "that"],
            &["you", "gain", "life", "equal", "to", "its"],
            &["you", "gain", "life", "equal", "to", "their"],
            &["you", "lose", "life", "equal", "to", "that"],
            &["you", "lose", "life", "equal", "to", "its"],
            &["you", "lose", "life", "equal", "to", "their"],
            &["gain", "life", "equal", "to", "that"],
            &["gain", "life", "equal", "to", "its"],
            &["gain", "life", "equal", "to", "their"],
            &["gains", "life", "equal", "to", "that"],
            &["gains", "life", "equal", "to", "its"],
            &["gains", "life", "equal", "to", "their"],
            &["lose", "life", "equal", "to", "that"],
            &["lose", "life", "equal", "to", "its"],
            &["lose", "life", "equal", "to", "their"],
            &["loses", "life", "equal", "to", "that"],
            &["loses", "life", "equal", "to", "its"],
            &["loses", "life", "equal", "to", "their"],
        ],
    )
}

fn damage_equal_followup(tokens: &[OwnedLexToken]) -> bool {
    starts_any(
        tokens,
        &[
            &["it", "deal", "damage", "equal", "to"],
            &["it", "deals", "damage", "equal", "to"],
            &["that", "creature", "deal", "damage", "equal", "to"],
            &["that", "creature", "deals", "damage", "equal", "to"],
            &["that", "objects", "deal", "damage", "equal", "to"],
            &["that", "objects", "deals", "damage", "equal", "to"],
        ],
    ) || (find_chain_verb_tokens(tokens)
        .is_some_and(|found| found.kind == super::ChainVerbKind::Deal)
        && contains_all(tokens, &["damage", "equal", "to"]))
}

fn exact_tail_from_any_word(
    tokens: &[OwnedLexToken],
    words: &'static [&'static str],
    tails: &'static [&'static [&'static str]],
) -> bool {
    find_any_word(tokens, words).is_some_and(|(idx, _, _)| exact_any(&tokens[idx..], tails))
}

fn find_any_word<'a>(
    tokens: &'a [OwnedLexToken],
    words: &'static [&'static str],
) -> Option<(usize, (), &'a [OwnedLexToken])> {
    primitives::find_prefix(tokens, || {
        move |input: &mut LexStream<'a>| {
            for word in words {
                let mut probe = input.clone();
                if primitives::kw(word).parse_next(&mut probe).is_ok() {
                    *input = probe;
                    return Ok(());
                }
            }
            Err(ErrMode::Backtrack(ContextError::new()))
        }
    })
}

fn starts_any(tokens: &[OwnedLexToken], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| primitives::match_word_prefix(tokens, phrase).is_some())
}

fn ends_any(tokens: &[OwnedLexToken], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| primitives::match_word_suffix(tokens, phrase).is_some())
}

fn exact_any(tokens: &[OwnedLexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    phrases.iter().any(|phrase| {
        primitives::parse_all(
            tokens,
            (primitives::phrase(phrase), primitives::sentence_end()).void(),
            "chain exact phrase",
        )
        .is_ok()
    })
}

fn has_any_phrase(tokens: &[OwnedLexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| primitives::find_phrase_start(tokens, phrase).is_some())
}

fn contains_any(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    words
        .iter()
        .any(|word| primitives::contains_word(tokens, word))
}

fn contains_all(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    words
        .iter()
        .all(|word| primitives::contains_word(tokens, word))
}

fn first_word(tokens: &[OwnedLexToken]) -> Option<&str> {
    let mut input = LexStream::new(tokens);
    loop {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let token = parsed.ok()?;
        if let Some(word) = token.as_word() {
            return Some(word);
        }
    }
}

fn nth_word(tokens: &[OwnedLexToken], wanted: usize) -> Option<&str> {
    let mut input = LexStream::new(tokens);
    let mut index = 0usize;
    loop {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let token = parsed.ok()?;
        if let Some(word) = token.as_word() {
            if index == wanted {
                return Some(word);
            }
            index += 1;
        }
    }
}

fn last_word(tokens: &[OwnedLexToken]) -> Option<&str> {
    let mut input = LexStream::new(tokens);
    let mut last = None;
    loop {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = parsed else {
            return last;
        };
        if let Some(word) = token.as_word() {
            last = Some(word);
        }
    }
}

fn last_non_quantifier_word(tokens: &[OwnedLexToken]) -> Option<&str> {
    let mut input = LexStream::new(tokens);
    let mut last = None;
    loop {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = parsed else {
            return last;
        };
        if let Some(word) = token.as_word()
            && !matches!(word, "a" | "an" | "the" | "all" | "each")
        {
            last = Some(word);
        }
    }
}

fn has_zone_word(tokens: &[OwnedLexToken]) -> bool {
    token_word_refs(tokens)
        .iter()
        .any(|word| is_zone_word(word))
}

fn is_zone_word(word: &str) -> bool {
    leaf::parse_leaf_zone_complete(word).is_ok()
}

fn is_card_type_word(word: &str) -> bool {
    CARD_TYPE_WORDS.iter().any(|expected| word == *expected)
}

fn is_color_word(word: &str) -> bool {
    matches!(
        word,
        "white" | "blue" | "black" | "red" | "green" | "colorless"
    )
}

#[cfg(test)]
mod tests {
    use super::super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn recognizes_effect_heads_and_preserved_and_boundaries() {
        let tokens = lex_line(
            "Prevent the next 3 damage that would be dealt to any target this turn.",
            0,
        )
        .unwrap();
        assert!(has_extended_effect_head_tokens(&tokens));

        let tokens = lex_line("Create a white and blue creature token.", 0).unwrap();
        let and_idx = tokens
            .iter()
            .position(|token| token.is_word("and"))
            .unwrap();
        assert_eq!(
            preserve_and_reason(&tokens[..and_idx], &tokens[and_idx + 1..], true),
            Some(AndPreservation::ColorPair)
        );

        let tokens = lex_line(
            r#"You get an emblem with "You have no maximum hand size." and "{T}: Draw a card.""#,
            0,
        )
        .unwrap();
        let and_idx = tokens
            .iter()
            .position(|token| token.is_word("and"))
            .unwrap();
        assert_eq!(
            preserve_and_reason(&tokens[..and_idx], &tokens[and_idx + 1..], true),
            Some(AndPreservation::QuotedAbility)
        );

        let tokens = lex_line(
            "Until end of turn, target creature gains trample and \"Whenever this creature attacks, draw a card.\"",
            0,
        )
        .unwrap();
        let and_idx = tokens
            .iter()
            .position(|token| token.is_word("and"))
            .unwrap();
        assert_eq!(
            preserve_and_reason(&tokens[..and_idx], &tokens[and_idx + 1..], true),
            Some(AndPreservation::QuotedAbility)
        );
    }

    #[test]
    fn named_source_damage_equal_followup_is_an_effect_boundary() {
        let before = lex_line("Destroy target land", 0).unwrap();
        let after = lex_line(
            "Roiling Terrain deals damage to that land's controller equal to the number of land cards in that player's graveyard.",
            0,
        )
        .unwrap();

        assert!(then_followup_facts(&before, &after, false).should_split(false));
    }

    #[test]
    fn transform_back_reference_is_an_executable_then_boundary() {
        let before = lex_line("Untap it", 0).unwrap();
        let after = lex_line("transform it", 0).unwrap();

        assert!(then_followup_facts(&before, &after, false).should_split(false));
    }

    #[test]
    fn result_amount_damage_is_an_executable_then_boundary() {
        let before = lex_line("Put them into their owners' graveyards", 0).unwrap();
        let after = lex_line(
            "this enchantment deals that much damage to each opponent",
            0,
        )
        .unwrap();

        assert!(then_followup_facts(&before, &after, false).should_split(false));
    }
}
