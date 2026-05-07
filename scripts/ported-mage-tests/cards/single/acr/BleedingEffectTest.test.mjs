import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/acr/BleedingEffectTest.java",
  "tests": [
    {
      "name": "testAbilitiesGained",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bleeding Effect",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Knight of Malice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Boggart Brute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rumbling Baloth",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Rumbling Baloth",
          "ability": "FirstStrike",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Rumbling Baloth",
          "ability": "HexproofFromWhite",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Rumbling Baloth",
          "ability": false,
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Rumbling Baloth",
          "ability": "Flying",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Rumbling Baloth",
          "ability": "HexproofFromBlue",
          "expected": false
        }
      ]
    }
  ]
});
