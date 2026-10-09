//! cf8 p10: "That creature's owner can't cast spells with the same name as
//! that creature until your next turn." Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effect::{Restriction, Until};
use ironsmith::effects::CantEffect;
use ironsmith::target::PlayerFilter;

const REFLECTOR_MAGE: &str = "Mana cost: {1}{W}{U}\nType: Creature — Human Wizard\nPower/Toughness: 2/3\nWhen this creature enters, return target creature an opponent controls to its owner's hand. That creature's owner can't cast spells with the same name as that creature until your next turn.";

#[test]
fn reflector_mage_binds_owner_and_name_to_the_returned_creature() {
    for definition in support::definitions("Reflector Mage", REFLECTOR_MAGE) {
        let [cant] = support::find_all::<CantEffect>(&definition)
            .try_into()
            .expect("one cast restriction");
        assert_eq!(cant.duration, Until::YourNextTurn);
        let Restriction::CastSpellsMatching(player, spells) = &cant.restriction else {
            panic!("{:?}", cant.restriction);
        };
        assert!(matches!(player, PlayerFilter::OwnerOf(_)), "{player:?}");
        assert_eq!(spells.tagged_constraints.len(), 1);
        assert_eq!(
            spells.tagged_constraints[0].relation,
            ironsmith::filter::TaggedOpbjectRelation::SameNameAsTagged
        );
    }
}
