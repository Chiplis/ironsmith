import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/replacement/AcademyManufactorTest.java",
  "tests": [
    {
      "name": "testAcademyManufactor",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Academy Manufactor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thraben Inspector",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thraben Inspector"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Academy Manufactor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Thraben Inspector",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Clue Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Food Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 1
        }
      ]
    },
    {
      "name": "testMultipleReplacementEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Academy Manufactor",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Anointed Procession",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thraben Inspector",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thraben Inspector"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Academy Manufactor",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Anointed Procession",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Thraben Inspector",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Clue Token",
          "count": 6
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Food Token",
          "count": 6
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 6
        }
      ]
    },
    {
      "name": "testTokenLimit",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Academy Manufactor",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thraben Inspector",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thraben Inspector"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Academy Manufactor",
          "count": 6
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Thraben Inspector",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 508
        }
      ]
    },
    {
      "name": "testGingerbruteToken",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Academy Manufactor",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fractured Identity",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fractured Identity",
          "target": "Gingerbrute"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Tundra",
          "count": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Academy Manufactor",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gingerbrute",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Clue Token",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Food Token",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 3
        }
      ]
    }
  ]
});
