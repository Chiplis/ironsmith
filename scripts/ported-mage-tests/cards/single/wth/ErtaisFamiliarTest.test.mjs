import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/wth/ErtaisFamiliarTest.java",
  "tests": [
    {
      "name": "test_phaseout",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ertai's Familiar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ertai's Familiar"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ertai's Familiar",
          "count": 0
        }
      ]
    },
    {
      "name": "test_activate_cant_phaseout",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ertai's Familiar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ertai's Familiar"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{U}:"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ertai's Familiar",
          "count": 1
        }
      ]
    },
    {
      "name": "test_activate_cant_phaseout_other_effect",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ertai's Familiar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Slip Out the Back",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ertai's Familiar"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "{U}:"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Slip Out the Back",
          "target": "Ertai's Familiar"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ertai's Familiar",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ertai's Familiar",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "test_activate_cant_phaseout_then_blink",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ertai's Familiar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blur",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ertai's Familiar"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "{U}:"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Blur",
          "target": "Ertai's Familiar"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": "3 * 2 + 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ertai's Familiar",
          "count": 0
        }
      ]
    }
  ]
});
