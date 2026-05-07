import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/iko/GenesisUltimatumTest.java",
  "tests": [
    {
      "name": "test_Playable",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Genesis Ultimatum",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Alpha Tyrranax",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Kitesail Corsair",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Riverglide Pathway",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Genesis Ultimatum"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears^Kitesail Corsair^Riverglide Pathway"
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Genesis Ultimatum",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kitesail Corsair",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Riverglide Pathway",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Alpha Tyrranax",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "count": 0,
          "name": 0
        }
      ]
    }
  ]
});
