//! Gonti, Night Minister: the damaging creature's controller privately looks
//! at the damaged opponent's top card, exiles it face down (keeping that
//! private view), and may play it with any-type mana. Source-authored, UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::LookAtTopCardsEffect;
use ironsmith::target::PlayerFilter;

const GONTI: &str = "Mana cost: {2}{B}{B}\nType: Legendary Creature — Aetherborn Rogue\nPower/Toughness: 2/3\nWhenever a player casts a spell they don't own, that player creates a Treasure token.\nWhenever a creature deals combat damage to one of your opponents, its controller looks at the top card of that opponent's library and exiles it face down. They may play that card for as long as it remains exiled. Mana of any type can be spent to cast a spell this way.";

#[test]
fn gonti_look_exile_and_permission_belong_to_the_damaging_creatures_controller() {
    for definition in support::definitions("Gonti, Night Minister", GONTI) {
        let looks = support::find_all::<LookAtTopCardsEffect>(&definition);
        let look = looks.first().unwrap_or_else(|| panic!("{:#?}", definition.abilities));
        assert_ne!(look.viewer, PlayerFilter::You, "not the ability's controller: {look:#?}");
        assert_ne!(look.viewer, look.player, "the viewer isn't the library owner");
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("face_down: true"), "{debug}");
        assert!(debug.contains("ForAsLongAsExiled"), "{debug}");
        assert!(debug.contains("AnyType"), "{debug}");
        assert!(debug.contains("triggering_source"), "{debug}");
    }
}
