use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageSourceControllerKind {
    None,
    You,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageSourceShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub controller: DamageSourceControllerKind,
    /// Qualifiers that follow the controller ("a source you control with an
    /// odd mana value would ...").
    pub trailing_filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageMultiplierSpec<'a> {
    pub source: DamageSourceShape<'a>,
    pub damaged_tokens: &'a [OwnedLexToken],
    pub factor: u32,
    pub combat_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdditiveDamageAmountSpec<'a> {
    pub source: DamageSourceShape<'a>,
    pub damaged_tokens: &'a [OwnedLexToken],
    pub repeated_target_tokens: Option<&'a [OwnedLexToken]>,
    pub delta: i32,
    pub noncombat_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreventDamageToYouSpec<'a> {
    pub source_tokens: &'a [OwnedLexToken],
    pub amount: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageAmountTail<'a> {
    Instead,
    ToThatTarget(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRedirectControllerSpec<'a> {
    pub source_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatMaximumKind {
    AttackYou,
    Attack,
    Block,
}

pub fn parse_damage_multiplier_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DamageMultiplierSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_damage_multiplier_lexed,
        "damage multiplier line",
    )
    .ok()
}

pub fn parse_additive_damage_amount_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AdditiveDamageAmountSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_additive_damage_amount_lexed,
        "additive damage amount replacement",
    )
    .ok()
}

pub fn parse_minimum_red_noncombat_damage_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_minimum_red_noncombat_damage_lexed,
        "minimum red noncombat damage replacement",
    )
    .is_ok()
}

pub fn parse_prevent_damage_to_you_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PreventDamageToYouSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_prevent_damage_to_you_lexed,
        "prevent damage to you from source filter",
    )
    .ok()
}

pub fn parse_damage_redirect_controller_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DamageRedirectControllerSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_damage_redirect_controller_lexed,
        "damage redirect to source controller",
    )
    .ok()
}

pub fn parse_combat_maximum_tail_tokens(tokens: &[OwnedLexToken]) -> Option<CombatMaximumKind> {
    primitives::parse_all(
        tokens,
        parse_combat_maximum_tail_lexed,
        "combat maximum tail",
    )
    .ok()
}

fn parse_damage_multiplier_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DamageMultiplierSpec<'a>> {
    primitives::kw("if").parse_next(input)?;
    let source = parse_damage_source_shape_lexed(input)?;
    let combat_only = alt((
        primitives::phrase(&["would", "deal", "combat", "damage", "to"]).value(true),
        primitives::phrase(&["would", "deal", "damage", "to"]).value(false),
    ))
    .parse_next(input)?;
    let damaged_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((
            primitives::phrase(&["it", "deals"]),
            alt((primitives::kw("double"), primitives::kw("triple"))),
            primitives::phrase(&["that", "damage", "to"]),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["it", "deals"]).parse_next(input)?;
    let factor = alt((
        primitives::kw("double").value(2),
        primitives::kw("triple").value(3),
    ))
    .parse_next(input)?;
    primitives::phrase(&["that", "damage", "to", "that"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("instead")))
        .void()
        .parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(DamageMultiplierSpec {
        source,
        damaged_tokens: trim_lexed_commas(damaged_tokens),
        factor,
        combat_only,
    })
}

fn parse_additive_damage_amount_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AdditiveDamageAmountSpec<'a>> {
    primitives::kw("if").parse_next(input)?;
    let source = parse_damage_source_shape_lexed(input)?;
    let noncombat_only = alt((
        primitives::phrase(&["would", "deal", "noncombat", "damage", "to"]).value(true),
        primitives::phrase(&["would", "deal", "damage", "to"]).value(false),
    ))
    .parse_next(input)?;
    let damaged_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((
            opt(primitives::comma()),
            primitives::phrase(&["it", "deals", "that", "much", "damage", "plus"]),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "deals", "that", "much", "damage", "plus"]).parse_next(input)?;
    let delta = leaf::parse_leaf_number_prefix_lexed
        .try_map(i32::try_from)
        .parse_next(input)?;
    let repeated_target_tokens = match parse_damage_amount_tail_lexed(input)? {
        DamageAmountTail::Instead => None,
        DamageAmountTail::ToThatTarget(tokens) => Some(tokens),
    };
    Ok(AdditiveDamageAmountSpec {
        source,
        damaged_tokens: trim_lexed_commas(damaged_tokens),
        repeated_target_tokens,
        delta,
        noncombat_only,
    })
}

