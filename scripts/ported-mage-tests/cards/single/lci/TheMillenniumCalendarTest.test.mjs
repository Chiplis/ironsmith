import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lci/TheMillenniumCalendarTest.java",
  "tests": [
    {
      "name": "test_untap_effect_not_triggering",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Millennium Calendar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Aphetto Alchemist",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{T}: Untap target artifact or creature.",
          "target": "Aphetto Alchemist"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Millennium Calendar",
          "counter": "TIME",
          "count": 0
        }
      ]
    },
    {
      "name": "test_untap_trigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Millennium Calendar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Raging Goblin",
          "count": 10
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Millennium Calendar",
          "counter": "TIME",
          "count": 10
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 10
        }
      ]
    }
  ]
});
