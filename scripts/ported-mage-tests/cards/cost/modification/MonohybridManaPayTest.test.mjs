import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/modification/MonohybridManaPayTest.java",
  "tests": [
    {
      "name": "test_PaySimpleMana_Manually",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
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
          "ability": "{T}: Add {B}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Green"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Black"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "test_PayMonohybridMana_ColorPart_SameUnlockOrder",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reaper King",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
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
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {U}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {R}",
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
          "name": "Reaper King"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "White"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blue"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Black"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Red"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Green"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Reaper King",
          "count": 1
        }
      ]
    },
    {
      "name": "test_PayMonohybridMana_ColorPart_AnyUnlockOrder",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reaper King",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
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
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Add {W}",
          "source": "Plains",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Add {U}",
          "source": "Island",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Add {B}",
          "source": "Swamp",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Add {R}",
          "source": "Mountain",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Add {G}",
          "source": "Forest",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "W",
          "amount": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "U",
          "amount": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "B",
          "amount": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "R",
          "amount": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 1 of Reaper King",
          "choices": [
            "{W} (White mana)"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 2 of Reaper King",
          "choices": [
            "{U} (Blue mana)"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 3 of Reaper King",
          "choices": [
            "{B} (Black mana)"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 4 of Reaper King",
          "choices": [
            "{R} (Red mana)"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 5 of Reaper King",
          "choices": [
            "{G} (Green mana)"
          ]
        },
        {
          "op": "castSpell",
          "name": "Reaper King",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Reaper King",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRG",
          "amount": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Reaper King",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "test_PayMonohybridMana_GenericPart_AnyUnlockOrder",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reaper King",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
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
          "name": "Forest",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Add {U}",
          "source": "Island",
          "count": 5
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Add {B}",
          "source": "Swamp",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Add {G}",
          "source": "Forest",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "U",
          "amount": 5
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "B",
          "amount": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 1 of Reaper King",
          "choices": [
            "{2} (2 generic mana)"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 2 of Reaper King",
          "choices": [
            "{U} (Blue mana)"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 3 of Reaper King",
          "choices": [
            "{B} (Black mana)"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 4 of Reaper King",
          "choices": [
            "{2} (2 generic mana)"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 5 of Reaper King",
          "choices": [
            "{G} (Green mana)"
          ]
        },
        {
          "op": "castSpell",
          "name": "Reaper King",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Reaper King",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRG",
          "amount": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Reaper King",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    }
  ]
});
