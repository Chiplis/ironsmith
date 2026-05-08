import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tor/HypnoxTest.java",
  "tests": [
    {
      "name": "testExileAndReturn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hypnox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Watchwolf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bloodthrone Vampire",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hypnox"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Hypnox",
          "power": 8,
          "toughness": 8
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Watchwolf",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 0,
          "ability": "Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hypnox"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Hypnox",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Watchwolf",
          "count": 1
        }
      ]
    }
  ]
});
