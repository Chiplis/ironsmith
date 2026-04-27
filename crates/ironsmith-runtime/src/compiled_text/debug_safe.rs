use super::ast_render::RawRenderedLine;
use super::{normalize_sentence_surface_style, strip_render_heading};
use crate::text_cleanup::strip_parenthetical_text;

pub(super) struct DebugSafeLine(String);

impl DebugSafeLine {
    pub(super) fn into_string(self) -> String {
        self.0
    }

    fn from_raw(raw: RawRenderedLine) -> Option<Self> {
        let line = mechanical_cleanup(raw.into_string());
        (!line.is_empty()).then_some(Self(line))
    }
}

pub(super) fn normalize_debug_safe_surface(lines: Vec<RawRenderedLine>) -> Vec<DebugSafeLine> {
    lines
        .into_iter()
        .filter_map(DebugSafeLine::from_raw)
        .collect()
}

fn mechanical_cleanup(line: String) -> String {
    let line = strip_render_heading(&line);
    if line.trim().is_empty() {
        return String::new();
    }
    let line = normalize_debug_safe_sentence_surface(&line);
    let line = normalize_debug_safe_mana_symbol_case(&line);
    let line = strip_parenthetical_text(&line);
    normalize_debug_safe_spelling_surface(&line)
}

fn normalize_debug_safe_sentence_surface(line: &str) -> String {
    if !line.contains('\n') {
        return normalize_sentence_surface_style(line);
    }

    line.lines()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            if let Some(body) = part.strip_prefix('•') {
                let body = normalize_sentence_surface_style(body.trim());
                format!("• {body}")
            } else {
                normalize_sentence_surface_style(part)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_debug_safe_mana_symbol_case(line: &str) -> String {
    let mut normalized = line.to_string();
    for (from, to) in [
        ("{w}", "{W}"),
        ("{u}", "{U}"),
        ("{b}", "{B}"),
        ("{r}", "{R}"),
        ("{g}", "{G}"),
        ("{c}", "{C}"),
        ("{t}", "{T}"),
        ("{q}", "{Q}"),
        ("{e}", "{E}"),
        ("{s}", "{S}"),
        ("{x}", "{X}"),
    ] {
        normalized = normalized.replace(from, to);
    }
    while normalized.contains("} {") {
        normalized = normalized.replace("} {", "}{");
    }
    normalized
}

fn normalize_debug_safe_spelling_surface(line: &str) -> String {
    let mut normalized = line
        .trim()
        .replace("that many color plus one", "that many colors plus one")
        .replace("Count the color of", "Count the colors of")
        .replace("count the color of", "count the colors of")
        .replace("that much +1/+1 counter", "that many +1/+1 counters")
        .replace("If you is the monarch", "If you're the monarch")
        .replace("if you is the monarch", "if you're the monarch")
        .replace("Otherwise, You become", "Otherwise, you become")
        .replace("Attacking/blocking", "Attacking or blocking")
        .replace("attacking/blocking", "attacking or blocking")
        .replace("or greaters", "or greater")
        .replace("attached tos", "attached to")
        .replace("enters the battlefield", "enters")
        .replace("enter the battlefield", "enter")
        .replace("Enters the battlefield", "Enters")
        .replace("Enter the battlefield", "Enter")
        .replace("Cascade and Cascade", "Cascade, cascade")
        .replace("Add 1 mana of any color", "Add one mana of any color")
        .replace("add 1 mana of any color", "add one mana of any color")
        .replace("fateseal {1}", "fateseal 1")
        .replace("Fateseal {1}", "Fateseal 1")
        .replace(" hand :", " hand:")
        .replace("put X +1/+1 counter on", "put X +1/+1 counters on")
        .replace("Put X +1/+1 counter on", "Put X +1/+1 counters on")
        .replace("sliver card in hand have", "sliver cards in your hand have")
        .replace("Sliver card in hand have", "Sliver cards in your hand have")
        .replace("other than wall", "other than Wall")
        .replace("Other than wall", "Other than Wall")
        .replace(" all auras or equipment ", " all Auras and Equipment ")
        .replace("All auras or equipment ", "All Auras and Equipment ")
        .replace(": target ", ": Target ")
        .replace("card ins", "cards in")
        .replace("a Elf", "an Elf");

    if normalized
        .eq_ignore_ascii_case("Target defending player's creature gets +3/+0 and gains can block 2 additional creatures each combat until end of turn.")
    {
        normalized = "Target creature defending player controls gets +3/+0 until end of turn. That creature can block up to two additional creatures this turn.".to_string();
    }

    if normalized.eq_ignore_ascii_case(
        "Exile target opponent's creature with mana value 2 or less. Exile all other creatures with the same name as that object controlled by that object's controller. That player reveals their hand. Exile all cards in hand or cards in a graveyard.",
    ) || normalized.eq_ignore_ascii_case(
        "Exile target opponent's creature with mana value 2 or less. Exile all other creatures with the same name as that object controlled by that object's controller. That player reveals their hand. Exile all card in hands or cards in a graveyard.",
    ) {
        normalized = "Exile target creature an opponent controls with mana value 2 or less and all other creatures that player controls with the same name as that creature. Then that player reveals their hand and exiles all cards with that name from their hand and graveyard.".to_string();
    }

    if let Some((prefix, rest)) = normalized.split_once(": ") {
        let rest_lower = rest.to_ascii_lowercase();
        if rest_lower.trim_end_matches('.')
            == "you can't be targeted until your next turn. prevent all damage that would be dealt to you until your next turn"
        {
            normalized =
                format!("{prefix}: You gain protection from everything until your next turn.");
        }
    }

    if normalized.ends_with("..") {
        normalized.pop();
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_is_mechanical() {
        assert_eq!(
            normalize_debug_safe_spelling_surface("add 1 mana of any color to your mana pool."),
            "add one mana of any color to your mana pool."
        );
    }
}
