use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::grammar::{leaf, primitives};
use crate::lexer::{LexStream, LexedClause, OwnedLexToken};
use crate::mana::ManaCost;
use crate::util::trim_edge_punctuation_tokens;

#[derive(Debug, Clone, Copy)]
pub struct DestroyConsultLoopShape<'a> {
    pub consult_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct FlashbackGrantShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct EachPlayerRevealTypesShape<'a> {
    pub battlefield_filter_tokens: &'a [OwnedLexToken],
    pub extra_filter_tokens: Option<&'a [OwnedLexToken]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayedUpkeepPaymentShape {
    pub mana: ManaCost,
}

#[derive(Debug, Clone, Copy)]
pub struct StartingEachPlayerOptionalRepeatShape<'a> {
    /// The first sentence with the participant-ordering prefix removed.
    ///
    /// Keeping the ordinary `each player may ...` clause lets the existing
    /// effect parser produce the complete typed optional action instead of
    /// teaching the sequence recognizer about individual action families.
    pub each_player_clause_tokens: &'a [OwnedLexToken],
}

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    trim_edge_punctuation_tokens(tokens)
}

fn article<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("the"),
        primitives::kw("a"),
        primitives::kw("an"),
    ))
    .void()
    .parse_next(input)
}

fn exact_unit<'a>(
    tokens: &'a [OwnedLexToken],
    parser: fn(&mut LexStream<'a>) -> WResult<()>,
) -> bool {
    primitives::parse_prefix(trimmed(tokens), parser)
        .is_some_and(|(_, rest)| trimmed(rest).is_empty())
}

fn is_article_token(token: &OwnedLexToken) -> bool {
    exact_unit(std::slice::from_ref(token), article)
}

fn without_articles(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    tokens
        .iter()
        .filter(|token| !is_article_token(token))
        .cloned()
        .collect()
}

fn exile_top_card<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["exile", "top", "card", "of", "your", "library"])
        .void()
        .parse_next(input)
}

fn iterative_keep<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&[
            "you", "may", "put", "that", "card", "into", "your", "hand", "unless", "it", "has",
            "same", "name", "as", "another", "card", "exiled", "this", "way",
        ]),
        primitives::phrase(&[
            "you", "may", "put", "it", "into", "your", "hand", "unless", "it", "has", "same",
            "name", "as", "another", "card", "exiled", "this", "way",
        ]),
    ))
    .void()
    .parse_next(input)
}

fn iterative_repeat<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        primitives::phrase(&[
            "repeat", "this", "process", "until", "you", "put", "card", "into", "your", "hand",
            "or", "you", "exile", "two", "cards", "with", "same", "name",
        ]),
        opt(primitives::comma()),
        primitives::phrase(&["whichever", "comes", "first"]),
    )
        .void()
        .parse_next(input)
}

pub fn parse_iterative_library_sequence_shape(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> bool {
    exact_unit(&without_articles(first), exile_top_card)
        && exact_unit(&without_articles(second), iterative_keep)
        && exact_unit(&without_articles(third), iterative_repeat)
}

fn pay_life_start<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["starting", "with", "you"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "each", "player", "may", "pay", "any", "amount", "of", "life",
    ])
    .void()
    .parse_next(input)
}

fn pay_life_repeat<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "repeat", "this", "process", "until", "no", "one", "pays", "life",
    ])
    .void()
    .parse_next(input)
}

fn create_rats<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "each", "player", "creates", "1/1", "black", "rat", "creature", "token", "for", "each",
        "1", "life", "they", "paid", "this", "way",
    ])
    .void()
    .parse_next(input)
}

pub fn parse_each_player_pay_life_sequence_shape(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> bool {
    exact_unit(&without_articles(first), pay_life_start)
        && exact_unit(&without_articles(second), pay_life_repeat)
        && exact_unit(&without_articles(third), create_rats)
}

fn starting_with_you_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["starting", "with", "you"])
        .void()
        .parse_next(input)?;
    opt(primitives::comma()).void().parse_next(input)
}

