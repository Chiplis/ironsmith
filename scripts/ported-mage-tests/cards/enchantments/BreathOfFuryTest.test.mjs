import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/enchantments/BreathOfFuryTest.java",
  "tests": [
    {
      "name": "testMoveEnchantment",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Breath of Fury",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Breath of Fury",
          "target": "Silvercoat Lion"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Pillarfield Ox",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Pillarfield Ox"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Pillarfield Ox",
          "tapped": false,
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Breath of Fury",
          "count": 1
        }
      ]
    }
  ]
});
