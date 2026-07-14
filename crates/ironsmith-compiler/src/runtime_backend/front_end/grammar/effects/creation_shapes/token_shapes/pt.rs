use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PtComponent {
    Fixed(i32),
    X,
    Star,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PtSurface {
    pub(crate) power: PtComponent,
    pub(crate) toughness: PtComponent,
}

fn parse_pt_component(input: &mut &str) -> WResult<PtComponent> {
    alt((
        "x".value(PtComponent::X),
        "*".value(PtComponent::Star),
        dec_int.map(PtComponent::Fixed),
    ))
    .parse_next(input)
}

fn parse_pt(input: &mut &str) -> WResult<PtSurface> {
    separated_pair(parse_pt_component, '/', parse_pt_component)
        .map(|(power, toughness)| PtSurface { power, toughness })
        .parse_next(input)
}

fn parse_unsigned_pt(input: &mut &str) -> WResult<(i32, i32)> {
    separated_pair(
        digit1.try_map(|digits: &str| digits.parse::<i32>()),
        '/',
        digit1.try_map(|digits: &str| digits.parse::<i32>()),
    )
    .parse_next(input)
}

pub(crate) fn parse_pt_word(word: &str) -> Option<PtSurface> {
    parse_pt.parse(word).ok()
}

pub(crate) fn parse_unsigned_pt_word(word: &str) -> Option<(i32, i32)> {
    parse_unsigned_pt.parse(word).ok()
}

pub(crate) fn first_pt_word(words: &[&str]) -> Option<(usize, PtSurface)> {
    for (idx, word) in words.iter().enumerate() {
        if let Some(parsed) = parse_pt_word(word) {
            return Some((idx, parsed));
        }
    }
    None
}
