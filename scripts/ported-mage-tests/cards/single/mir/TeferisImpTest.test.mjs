import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mir/TeferisImpTest.java",
  "tests": [
    {
      "name": "test_Phasing_triggers",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Teferi's Imp",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Teferi's Imp"
        },
        {
          "op": "assertHandCount",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "UPKEEP",
          "player": null
        },
        {
          "op": "assertHandCount",
          "turn": 3,
          "phase": "UPKEEP",
          "player": 0,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 4,
          "phase": "END_TURN",
          "player": 0,
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "UPKEEP",
          "player": null
        },
        {
          "op": "assertHandCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 0,
          "count": 3,
          "seatIndependent": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    }
  ]
});
