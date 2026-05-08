import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mat/UrborgScavengersTests.java",
  "tests": [
    {
      "name": "getsHexproofHasteTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Urborg Scavengers",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Cragplate Baloth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Urborg Scavengers"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Cragplate Baloth"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbilities",
          "player": 0,
          "name": "Urborg Scavengers",
          "abilities": [
            "Haste",
            "Hexproof"
          ]
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Urborg Scavengers",
          "counter": "P1P1",
          "count": 1
        }
      ]
    }
  ]
});
