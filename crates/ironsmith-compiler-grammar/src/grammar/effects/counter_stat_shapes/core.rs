use super::*;

pub(super) fn words_after_first<'a>(words: &'a [&'a str]) -> &'a [&'a str] {
    words.get(1..).unwrap_or_default()
}

pub(super) fn parse_possessive_stem(input: &mut &str) -> WResult<String> {
    let stem: &str = take_till(1.., |character: char| matches!(character, '\'' | '’' | '‘'))
        .parse_next(input)?;
    alt((literal("'s"), literal("’s"), literal("‘s"))).parse_next(input)?;
    eof.parse_next(input)?;
    let mut output = String::new();
    output.push_str(stem);
    let _: &str = rest.parse_next(input)?;
    Ok(output)
}
