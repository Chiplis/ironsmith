import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/who/TheDayOfTheDoctorTest.java",
  "tests": [
    {
      "name": "test_SimplePlay",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Yawgmoth's Bargain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Academy Ruins",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Arcade Gannon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Memnite",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Thalia, Guardian of Thraben",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Ninth Doctor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Day of the Doctor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Day of the Doctor"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Thalia, Guardian of Thraben",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Arcade Gannon",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Academy Ruins",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Thalia, Guardian of Thraben",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Arcade Gannon",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Academy Ruins",
          "count": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertExileCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thalia, Guardian of Thraben",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Arcade Gannon",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Academy Ruins",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Academy Ruins"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thalia, Guardian of Thraben"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Arcade Gannon",
          "expected": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "The Ninth Doctor"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Arcade Gannon",
          "expected": false
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Thalia, Guardian of Thraben",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "The Ninth Doctor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Academy Ruins",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Arcade Gannon",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 7
        }
      ]
    }
  ]
});
