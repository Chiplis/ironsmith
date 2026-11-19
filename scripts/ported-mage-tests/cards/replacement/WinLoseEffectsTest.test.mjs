import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/replacement/WinLoseEffectsTest.java",
  "tests": [
    {
      "name": "testPlatinumAngelAndLaboratoryManiac",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Platinum Angel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Laboratory Maniac",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thought Reflection",
          "count": 4
        },
        {
          "op": "setStopAt",
          "turn": 40,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerState",
          "player": 0,
          "property": "hasWon",
          "expected": true
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "count": 0
        }
      ]
    },
    {
      "name": "testPlatinumAngelBouncedWithEmptyLibrary",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Platinum Angel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Set Adrift",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 6
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Set Adrift"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Set Adrift",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Platinum Angel",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Platinum Angel",
          "count": 1
        },
        {
          "op": "assertPlayerState",
          "player": 0,
          "property": "hasLost",
          "expected": false
        }
      ]
    },
    {
      "name": "testAngelsGrace",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Angel's Grace",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bandage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Laboratory Maniac",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertLibraryCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Angel's Grace"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Angel's Grace"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Angel's Grace",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertResult",
          "player": 0,
          "result": "ONGOING"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Bandage",
          "target": "Laboratory Maniac"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Bandage",
          "objectTarget": "Laboratory Maniac"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertResult",
          "player": 0,
          "result": "WON"
        },
        {
          "op": "assertResult",
          "player": 1,
          "result": "LOST"
        },
        {
          "op": "assertLife",
          "life": 20,
          "player": 0,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "life": 20,
          "player": 1,
          "seatIndependent": true
        }
      ]
    },
    {
      "name": "testAngelsGrace2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Angel's Grace",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ad Nauseam",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Laboratory Maniac",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Serum Visions",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Bogardan Hellkite",
          "count": 3
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertLibraryCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 4,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Angel's Grace"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Angel's Grace"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Angel's Grace",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertResult",
          "player": 0,
          "result": "ONGOING"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ad Nauseam"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Ad Nauseam"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "life": -5,
          "seatIndependent": true
        },
        {
          "op": "assertResult",
          "player": 0,
          "result": "ONGOING"
        },
        {
          "op": "assertLibraryCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Serum Visions",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bogardan Hellkite",
          "count": 3,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ad Nauseam",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Serum Visions"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Serum Visions"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertResult",
          "player": 0,
          "result": "WON"
        },
        {
          "op": "assertResult",
          "player": 1,
          "result": "LOST"
        },
        {
          "op": "assertLife",
          "life": -5,
          "player": 0,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "life": 20,
          "player": 1,
          "seatIndependent": true
        }
      ]
    }
  ]
});
