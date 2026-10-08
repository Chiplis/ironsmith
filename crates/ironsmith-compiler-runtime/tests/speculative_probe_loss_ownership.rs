//! cf8 p01: complete frozen bodies whose only baseline defect was a
//! speculative suffix-recovery diagnostic leaked by an abandoned static probe
//! (fixed by the captured copular ownership probe and by keeping static-line
//! losses only for committed static readings). Every body must now compile
//! strictly and loss-free on both routes. Source-authored, deliberately unrun.
#[path = "p01_support/mod.rs"]
mod support;

const BODIES: &[&str] = &[
    "Contamination", "Damping Sphere", "Infernal Darkness", "Ritual of Subdual",
    "Diligent Farmhand", "Gemstone Caverns", "Leyline Axe", "Leyline of Abundance",
    "Leyline of Anticipation", "Leyline of Combustion", "Leyline of Hope",
    "Leyline of Lifeforce", "Leyline of Lightning", "Leyline of Punishment",
    "Leyline of Resonance", "Leyline of Sanctity", "Leyline of Singularity",
    "Leyline of Vitality", "Leyline of the Guildpact", "Leyline of the Meek",
    "Leyline of the Void", "Pardic Firecat", "Faceless One", "Quicksilver, Brash Blur",
    "The Prismatic Piper", "Serum Powder", "Scythecat Cub", "Oran-Rief Hydra",
    "Nantuko Blightcutter", "Embodiment of Fury", "Embodiment of Insight",
    "Enduring Renewal", "Sparring Dummy", "Baru, Wurmspeaker", "Belt of Giant Strength",
    "Sewer Crocodile", "Survey Mechan", "Life and Limb", "Shard of the Void Dragon",
    "Mob Verdict", "Solitary Camel", "Victor, Valgavoth's Seneschal", "Rose Room Treasurer",
    "The Prydwen, Steel Flagship", "Skybind", "Inalla, Archmage Ritualist", "Territory Culler",
    "Strength from the Fallen", "Venom Connoisseur", "Daxos's Torment", "Emeria Shepherd",
    "Springheart Nantuko", "Whiskervale Forerunner", "Lumengrid Drake", "Wall of Mourning",
    "Hawkeye, Young Avenger", "The Rollercrusher Ride", "Akoum Hellkite", "Impending Flux",
    "Calamitous Cave-In", "Hum of the Radix", "Haunting Imitation", "Collective Restraint",
    "Disorienting Choice", "Unexplained Absence", "Displaced Dinosaurs", "Gaze of Pain",
    "Be'lakor, the Dark Master", "Master Biomancer", "Elemental Expressionist",
    "Brenard, Ginger Sculptor", "Hofri Ghostforge", "Breathstealer's Crypt",
    "Thunderous Velocipede", "Icewind Stalwart", "Bloodspore Thrinax", "Galvanoth",
    "Rashmi, Eternities Crafter", "Juvenile Mist Dragon", "Sphere of Safety",
    "Strength-Testing Hammer", "Guardian of Tazeem", "Guul Draz Overseer", "Bring the Ending",
    "Bygone Marvels", "Belladonna Took", "Vito, Fanatic of Aclazotz",
    "Omnath, Locus of Creation", "Arahbo, Roar of the World", "Johan", "Nissa, Leyline Tamer",
    "Nissa, Resurgent Animist", "Rumor Gatherer",
];

#[test]
fn speculative_suffix_recovery_never_taints_a_committed_body() {
    assert_eq!(BODIES.len(), 93);
    for name in BODIES {
        for definition in support::definitions(name) {
            assert_eq!(definition.card.name, *name);
        }
    }
}

#[test]
fn opening_hand_pregame_bodies_keep_their_owner() {
    for name in ["Leyline of Sanctity", "Gemstone Caverns", "Quicksilver, Brash Blur"] {
        for definition in support::definitions(name) {
            let text = support::rendered(&definition);
            assert!(text.contains("opening hand"), "{name}: {text}");
            assert!(text.contains("begin the game with"), "{name}: {text}");
        }
    }
}
