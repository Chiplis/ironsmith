import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/enchantments/AuraMovingTest.java",
  "tests": [
    {
      "name": "testOneAttackerDamage",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Alexi's Cloak",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bruna, Light of Alabaster",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Unholy Strength",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Hostile Realm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Alexi's Cloak",
          "target": "Bruna, Light of Alabaster"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Bruna, Light of Alabaster",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Yes"
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Alexi's Cloak",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Unholy Strength",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Hostile Realm",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Hostile Realm",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 13
        }
      ]
    }
  ]
});
