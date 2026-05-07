import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/equipped/SwordOfTheMeekTest.java",
  "tests": [
    {
      "name": "testEquipAlive",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Master of Etherium",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Myrsmith",
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
          "name": "Chromatic Star",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Sword of the Meek",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chromatic Star"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Chromatic Star",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Myr Token",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Myr Token",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sword of the Meek",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Master of Etherium",
          "power": 3,
          "toughness": 3
        }
      ]
    }
  ]
});
