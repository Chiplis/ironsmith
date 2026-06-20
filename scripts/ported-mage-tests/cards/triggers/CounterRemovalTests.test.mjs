import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/CounterRemovalTests.java",
  "tests": [
    {
      "name": "CounterRemovalTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Basri's Solidarity",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aether Snap",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"multicountertrigdcard\", playerA, ability, null, CardType.CREATURE, \"\", Zone.BATTLEFIELD)"
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"singlecountertrigdcard\", playerA, ability, null, CardType.CREATURE, \"\", Zone.BATTLEFIELD)"
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"multicounterreplcard\", playerA, ability, null, CardType.CREATURE, \"\", Zone.BATTLEFIELD)"
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"singlecounterreplcard\", playerA, ability, null, CardType.CREATURE, \"\", Zone.BATTLEFIELD)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When a counter"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": "currentGame.getStartingLife() - 1"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "ENERGY",
          "count": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "multicounterreplcard",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "singlecounterreplcard",
          "counter": "P1P1",
          "count": 2
        }
      ]
    }
  ]
});
