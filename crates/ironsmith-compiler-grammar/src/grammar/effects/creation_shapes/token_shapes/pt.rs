use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtComponent {
    Fixed(i32),
    X,
    Star,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtSurface {
    pub power: PtComponent,
    pub toughness: PtComponent,
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

pub fn parse_pt_word(word: &str) -> Option<PtSurface> {
    let normalized = word.to_ascii_lowercase();
    crate::grammar::primitives::probe_shape(parse_pt.parse(&normalized))
}

pub fn parse_unsigned_pt_word(word: &str) -> Option<(i32, i32)> {
    crate::grammar::primitives::probe_shape(parse_unsigned_pt.parse(word))
}

pub fn first_pt_word(words: &[&str]) -> Option<(usize, PtSurface)> {
    for (idx, word) in words.iter().enumerate() {
        if let Some(parsed) = parse_pt_word(word) {
            return Some((idx, parsed));
        }
    }
    None
}
