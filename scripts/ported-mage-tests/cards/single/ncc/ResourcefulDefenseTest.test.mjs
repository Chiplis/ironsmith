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
          "op": "setChoice",
          "value": "Green"
        },
        {
          "op": "setChoice",
          "value": "Green"
        },
        {
          "op": "setChoice",
          "value": "Green"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Archway Commons",
          "ability": "Add one mana of any color",
          "count": 3
        },
        {
          "op": "assertChoiceMenu",
          "kind": "number",
          "description": "Choose value for X",
          "player": 0,
          "value": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Steelbane Hydra"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Steelbane Hydra",
          "count": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Steelbane Hydra",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vivid Creek",
          "counter": "CHARGE",
          "count": 2
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Archway Commons",
          "ability": "Add one mana of any color",
          "count": 5
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{W}: ",
          "source": "Resourceful Defense"
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
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Resourceful Defense",
          "count": 1,
          "onTop": true,
          "objectTarget": "Vivid Creek",
          "targetCount": 2
        },
        {
          "op": "assertChoiceMenu",
          "kind": "number",
          "player": 0,
          "description": "Choose how many charge counters to move from Vivid Creek",
          "value": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Steelbane Hydra",
          "counter": "CHARGE",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vivid Creek",
          "counter": "CHARGE",
          "count": 0
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "setChoice",
          "value": "White"
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Archway Commons",
          "ability": "Add one mana of any color",
          "count": 8
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{W}: ",
          "source": "Resourceful Defense"
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
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Resourceful Defense",
          "count": 1,
          "onTop": true,
          "objectTarget": "Steelbane Hydra",
          "targetCount": 2
        },
        {
          "op": "assertChoiceMenu",
          "kind": "number",
          "player": 0,
          "description": "Choose how many +1/+1 counters to move from Steelbane Hydra",
          "value": 1
        },
        {
          "op": "assertChoiceMenu",
          "kind": "number",
          "player": 0,
          "description": "Choose how many charge counters to move from Steelbane Hydra",
          "value": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Resourceful Defense",
          "abilityKind": "Triggered",
          "count": 0
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
