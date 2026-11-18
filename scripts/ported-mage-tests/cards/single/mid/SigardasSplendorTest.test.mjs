import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mid/SigardasSplendorTest.java",
  "tests": [
    {
      "name": "sigardasSplendorTestBasic",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sigarda's Splendor",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sigarda's Splendor"
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "life": 20,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 3,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sigarda's Splendor"
        },
        {
          "op": "assertHandCount",
          "turn": 3,
          "phase": "END_TURN",
          "player": 0,
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 3,
          "phase": "END_TURN",
          "player": 0,
          "life": 21,
          "seatIndependent": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "At the beginning of your upkeep"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sigarda's Splendor",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 5
        }
      ]
    },
    {
      "name": "sigardasSplendorTestDamaged",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sigarda's Splendor",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Scorching Spear",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sigarda's Splendor"
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "life": 20,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Scorching Spear",
          "target": 0
        },
        {
          "op": "assertLife",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "life": 19,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sigarda's Splendor"
        },
        {
          "op": "assertHandCount",
          "turn": 3,
          "phase": "END_TURN",
          "player": 0,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 3,
          "phase": "END_TURN",
          "player": 0,
          "life": 20,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Scorching Spear",
          "target": 0
        },
        {
          "op": "assertLife",
          "turn": 4,
          "phase": "END_TURN",
          "player": 0,
          "life": 19,
          "seatIndependent": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "At the beginning of your upkeep"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sigarda's Splendor",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        }
      ]
    }
  ]
});