fn each_player_may_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["each", "player", "may"])
        .void()
        .parse_next(input)
}

fn repeat_until_no_one_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["repeat", "this", "process", "until", "no", "one"])
        .void()
        .parse_next(input)
}

fn verbs_match_optional_action(first: &[OwnedLexToken], repeated: &[OwnedLexToken]) -> bool {
    let first_words = LexedClause::new(first).word_refs();
    let repeated_words = LexedClause::new(repeated).word_refs();
    let (Some(first_verb), Some(repeated_verb)) = (first_words.first(), repeated_words.first())
    else {
        return false;
    };
    first_verb.eq_ignore_ascii_case(repeated_verb)
        || repeated_verb.eq_ignore_ascii_case(&format!("{first_verb}s"))
        || repeated_verb.eq_ignore_ascii_case(&format!("{first_verb}es"))
}

/// Recognize the reusable two-sentence process:
///
/// `Starting with you, each player may <action>. Repeat this process until no
/// one <action>.`
///
/// The second clause describes the loop's termination, not another action.
/// Only the leading verbs need agree: Oracle commonly abbreviates the repeated
/// action's object phrase (for example, "a permanent card from their hand" to
/// "a card").
pub fn parse_starting_each_player_optional_repeat_shape<'a>(
    first: &'a [OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<StartingEachPlayerOptionalRepeatShape<'a>> {
    let ((), after_starting) = primitives::parse_prefix(trimmed(first), starting_with_you_prefix)?;
    let each_player_clause_tokens = trimmed(after_starting);
    let ((), first_action) =
        primitives::parse_prefix(each_player_clause_tokens, each_player_may_prefix)?;
    let ((), repeated_action) =
        primitives::parse_prefix(trimmed(second), repeat_until_no_one_prefix)?;
    let first_action = trimmed(first_action);
    let repeated_action = trimmed(repeated_action);
    if first_action.is_empty()
        || repeated_action.is_empty()
        || !verbs_match_optional_action(first_action, repeated_action)
    {
        return None;
    }
    Some(StartingEachPlayerOptionalRepeatShape {
        each_player_clause_tokens,
    })
}

fn for_each_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["for", "each"])
        .void()
        .parse_next(input)
}

fn destroyed_this_way<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["destroyed", "this", "way"])
        .void()
        .parse_next(input)
}

fn exile_that_card_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["and", "exiles", "that", "card"]),
        primitives::phrase(&["and", "exile", "that", "card"]),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_destroy_consult_loop_shape(
    tokens: &[OwnedLexToken],
) -> Option<DestroyConsultLoopShape<'_>> {
    let clause = trimmed(tokens);
    let (comma_idx, (), after_comma) =
        primitives::find_prefix(clause, || primitives::comma().void())?;
    let prefix = without_articles(trimmed(&clause[..comma_idx]));
    primitives::parse_prefix(&prefix, for_each_prefix)?;
    primitives::find_prefix(&prefix, || destroyed_this_way)?;
    let consult_clause = trimmed(after_comma);
    let (tail_idx, (), _) = primitives::find_prefix(consult_clause, || exile_that_card_tail)?;
    let consult_tokens = trimmed(&consult_clause[..tail_idx]);
    (!consult_tokens.is_empty()).then_some(DestroyConsultLoopShape { consult_tokens })
}

fn all_markers_present(tokens: &[OwnedLexToken], words: &[&'static str]) -> bool {
    words
        .iter()
        .all(|word| primitives::find_prefix(tokens, || primitives::kw(word)).is_some())
}

pub fn parse_put_exiled_then_shuffle_shape(tokens: &[OwnedLexToken]) -> bool {
    let normalized = without_articles(trimmed(tokens));
    all_markers_present(
        &normalized,
        &[
            "players",
            "put",
            "exiled",
            "cards",
            "battlefield",
            "shuffle",
        ],
    ) || all_markers_present(
        &normalized,
        &[
            "player",
            "puts",
            "exiled",
            "cards",
            "battlefield",
            "shuffle",
        ],
    )
}

fn gain_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("gain"), primitives::kw("gains")))
        .void()
        .parse_next(input)
}

