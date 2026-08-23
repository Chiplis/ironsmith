const SENTENCE_HELPER_TAG_PREFIX: &str = "__sentence_helper__";
const SENTENCE_HELPER_TAG_PREFIX_LEGACY: &str = "__sentence_helper_";

pub fn is_sentence_helper_tag(tag: &str, prefix: &str) -> bool {
    let Some(rest) = tag
        .strip_prefix(SENTENCE_HELPER_TAG_PREFIX)
        .or_else(|| tag.strip_prefix(SENTENCE_HELPER_TAG_PREFIX_LEGACY))
    else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(prefix) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix("_l") else {
        return false;
    };
    let mut parts = rest.split("_s");
    let Some(line) = parts.next() else {
        return false;
    };
    let Some(rest) = parts.next() else {
        return false;
    };
    let mut parts = rest.split("_e");
    let Some(start) = parts.next() else {
        return false;
    };
    let Some(end) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && line.parse::<usize>().is_ok()
        && start.parse::<usize>().is_ok()
        && end.parse::<usize>().is_ok()
}
