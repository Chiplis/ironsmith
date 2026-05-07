import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/AddManaAnyCombinationTest.java",
  "tests": [
    {
      "name": "testOrcishLumberjack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Orcish Lumberjack",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Living Twister",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}, Sacrifice a Forest: Add three mana in any combination of {R} and/or {G}."
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Forest"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 2, 1)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Living Twister"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Living Twister",
          "power": 2,
          "toughness": 5
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Forest",
          "count": 1
        }
      ]
    }
  ]
});
