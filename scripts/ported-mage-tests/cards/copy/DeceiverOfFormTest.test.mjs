import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/DeceiverOfFormTest.java",
  "tests": [
    {
      "name": "testCopyEndsEndOfTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Affa Protector",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Deceiver of Form",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 3
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Deceiver of Form",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Affa Protector",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Affa Protector",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Affa Protector",
          "defender": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Deceiver of Form",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 9
        }
      ]
    }
  ]
});