fn flashback_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["flashback", "until", "end", "of", "turn"])
        .void()
        .parse_next(input)
}

fn flashback_cost<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&[
            "the",
            "flashback",
            "cost",
            "is",
            "equal",
            "to",
            "its",
            "mana",
            "cost",
        ]),
        primitives::phrase(&[
            "that",
            "cards",
            "flashback",
            "cost",
            "is",
            "equal",
            "to",
            "its",
            "mana",
            "cost",
        ]),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_flashback_grant_shape<'a>(
    first: &'a [OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<FlashbackGrantShape<'a>> {
    let first = trimmed(first);
    let (gain_idx, (), after_gain) = primitives::find_prefix(first, || gain_marker)?;
    if !exact_unit(after_gain, flashback_tail) || !exact_unit(second, flashback_cost) {
        return None;
    }
    let target_tokens = trimmed(&first[..gain_idx]);
    (!target_tokens.is_empty()).then_some(FlashbackGrantShape { target_tokens })
}

fn each_player_shuffle_intro<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["each", "player", "shuffles", "all"])
        .void()
        .parse_next(input)
}

fn own_into_library<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["they", "own", "into", "their", "library"])
        .void()
        .parse_next(input)
}

fn reveal_that_many<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "then", "reveals", "that", "many", "cards", "from", "the", "top", "of", "their", "library",
    ])
    .void()
    .parse_next(input)
}

fn each_player_put_intro<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["each", "player", "puts", "all"])
        .void()
        .parse_next(input)
}

fn revealed_this_way<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["revealed", "this", "way"])
        .void()
        .parse_next(input)
}

fn then_same_for<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["then", "does", "the", "same", "for"])
        .void()
        .parse_next(input)
}

fn bottom_their_library<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["on", "the", "bottom", "of", "their", "library"])
        .void()
        .parse_next(input)
}

fn same_for<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["same", "for"])
        .void()
        .parse_next(input)
}

pub fn parse_each_player_reveal_types_shape<'a>(
    first: &[OwnedLexToken],
    second: &'a [OwnedLexToken],
) -> Option<EachPlayerRevealTypesShape<'a>> {
    let first = trimmed(first);
    primitives::parse_prefix(first, each_player_shuffle_intro)?;
    primitives::find_prefix(first, || own_into_library)?;
    primitives::find_prefix(first, || reveal_that_many)?;

    let second = trimmed(second);
    let ((), filter_surface) = primitives::parse_prefix(second, each_player_put_intro)?;
    primitives::find_prefix(second, || revealed_this_way)?;
    primitives::find_prefix(second, || then_same_for)?;
    primitives::find_prefix(second, || bottom_their_library)?;
    let (revealed_idx, (), _) = primitives::find_prefix(filter_surface, || revealed_this_way)?;
    let battlefield_filter_tokens = trimmed(&filter_surface[..revealed_idx]);
    if battlefield_filter_tokens.is_empty() {
        return None;
    }

    let extra_filter_tokens =
        if let Some((_, (), after_same)) = primitives::find_prefix(second, || same_for) {
            let after_same = trimmed(after_same);
            let end = primitives::find_prefix(after_same, || primitives::kw("then").void())
                .map(|(idx, _, _)| idx)
                .unwrap_or(after_same.len());
            let extra = trimmed(&after_same[..end]);
            (!extra.is_empty()).then_some(extra)
        } else {
            None
        };
    Some(EachPlayerRevealTypesShape {
        battlefield_filter_tokens,
        extra_filter_tokens,
    })
}

fn prevention_counter_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["for", "each", "1", "damage", "prevented", "this", "way"])
        .void()
        .parse_next(input)
}

