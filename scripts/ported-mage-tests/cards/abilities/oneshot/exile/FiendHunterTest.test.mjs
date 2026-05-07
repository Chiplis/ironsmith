import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/exile/FiendHunterTest.java",
  "tests": [
    {
      "name": "testExileWorks",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fiend Hunter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Primeval Titan",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fiend Hunter"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Primeval Titan",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Fiend Hunter",
          "count": 1
        }
      ]
    },
    {
      "name": "testExileAndReturnWorks",
      "operations": [
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
          "player": 0,
          "name": "Fiend Hunter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Restoration Angel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Primeval Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fiend Hunter"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Restoration Angel"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Fiend Hunter",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Restoration Angel",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Primeval Titan",
          "count": 1
        }
      ]
    }
  ]
});
