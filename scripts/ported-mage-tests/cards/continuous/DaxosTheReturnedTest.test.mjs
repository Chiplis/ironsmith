import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/DaxosTheReturnedTest.java",
  "tests": [
    {
      "name": "testCounterAddAndTokenStates",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 8
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
          "player": 0,
          "name": "Daxos the Returned",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Underworld Dreams",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Underworld Dreams"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Underworld Dreams"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W}{B}"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Underworld Dreams",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "EXPERIENCE",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Spirit Token",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Spirit Token\", CardType.ENCHANTMENT, SubType.SPIRIT)"
        }
      ]
    }
  ]
});
