import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ncc/PhabineBosssConfidantTest.java",
  "tests": [
    {
      "name": "boostWorks",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phabine, Boss's Confidant",
          "count": 1
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
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Citizen Token",
          "count": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Citizen Token",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
