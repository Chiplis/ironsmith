import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/snc/UnlicensedHearseTest.java",
  "tests": [
    {
      "name": "testExileOneCardFromGraveyard",
      "operations": [
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Exile up to two target cards from a single graveyard.",
          "target": "Grizzly Bears"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Unlicensed Hearse",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testExileTwoCardsFromGraveyard",
      "operations": [
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Exile up to two target cards from a single graveyard.",
          "target": "new String[]{\"Grizzly Bears\", \"Forest Bear\"}"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Unlicensed Hearse",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
