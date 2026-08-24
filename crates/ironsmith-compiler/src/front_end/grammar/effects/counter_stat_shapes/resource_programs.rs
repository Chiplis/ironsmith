use super::*;

pub fn parse_half_life(words: &[&str]) -> Option<HalfLifeShape> {
    if parse_prefix(words, &[&["half"]]).is_none()
        || find_word(words, &["life"]).is_none()
        || find_word(words, &["lost"]).is_some()
    {
        return None;
    }
    Some(HalfLifeShape {
        rounded_down: find_phrase(words, &[&["rounded", "down"]]).is_some(),
    })
}
