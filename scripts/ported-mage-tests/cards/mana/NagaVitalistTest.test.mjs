import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/NagaVitalistTest.java",
  "tests": [
    {
      "name": "nagaVitalist_GiftOfParadiseCanAnyColor",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gift of Paradise",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gift of Paradise",
          "target": "Swamp"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{W}{W}",
            "{U}{U}",
            "{B}{B}",
            "{R}{R}",
            "{G}{G}"
          ],
          "exact": true
        }
      ]
    },
    {
      "name": "nagaVitalist_GiftOfParadisesLandCanGiveAnyColorToNaga_SameStep1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gift of Paradise",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Naga Vitalist",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gift of Paradise",
          "target": "Swamp"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gift of Paradise",
          "count": 1,
          "objectTarget": "Swamp"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gift of Paradise",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Gift of Paradise",
          "target": "Swamp",
          "expected": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "{R}"
          ]
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Add one mana of any type",
          "count": 1,
          "source": "Naga Vitalist"
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "R",
          "amount": 1
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Naga Vitalist",
          "tapped": true,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "player": 0,
          "tapped": true,
          "count": 3
        },
        {
          "op": "assertTapped",
          "name": "Gift of Paradise",
          "tapped": false,
          "player": 0
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Swamp",
          "tapped": false
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Naga Vitalist",
          "tapped": true
        }
      ]
    },
    {
      "name": "nagaVitalist_GiftOfParadisesLandCanGiveAnyColorToNaga_DiffStep1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gift of Paradise",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Naga Vitalist",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gift of Paradise",
          "target": "Swamp"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gift of Paradise",
          "count": 1,
          "objectTarget": "Swamp"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gift of Paradise",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Gift of Paradise",
          "target": "Swamp",
          "expected": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "{R}"
          ]
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Add one mana of any type",
          "count": 1,
          "source": "Naga Vitalist"
        },
        {
          "op": "assertManaPool",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "R",
          "amount": 1
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Naga Vitalist",
          "tapped": true,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "player": 0,
          "tapped": true,
          "count": 3
        },
        {
          "op": "assertTapped",
          "name": "Gift of Paradise",
          "tapped": false,
          "player": 0
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Swamp",
          "tapped": false
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Naga Vitalist",
          "tapped": true
        }
      ]
    },
    {
      "name": "nagaVitalist_GiftOfParadisesLandCanGiveAnyColorToNaga_SameStep3",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gift of Paradise",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Naga Vitalist",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gift of Paradise",
          "target": "Swamp"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gift of Paradise",
          "count": 1,
          "objectTarget": "Swamp"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gift of Paradise",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Gift of Paradise",
          "target": "Swamp",
          "expected": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "{R}"
          ]
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Add one mana of any type",
          "count": 1,
          "source": "Naga Vitalist"
        },
        {
          "op": "assertManaPool",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "R",
          "amount": 1
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Naga Vitalist",
          "tapped": true,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "player": 0,
          "tapped": true,
          "count": 3
        },
        {
          "op": "assertTapped",
          "name": "Gift of Paradise",
          "tapped": false,
          "player": 0
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Swamp",
          "tapped": false
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Naga Vitalist",
          "tapped": true
        }
      ]
    },
    {
      "name": "nagaVitalist_GiftOfParadisesLandCanGiveAnyColorToNaga_DiffStep2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gift of Paradise",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Naga Vitalist",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1,
          "source": "Forest"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gift of Paradise",
          "target": "Swamp"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gift of Paradise",
          "count": 1,
          "objectTarget": "Swamp"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gift of Paradise",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Gift of Paradise",
          "target": "Swamp",
          "expected": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "{R}"
          ]
        },
        {
          "op": "activateManaAbility",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Add one mana of any type",
          "count": 1,
          "source": "Naga Vitalist"
        },
        {
          "op": "assertManaPool",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "R",
          "amount": 1
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Naga Vitalist",
          "tapped": true,
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "player": 0,
          "tapped": true,
          "count": 3
        },
        {
          "op": "assertTapped",
          "name": "Gift of Paradise",
          "tapped": false,
          "player": 0
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Swamp",
          "tapped": false
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Naga Vitalist",
          "tapped": true
        }
      ]
    },
    {
      "name": "nagaVitalist_InteractionGiftOfParadise",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Naga Vitalist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gift of Paradise",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gift of Paradise",
          "target": "Forest"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gift of Paradise",
          "count": 1,
          "objectTarget": "Forest"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gift of Paradise",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Gift of Paradise",
          "target": "Forest",
          "expected": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "{R}"
          ]
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Add one mana of any type",
          "count": 1,
          "source": "Naga Vitalist"
        },
        {
          "op": "assertManaPool",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "R",
          "amount": 1
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Naga Vitalist",
          "tapped": true,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertTapped",
          "name": "Naga Vitalist",
          "tapped": true,
          "player": 0
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Gift of Paradise",
          "tapped": false
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "player": 0,
          "tapped": false,
          "count": 3
        }
      ]
    }
  ]
});
