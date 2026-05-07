import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/LadyOctopusInspiredInventorTest.java",
  "tests": [
    {
      "name": "testLadyOctopusInspiredInventor",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "untap all creatures",
          "player": 0,
          "name": "new SimpleActivatedAbility(\n                new UntapAllControllerEffect(new FilterCreaturePermanent()),\n                new ManaCostsImpl<>(\"\")\n        )",
          "custom": true,
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "draw a card",
          "player": 0,
          "name": "new SimpleActivatedAbility(\n                new DrawCardSourceControllerEffect(1),\n                new ManaCostsImpl<>(\"\")\n        )",
          "custom": true,
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lady Octopus, Inspired Inventor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aether Vial",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tormod's Crypt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Howling Mine",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "activateDrawCardAndUntap()"
        },
        {
          "op": "unsupported",
          "source": "activateDrawCardAndUntap()"
        },
        {
          "op": "unsupported",
          "source": "activateDrawCardAndUntap()"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Lady Octopus, Inspired Inventor",
          "counter": "INGENUITY",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 3
        }
      ]
    },
    {
      "name": "testLadyOctopusInspiredInventorChoose",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "draw a card",
          "player": 0,
          "name": "new SimpleActivatedAbility(\n                new DrawCardSourceControllerEffect(3),\n                new ManaCostsImpl<>(\"\")\n        )",
          "custom": true,
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lady Octopus, Inspired Inventor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aether Vial",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tormod's Crypt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Howling Mine",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "draw "
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever you draw your first"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You may cast"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Tormod's Crypt"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Lady Octopus, Inspired Inventor",
          "counter": "INGENUITY",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 5
        }
      ]
    }
  ]
});
