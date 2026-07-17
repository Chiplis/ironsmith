use super::shard_16::parse_oracle_card_definition;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn hand_reveal_and_same_name_search_cards_hide_internal_choices() {
    for name in ["Assembly Hall", "Infernal Tutor"] {
        let definition = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&definition)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            rendered.contains("reveal a") && rendered.contains("from your hand")
                || rendered.contains("in your hand"),
            "{name}: {rendered}"
        );
        assert!(
            rendered.contains("with the same name as that card"),
            "{name}: {rendered}"
        );
        assert!(
            !rendered.contains("choose a card, then reveal it"),
            "{name}: {rendered}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn target_antecedent_same_name_searches_render_inline() {
    for (name, reference) in [
        ("Mask of the Mimic", "target nontoken creature"),
        ("Pack Hunt", "target creature"),
    ] {
        let definition = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&definition)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            rendered.contains(&format!("same name as that creature"))
                || rendered.contains(&format!("same name as {reference}")),
            "{name}: {rendered}"
        );
        assert!(
            !rendered.contains(&format!("choose {reference}."))
                && !rendered.contains("same name as it"),
            "{name}: {rendered}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn targeted_library_exile_pipelines_hide_target_setup() {
    for name in ["Denying Wind", "Supreme Inquisitor"] {
        let definition = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&definition)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            rendered.contains("search target player's library")
                && rendered.contains("and exile them")
                && rendered.contains("then that player shuffles"),
            "{name}: {rendered}"
        );
        assert!(
            !rendered.contains("choose target player"),
            "{name}: {rendered}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn kicked_search_pipeline_has_one_shuffle_per_branch() {
    let definition = parse_oracle_card_definition("Sadistic Sacrament");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(
        !debug.contains("IteratedPlayer"),
        "targeted spell search must bind that player to its selected target: {debug}"
    );
    let rendered = unprocessed_compiled_lines(&definition)
        .join(" ")
        .to_ascii_lowercase();
    assert_eq!(rendered.matches("shuffles").count(), 2, "{rendered}");
    assert!(
        rendered.contains("if this spell was kicked, instead"),
        "{rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn dual_nature_keeps_typed_antecedent_and_creation_provenance() {
    let definition = parse_oracle_card_definition("Dual Nature");
    let rendered = unprocessed_compiled_lines(&definition)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("same name as that creature")
            && rendered.contains("tokens created with this enchantment"),
        "{rendered}"
    );
    assert!(!rendered.contains("enchantment tokens"), "{rendered}");
}