fn counter_target<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["that", "creature"]),
        primitives::kw("it").void(),
        primitives::phrase(&["that", "permanent"]),
        primitives::phrase(&["that", "object"]),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_prevention_counter_followup_shape(tokens: &[OwnedLexToken]) -> bool {
    let normalized = without_articles(trimmed(tokens));
    if primitives::parse_prefix(&normalized, prevention_counter_prefix).is_none()
        || !all_markers_present(&normalized, &["put", "+1/+1", "counter", "on"])
    {
        return false;
    }
    let Some((_, (), after_on)) =
        primitives::find_prefix(&normalized, || primitives::kw("on").void())
    else {
        return false;
    };
    exact_unit(after_on, counter_target)
}

fn prevented_this_way<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "damage", "is", "prevented", "this", "way"])
        .void()
        .parse_next(input)
}

fn deal_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("deal"), primitives::kw("deals")))
        .void()
        .parse_next(input)
}

fn reflect_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["that", "much", "damage", "to", "any", "target"])
        .void()
        .parse_next(input)
}

pub fn parse_prevention_reflect_followup_shape(tokens: &[OwnedLexToken]) -> bool {
    let clause = trimmed(tokens);
    let Some(((), rest)) = primitives::parse_prefix(clause, prevented_this_way) else {
        return false;
    };
    let Some((deal_idx, (), after_deal)) = primitives::find_prefix(rest, || deal_marker) else {
        return false;
    };
    deal_idx > 0 && exact_unit(after_deal, reflect_tail)
}

fn prevention_gain_life_followup<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "you",
        "gain",
        "life",
        "equal",
        "to",
        "the",
        "damage",
        "prevented",
        "this",
        "way",
    ])
    .void()
    .parse_next(input)
}

pub fn parse_prevention_gain_life_followup_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_unit(trimmed(tokens), prevention_gain_life_followup)
}

fn exile_top_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["exile", "cards", "from", "the", "top", "of"]),
        primitives::phrase(&["exile", "cards", "from", "top", "of"]),
    ))
    .void()
    .parse_next(input)
}

fn prevented_amount<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["equal", "to", "the", "damage", "prevented", "this", "way"])
        .void()
        .parse_next(input)
}

pub fn parse_prevention_exile_top_followup_shape(tokens: &[OwnedLexToken]) -> bool {
    let clause = trimmed(tokens);
    primitives::parse_prefix(clause, exile_top_prefix).is_some()
        && primitives::find_prefix(clause, || prevented_amount).is_some()
}

fn untap_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["they", "dont", "untap", "during"]),
        primitives::phrase(&["they", "don't", "untap", "during"]),
        primitives::phrase(&["those", "permanents", "dont", "untap", "during"]),
        primitives::phrase(&["those", "permanents", "don't", "untap", "during"]),
    ))
    .void()
    .parse_next(input)
}

fn source_tapped_duration<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["for", "as", "long", "as"])
        .void()
        .parse_next(input)
}

fn remains_tapped<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["remains", "tapped"])
        .void()
        .parse_next(input)
}

fn source_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("this"),
        primitives::kw("thiss"),
        primitives::kw("source"),
        primitives::kw("artifact"),
        primitives::kw("creature"),
        primitives::kw("permanent"),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_source_tapped_lock_shape(tokens: &[OwnedLexToken]) -> bool {
    let clause = trimmed(tokens);
    primitives::parse_prefix(clause, untap_prefix).is_some()
        && primitives::find_prefix(clause, || source_tapped_duration).is_some()
        && primitives::find_prefix(clause, || remains_tapped).is_some()
        && primitives::find_prefix(clause, || source_marker).is_some()
}

pub fn parse_untap_clause_prefix_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(trimmed(tokens), untap_prefix).is_some()
}

fn upkeep_pay_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["at", "the", "beginning", "of", "your", "next", "upkeep"]),
        primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "upkeep"]),
    ))
    .void()
    .parse_next(input)?;
    winnow::combinator::opt(primitives::comma())
        .void()
        .parse_next(input)?;
    primitives::kw("pay").void().parse_next(input)
}

