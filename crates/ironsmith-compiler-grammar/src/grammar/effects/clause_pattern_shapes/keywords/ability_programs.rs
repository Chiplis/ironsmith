use super::*;

pub(super) fn parse_keyword_mechanic_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordMechanicShape<'a>> {
    opt(primitives::kw("then")).parse_next(input)?;
    opt(primitives::kw("you")).parse_next(input)?;
    alt((
        parse_amass,
        parse_forage,
        parse_harness,
        parse_roll_d6,
        parse_odd_even_result,
        parse_phase,
        parse_open_attraction,
        alt((
            parse_behold,
            parse_blight,
            parse_manifest_dread,
            parse_manifest_from_hand,
            alt((
                parse_cloak_top_you,
                parse_manifest_top_you,
                parse_cloak_top_that_player,
                parse_manifest_top_that_player,
            )),
            parse_populate,
            parse_meld,
            alt((
                parse_numeric_keyword,
                parse_fateseal,
                parse_discover,
                parse_explore,
                parse_endure,
            )),
        )),
    ))
    .parse_next(input)
}
