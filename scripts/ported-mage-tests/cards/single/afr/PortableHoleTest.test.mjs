import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/afr/PortableHoleTest.java",
  "tests": [
    {
      "name": "opponentsPermanentisExiled",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Portable Hole",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Portable Hole"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "opponentsPermanentisReturned",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Portable Hole",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Shatter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 2
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Portable Hole"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Shatter",
          "target": "Portable Hole"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    }
  ]
});
