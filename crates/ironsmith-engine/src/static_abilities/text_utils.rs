pub(crate) fn join_with_and<T: AsRef<str>>(items: &[T]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].as_ref().to_string(),
        2 => format!("{} and {}", items[0].as_ref(), items[1].as_ref()),
        _ => {
            let mut out = items[..items.len() - 1]
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(", and ");
            out.push_str(items.last().map(AsRef::as_ref).unwrap_or_default());
            out
        }
    }
}

pub(crate) fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

pub(crate) fn number_word_u32(n: u32) -> Option<String> {
    ironsmith_core::cardinal_word(n)
}
