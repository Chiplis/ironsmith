#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn equipment_search_put_attach_shuffle_cards_render_exactly() {
    for (name, oracle) in [
        (
            "Stonehewer Giant",
            "Vigilance\n{1}{W}, {T}: Search your library for an Equipment card, put it onto the battlefield, attach it to a creature you control, then shuffle.",
        ),
        (
            "Quest for the Holy Relic",
            "Whenever you cast a creature spell, you may put a quest counter on this enchantment.\nRemove five quest counters from this enchantment and sacrifice it: Search your library for an Equipment card, put it onto the battlefield, attach it to a creature you control, then shuffle.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let compiled = canonical_compiled_lines(&definition).join("\n");
        let debug = format!("{definition:#?}");

        assert_eq!(compiled, oracle, "{name}: {debug}");
        assert!(
            debug.contains("ChooseObjectsEffect")
                && debug.contains("MoveToZoneEffect")
                && debug.contains("AttachObjectsEffect")
                && debug.contains("ShuffleLibraryEffect")
                && debug.contains("searched_multi_zone")
                && debug.contains("attachment_target_0"),
            "{name}: {debug}"
        );
    }
}
