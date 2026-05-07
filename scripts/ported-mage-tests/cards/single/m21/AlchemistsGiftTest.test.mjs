import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m21/AlchemistsGiftTest.java",
  "tests": [
    {
      "name": "giveDeathTouch",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Alchemist's Gift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Adherent of Hope",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Alchemist's Gift",
          "target": "Adherent of Hope"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "deathtouch"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertAbility",
          "player": 0,
          "name": "Adherent of Hope",
          "ability": "Deathtouch",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Adherent of Hope",
          "ability": "Lifelink",
          "expected": false
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Adherent of Hope",
          "power": 3,
          "toughness": 2
        }
      ]
    },
    {
      "name": "giveLifelink",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Alchemist's Gift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Adherent of Hope",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Alchemist's Gift",
          "target": "Adherent of Hope"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "lifelink"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertAbility",
          "player": 0,
          "name": "Adherent of Hope",
          "ability": "Lifelink",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Adherent of Hope",
          "ability": "Deathtouch",
          "expected": false
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Adherent of Hope",
          "power": 3,
          "toughness": 2
        }
      ]
    }
  ]
});
