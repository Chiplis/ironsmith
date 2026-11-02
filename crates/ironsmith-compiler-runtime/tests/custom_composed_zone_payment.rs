//! Generic composed zone permissions retain scoped payment attribution.
use ironsmith_compiler_runtime::compile_to_runtime_definition;
#[test]
fn independent_zone_price_can_receive_custom_payment_identity_without_renaming_official_costs() {
 let text="Mana cost: {3}{G}\nType: Creature\nPower/Toughness: 2/2\nYou may cast this spell from your hand for {G} rather than pay its mana cost. If you cast this spell this way, it enters tapped.";
 let mut definition=compile_to_runtime_definition("Independent zone price",text,false).unwrap();
 assert_eq!(definition.alternative_casts.len(),1);
 assert_eq!(definition.alternative_casts[0].name(),"Parsed alternative cost");
 assert!(definition.attribute_custom_keyword_payment("journey"));
 assert_eq!(definition.alternative_casts[0].name(),"journey");
 assert!(definition.alternative_casts[0].battlefield_instructions().enters_tapped);
 let mut official=compile_to_runtime_definition("Native price","Mana cost: {3}{G}\nType: Creature\nPower/Toughness: 2/2\nDash {G}",false).unwrap();
 assert!(!official.attribute_custom_keyword_payment("journey"));
 assert_eq!(official.alternative_casts[0].name(),"Dash");
}