fn parse_damage_source_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DamageSourceShape<'a>> {
    alt((
        parse_explicit_damage_source_shape_lexed,
        parse_object_damage_source_shape_lexed,
    ))
    .parse_next(input)
}

fn parse_explicit_damage_source_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DamageSourceShape<'a>> {
    let filter_tokens =
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::kw("source")))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::kw("source").parse_next(input)?;
    let controller = opt(alt((
        primitives::phrase(&["you", "control"]).value(DamageSourceControllerKind::You),
        alt((
            primitives::phrase(&["an", "opponent", "controls"]),
            primitives::phrase(&["opponent", "controls"]),
        ))
        .value(DamageSourceControllerKind::Opponent),
    )))
    .map(|controller| controller.unwrap_or(DamageSourceControllerKind::None))
    .parse_next(input)?;
    let trailing_filter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(alt((
            primitives::phrase(&["would", "deal", "combat", "damage", "to"]),
            primitives::phrase(&["would", "deal", "noncombat", "damage", "to"]),
            primitives::phrase(&["would", "deal", "damage", "to"]),
        ))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    let filter_tokens = trim_lexed_commas(filter_tokens);
    let filter_tokens = if primitives::parse_all(
        filter_tokens,
        alt((primitives::kw("a"), primitives::kw("an"))),
        "unqualified damage source article",
    )
    .is_ok()
    {
        &filter_tokens[..0]
    } else {
        filter_tokens
    };
    Ok(DamageSourceShape {
        filter_tokens,
        controller,
        trailing_filter_tokens: trim_lexed_commas(trailing_filter_tokens),
    })
}

fn parse_object_damage_source_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DamageSourceShape<'a>> {
    let filter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((
            primitives::phrase(&["would", "deal", "combat", "damage", "to"]),
            primitives::phrase(&["would", "deal", "noncombat", "damage", "to"]),
            primitives::phrase(&["would", "deal", "damage", "to"]),
        ))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    Ok(DamageSourceShape {
        filter_tokens: trim_lexed_commas(filter_tokens),
        controller: DamageSourceControllerKind::None,
        trailing_filter_tokens: &[],
    })
}

fn parse_minimum_red_noncombat_damage_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "if",
        "a",
        "red",
        "source",
        "you",
        "control",
        "would",
        "deal",
        "an",
        "amount",
        "of",
        "noncombat",
        "damage",
        "less",
        "than",
    ])
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek((
            primitives::kw("power"),
            primitives::phrase(&["to", "an", "opponent"]),
        )),
    )
    .void()
    .parse_next(input)?;
    primitives::kw("power").parse_next(input)?;
    primitives::phrase(&["to", "an", "opponent"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["that", "source", "deals", "damage", "equal", "to"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek((primitives::kw("power"), primitives::kw("instead"))),
    )
    .void()
    .parse_next(input)?;
    primitives::phrase(&["power", "instead"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

fn parse_prevent_damage_to_you_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PreventDamageToYouSpec<'a>> {
    primitives::kw("if").parse_next(input)?;
    let source_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&[
            "would", "deal", "damage", "to", "you",
        ])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["would", "deal", "damage", "to", "you"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("prevent").parse_next(input)?;
    let amount = leaf::parse_leaf_number_prefix_lexed
        .verify(|amount: &u32| *amount > 0)
        .parse_next(input)?;
    primitives::phrase(&["of", "that", "damage"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PreventDamageToYouSpec {
        source_tokens: trim_lexed_commas(source_tokens),
        amount,
    })
}

fn parse_damage_amount_tail_lexed<'a>(input: &mut LexStream<'a>) -> WResult<DamageAmountTail<'a>> {
    alt((
        (primitives::kw("instead"), primitives::sentence_end()).value(DamageAmountTail::Instead),
        (
            primitives::phrase(&["to", "that"]),
            repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("instead")))
                .map(|((), _)| ())
                .take(),
            primitives::kw("instead"),
            primitives::sentence_end(),
        )
            .map(|(_, target, _, _)| DamageAmountTail::ToThatTarget(trim_lexed_commas(target))),
    ))
    .parse_next(input)
}

