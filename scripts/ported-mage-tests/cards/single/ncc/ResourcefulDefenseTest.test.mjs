import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ncc/ResourcefulDefenseTest.java",
  "tests": [
    {
      "name": "testMoveWhenDied",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Archway Commons",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Resourceful Defense",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Everflowing Chalice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Steelbane Hydra",
          "count": 1
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
          "name": "Steelbane Hydra"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Steelbane Hydra"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Everflowing Chalice"
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
          "name": "Everflowing Chalice",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "testMoveAllSingleCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Archway Commons",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Resourceful Defense",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vivid Creek",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Everflowing Chalice",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{W}: "
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Vivid Creek"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Everflowing Chalice"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 2)"
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
          "name": "Vivid Creek",
          "counter": "CHARGE",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Everflowing Chalice",
          "counter": "CHARGE",
          "count": 2
        }
      ]
    },
    {
      "name": "testSomeAllSingleCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Archway Commons",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Resourceful Defense",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vivid Creek",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Everflowing Chalice",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{W}: "
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Vivid Creek"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Everflowing Chalice"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 1)"
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
          "name": "Vivid Creek",
          "counter": "CHARGE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Everflowing Chalice",
          "counter": "CHARGE",
          "count": 1
        }
      ]
    },
    {
      "name": "testMoveAllMultipleCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Archway Commons",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Resourceful Defense",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vivid Creek",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Steelbane Hydra",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Steelbane Hydra"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
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
          "ability": "{4}{W}: "
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Vivid Creek"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Steelbane Hydra"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 2)"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{W}: "
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Steelbane Hydra"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Vivid Creek"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 1, 2)"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vivid Creek",
          "counter": "CHARGE",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vivid Creek",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Steelbane Hydra",
          "count": 1
        }
      ]
    },
    {
      "name": "testMoveMultipleWhenDied",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Archway Commons",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Resourceful Defense",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Everflowing Chalice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vivid Creek",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Steelbane Hydra",
          "count": 1
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
          "name": "Steelbane Hydra"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
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
          "ability": "{4}{W}: "
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Vivid Creek"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Steelbane Hydra"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 2)"
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
          "name": "Lightning Bolt",
          "target": "Steelbane Hydra"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Everflowing Chalice"
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
          "name": "Vivid Creek",
          "counter": "CHARGE",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Everflowing Chalice",
          "counter": "CHARGE",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Everflowing Chalice",
          "counter": "P1P1",
          "count": 1
        }
      ]
    }
  ]
});
