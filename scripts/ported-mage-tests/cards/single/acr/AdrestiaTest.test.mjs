import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/acr/AdrestiaTest.java",
  "tests": [
    {
      "name": "testAdrestia",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Adrestia",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nekrataal",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Crew"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Nekrataal"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Adrestia",
          "defender": 1
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
          "op": "unsupported",
          "source": "assertSubtype(adrestia, SubType.VEHICLE)"
        },
        {
          "op": "unsupported",
          "source": "assertType(adrestia, CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(adrestia, CardType.ARTIFACT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(adrestia, SubType.ASSASSIN)"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        }
      ]
    }
  ]
});