fn parse_damage_redirect_controller_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DamageRedirectControllerSpec<'a>> {
    primitives::kw("if").parse_next(input)?;
    let source_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&[
            "would", "deal", "damage", "to", "you",
        ])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["would", "deal", "damage", "to", "you"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&[
        "it",
        "deals",
        "that",
        "damage",
        "to",
        "its",
        "controller",
        "instead",
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(DamageRedirectControllerSpec {
        source_tokens: trim_lexed_commas(source_tokens),
    })
}

fn parse_combat_maximum_tail_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CombatMaximumKind> {
    let kind = alt((
        (
            alt((primitives::kw("creature"), primitives::kw("creatures"))),
            primitives::phrase(&["can", "attack", "you", "each", "combat"]),
        )
            .value(CombatMaximumKind::AttackYou),
        (
            alt((primitives::kw("creature"), primitives::kw("creatures"))),
            primitives::phrase(&["can", "attack", "each", "combat"]),
        )
            .value(CombatMaximumKind::Attack),
        (
            alt((primitives::kw("creature"), primitives::kw("creatures"))),
            primitives::phrase(&["can", "block", "each", "combat"]),
        )
            .value(CombatMaximumKind::Block),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(kind)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_multiplier_and_redirect_shapes() {
        let tokens = lex_line(
            "If a source would deal damage to an opponent, it deals double that damage to that opponent instead.",
            0,
        )
        .unwrap();
        let spec = parse_damage_multiplier_tokens(&tokens).unwrap();
        assert_eq!(spec.factor, 2);
        assert!(!spec.combat_only);
        assert_eq!(spec.source.controller, DamageSourceControllerKind::None);

        let tokens = lex_line(
            "If a creature would deal damage to you, it deals that damage to its controller instead.",
            0,
        )
        .unwrap();
        assert!(parse_damage_redirect_controller_tokens(&tokens).is_some());
    }

    #[test]
    fn parses_additive_minimum_and_prevention_shapes() {
        let tokens = lex_line(
            "If a red source you control would deal damage to an opponent, it deals that much damage plus 2 to that opponent instead.",
            0,
        )
        .unwrap();
        let spec = parse_additive_damage_amount_tokens(&tokens).unwrap();
        assert_eq!(spec.delta, 2);
        assert_eq!(spec.source.controller, DamageSourceControllerKind::You);
        assert!(spec.repeated_target_tokens.is_some());

        let tokens = lex_line(
            "If a red source you control would deal an amount of noncombat damage less than this creature's power to an opponent, that source deals damage equal to this creature's power instead.",
            0,
        )
        .unwrap();
        assert!(parse_minimum_red_noncombat_damage_tokens(&tokens));

        let tokens = lex_line(
            "If a red source would deal damage to you, prevent 1 of that damage.",
            0,
        )
        .unwrap();
        let spec = parse_prevent_damage_to_you_tokens(&tokens).unwrap();
        assert_eq!(spec.amount, 1);
    }

    #[test]
    fn parses_combat_maximum_tail() {
        let tokens = lex_line("creature can attack you each combat", 0).unwrap();
        assert_eq!(
            parse_combat_maximum_tail_tokens(&tokens),
            Some(CombatMaximumKind::AttackYou)
        );
    }
}
