import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ths/HeliodGodOfTheSun.java",
  "tests": [
    {
      "name": "testHeliodBecomesCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Angel of Jubilation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Heliod, God of the Sun",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spear of Heliod",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hold the Gates",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Angel of Jubilation"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Heliod, God of the Sun"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spear of Heliod"
        },
        {
          "op": "castSpell",
          "turn": 7,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hold the Gates"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel of Jubilation",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Heliod, God of the Sun",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Spear of Heliod",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hold the Gates",
          "count": 1
        }
      ]
    }
  ]
});
