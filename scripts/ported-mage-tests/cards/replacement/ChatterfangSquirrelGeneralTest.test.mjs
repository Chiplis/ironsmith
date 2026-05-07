import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/replacement/ChatterfangSquirrelGeneralTest.java",
  "tests": [
    {
      "name": "testChatterfang",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chatterfang, Squirrel General",
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
          "zone": "HAND",
          "player": 0,
          "name": "Raise the Alarm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Raise the Alarm"
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
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Chatterfang, Squirrel General",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldier Token",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 2
        }
      ]
    },
    {
      "name": "testChatterfangOpponent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Chatterfang, Squirrel General",
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
          "zone": "HAND",
          "player": 0,
          "name": "Raise the Alarm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Raise the Alarm"
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
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Chatterfang, Squirrel General",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldier Token",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Soldier Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Squirrel Token",
          "count": 0
        }
      ]
    },
    {
      "name": "testChatterfangPlusAcademyManufactor",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chatterfang, Squirrel General",
          "count": 1
        },
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
          "op": "setChoice",
          "player": 0,
          "value": "Academy Manufactor"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Chatterfang, Squirrel General",
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
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squirrel Token",
          "count": 3
        }
      ]
    }
  ]
});
