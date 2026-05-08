import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/exile/OblivionSowerTest.java",
  "tests": [
    {
      "name": "testPlayLandsFromExile",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Oblivion Sower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Canopy Vista",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Oblivion Sower"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Canopy Vista^Canopy Vista^Canopy Vista"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Oblivion Sower",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Oblivion Sower",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Canopy Vista",
          "count": 3
        },
        {
          "op": "assertTappedCount",
          "name": "Canopy Vista",
          "tapped": false,
          "count": 3
        }
      ]
    }
  ]
});
