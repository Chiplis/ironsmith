import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ncc/MariTheKillingQuillTest.java",
  "tests": [
    {
      "name": "testExiledWithCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mari, the Killing Quill",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Universal Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Metallic Sliver",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Metallic Sliver"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Universal Automaton"
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
          "op": "assertExileCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Universal Automaton",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Metallic Sliver",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertCounterOnExiledCardCount(sliver, CounterType.HIT, 1)"
        }
      ]
    },
    {
      "name": "testDrawAndTreasure",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mari, the Killing Quill",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Universal Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Metallic Sliver",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Metallic Sliver"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Universal Automaton",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Metallic Sliver"
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
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 2
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Metallic Sliver",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertCounterOnExiledCardCount(sliver, CounterType.HIT, 0)"
        }
      ]
    }
  ]
});
