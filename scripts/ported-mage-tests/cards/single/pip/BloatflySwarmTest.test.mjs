import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/pip/BloatflySwarmTest.java",
  "tests": [
    {
      "name": "test_Bolt",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bloatfly Swarm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Badlands",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bloatfly Swarm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Bloatfly Swarm"
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
          "name": "Bloatfly Swarm",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "RAD",
          "count": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "RAD",
          "count": 3
        }
      ]
    },
    {
      "name": "test_Combat",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Concordant Crossroads",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bloatfly Swarm",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Brimstone Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Giant Spider",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bloatfly Swarm"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bloatfly Swarm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Brimstone Dragon",
          "attacker": "Bloatfly Swarm"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Giant Spider",
          "attacker": "Bloatfly Swarm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "CHOICE_SKIP"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Bloatfly Swarm",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "RAD",
          "count": 5
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "RAD",
          "count": 5
        }
      ]
    },
    {
      "name": "test_Combat_Small",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Concordant Crossroads",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bloatfly Swarm",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wind Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Giant Spider",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bloatfly Swarm"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bloatfly Swarm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Wind Drake",
          "attacker": "Bloatfly Swarm"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Giant Spider",
          "attacker": "Bloatfly Swarm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "CHOICE_SKIP"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bloatfly Swarm",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "RAD",
          "count": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "RAD",
          "count": 4
        }
      ]
    }
  ]
});
