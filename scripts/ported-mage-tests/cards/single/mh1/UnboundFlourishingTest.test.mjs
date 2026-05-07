import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh1/UnboundFlourishingTest.java",
  "tests": [
    {
      "name": "testCastWanShiTong",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Wan Shi Tong, Librarian",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unbound Flourishing",
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
          "name": "Wan Shi Tong, Librarian"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wan Shi Tong, Librarian",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Wan Shi Tong, Librarian",
          "power": 5,
          "toughness": 5
        }
      ]
    },
    {
      "name": "testDiscoverWanShiTong",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Wan Shi Tong, Librarian",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unbound Flourishing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Geological Appraiser",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Geological Appraiser"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Geological Appraiser",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wan Shi Tong, Librarian",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Wan Shi Tong, Librarian",
          "power": 1,
          "toughness": 1
        }
      ]
    }
  ]
});
