import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/who/TheWarGamesTest.java",
  "tests": [
    {
      "name": "test_SimplePlay_NoExile",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The War Games",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The War Games"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 8,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Warrior Token",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Warrior Token",
          "count": 3
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 2
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 5
        }
      ]
    },
    {
      "name": "test_SimplePlay_Exile",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The War Games",
          "count": 1
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The War Games"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Warrior Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Warrior Token",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 2
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 5
        }
      ]
    }
  ]
});