pub fn parse_delayed_upkeep_payment_shape(
    upkeep_tokens: &[OwnedLexToken],
    lose_tokens: &[OwnedLexToken],
) -> Option<DelayedUpkeepPaymentShape> {
    let ((), mana_tokens) = primitives::parse_prefix(trimmed(upkeep_tokens), upkeep_pay_prefix)?;
    let mana_tokens = trimmed(mana_tokens);
    if mana_tokens.is_empty()
        || !super::delayed_step_shapes::is_delayed_lose_game_unless_paid_shape(lose_tokens)
    {
        return None;
    }
    let mana = leaf::parse_leaf_mana_cost_tokens(mana_tokens).ok()?;
    Some(DelayedUpkeepPaymentShape { mana })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{LexedClause, lex_line};

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_flashback_and_prevention_followups() {
        let first = lex("Target card gains flashback until end of turn");
        let second = lex("The flashback cost is equal to its mana cost");
        let shape = parse_flashback_grant_shape(&first, &second).unwrap();
        assert_eq!(
            LexedClause::new(shape.target_tokens).word_refs(),
            vec!["target", "card"]
        );
        assert!(parse_prevention_reflect_followup_shape(&lex(
            "If damage is prevented this way, this creature deals that much damage to any target"
        )));
    }

    #[test]
    fn parses_punctuated_iterative_library_sequence() {
        let first = lex("Exile the top card of your library.");
        let second = lex(
            "You may put that card into your hand unless it has the same name as another card exiled this way.",
        );
        let third = lex(
            "Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.",
        );
        assert!(parse_iterative_library_sequence_shape(
            &first, &second, &third
        ));
    }

    #[test]
    fn parses_punctuated_each_player_pay_life_sequence() {
        let first = lex("Starting with you, each player may pay any amount of life.");
        let second = lex("Repeat this process until no one pays life.");
        let third = lex(
            "Each player creates a 1/1 black Rat creature token for each 1 life they paid this way.",
        );
        assert!(parse_each_player_pay_life_sequence_shape(
            &first, &second, &third
        ));
    }

    #[test]
    fn parses_generic_starting_each_player_optional_repeat() {
        let eureka_first = lex(
            "Starting with you, each player may put a permanent card from their hand onto the battlefield.",
        );
        let eureka_repeat =
            lex("Repeat this process until no one puts a card onto the battlefield.");
        let eureka =
            parse_starting_each_player_optional_repeat_shape(&eureka_first, &eureka_repeat)
                .expect("the repeated optional action should be recognized");
        assert_eq!(
            LexedClause::new(eureka.each_player_clause_tokens).word_refs(),
            vec![
                "each",
                "player",
                "may",
                "put",
                "a",
                "permanent",
                "card",
                "from",
                "their",
                "hand",
                "onto",
                "the",
                "battlefield",
            ]
        );

        let pay_first = lex("Starting with you, each player may pay any amount of life.");
        let pay_repeat = lex("Repeat this process until no one pays life.");
        assert!(
            parse_starting_each_player_optional_repeat_shape(&pay_first, &pay_repeat).is_some(),
            "the recognizer should be action-generic and tolerate third-person verb agreement"
        );
    }

    #[test]
    fn rejects_unrelated_repeat_action() {
        let first = lex("Starting with you, each player may discard a card.");
        let second = lex("Repeat this process until no one draws a card.");
        assert!(parse_starting_each_player_optional_repeat_shape(&first, &second).is_none());
    }

    #[test]
    fn parses_delayed_upkeep_payment() {
        let upkeep = lex("At the beginning of your next upkeep, pay {2}{U}");
        let lose = lex("If you don't, you lose the game");
        let shape = parse_delayed_upkeep_payment_shape(&upkeep, &lose).unwrap();
        assert_eq!(shape.mana.to_oracle(), "{2}{U}");
    }
}
