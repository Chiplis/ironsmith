//! Canonical catalog alias route, authored only; UNVALIDATED/UNRUN.
use ironsmith_tools::{load_canonical_cards, compile_strict_snapshot_from_payload, ParseStatus};

#[test]
fn ordinary_and_combined_sakashima_entries_use_the_real_canonical_loader() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/enter_copy_exceptions.json.fixture");
    let cards = load_canonical_cards(path).unwrap();
    let ordinary_name = "Sakashima of a Thousand Faces";
    let alias_name = "Sakashima of a Thousand Faces // Sakashima of a Thousand Faces";
    let ordinary = cards.get(ordinary_name).expect("ordinary frozen catalog entry");
    let alias = cards.get(alias_name).expect("combined-name frozen catalog entry");
    assert_eq!(alias.name, alias_name);
    assert_eq!(alias.parse_name.as_deref(), Some(ordinary_name));
    assert_eq!(alias.oracle_text, ordinary.oracle_text);
    for payload in [ordinary, alias] {
        let snapshot = compile_strict_snapshot_from_payload(payload);
        assert_eq!(snapshot.card_name, payload.name);
        assert_eq!(snapshot.parse_status, ParseStatus::StrictCompiled, "{}: {:?}", payload.name, snapshot.parse_error);
        assert!(!snapshot.parse_lossy, "{}: {}", payload.name, snapshot.parse_loss_reasons);
        assert!(!snapshot.has_unimplemented);
        let rendered = snapshot.compiled_text.as_ref().unwrap().to_ascii_lowercase();
        assert!(rendered.contains("enter as a copy"), "{}: {rendered}", payload.name);
        assert!(rendered.contains("partner"));
        assert!(rendered.contains("legend rule"));
    }
}
