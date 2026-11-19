import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/woc/CourtOfGarenbrigTest.java",
  "tests": [
    {
      "name": "testNoTargetMonarch",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Garenbrig",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bond Beetle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bond Beetle",
          "resolve": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bond Beetle"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Garenbrig"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grizzly Bears",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Bond Beetle",
          "counter": "P1P1",
          "count": "1 * 2"
        }
      ]
    },
    {
      "name": "testNoTargetNotMonarch",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Garenbrig",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bond Beetle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bond Beetle",
          "resolve": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bond Beetle"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Garenbrig"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Goblin Piker",
          "defender": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grizzly Bears",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Bond Beetle",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "testOneTargetMonarch",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Garenbrig",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bond Beetle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bond Beetle"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bond Beetle"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Bond Beetle",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Bond Beetle",
          "count": 1,
          "abilityKind": "Triggered",
          "objectTarget": "Bond Beetle"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Garenbrig"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Court of Garenbrig",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 2
        },
        {
          "op": "assertMonarch",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "monarch": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                "Grizzly Bears"
              ]
            ]
          }
        },
        {
          "op": "addTargetAmount",
          "player": 0,
          "target": "Grizzly Bears",
          "amount": 2,
          "selectTarget": false
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "UPKEEP",
          "source": "Court of Garenbrig",
          "count": 1,
          "abilityKind": "Triggered",
          "targetCount": 1,
          "completeTargets": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "UPKEEP",
          "resolutions": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grizzly Bears",
          "counter": "P1P1",
          "count": "2 * 2"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Bond Beetle",
          "counter": "P1P1",
          "count": "1 * 2"
        }
      ]
    },
    {
      "name": "testOneTargetNotMonarch",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Garenbrig",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bond Beetle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bond Beetle"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bond Beetle"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Bond Beetle",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Bond Beetle",
          "count": 1,
          "abilityKind": "Triggered",
          "objectTarget": "Bond Beetle"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Garenbrig"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Court of Garenbrig",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 2
        },
        {
          "op": "assertMonarch",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "monarch": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Goblin Piker",
          "defender": 0
        },
        {
          "op": "assertMonarch",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "monarch": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                "Grizzly Bears"
              ]
            ]
          }
        },
        {
          "op": "addTargetAmount",
          "player": 0,
          "target": "Grizzly Bears",
          "amount": 2,
          "selectTarget": false
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "UPKEEP",
          "source": "Court of Garenbrig",
          "count": 1,
          "abilityKind": "Triggered",
          "targetCount": 1,
          "completeTargets": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "UPKEEP",
          "resolutions": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grizzly Bears",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Bond Beetle",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "testTwoTargetMonarch",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Garenbrig",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bond Beetle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bond Beetle",
          "resolve": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bond Beetle"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Garenbrig"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears^X=1"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bond Beetle^X=1"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grizzly Bears",
          "counter": "P1P1",
          "count": "1 * 2"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Bond Beetle",
          "counter": "P1P1",
          "count": "(1+1) * 2"
        }
      ]
    },
    {
      "name": "testTwoTargetNotMonarch",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Court of Garenbrig",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bond Beetle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bond Beetle"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Bond Beetle",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bond Beetle"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Bond Beetle",
          "count": 1,
          "counter": "P1P1",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Court of Garenbrig"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Court of Garenbrig",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 2
        },
        {
          "op": "assertMonarch",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "monarch": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Goblin Piker",
          "defender": 0
        },
        {
          "op": "assertMonarch",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "monarch": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                "Grizzly Bears",
                "Bond Beetle"
              ]
            ]
          }
        },
        {
          "op": "addTargetAmount",
          "player": 0,
          "target": "Grizzly Bears",
          "amount": 1,
          "selectTarget": false
        },
        {
          "op": "addTargetAmount",
          "player": 0,
          "target": "Bond Beetle",
          "amount": 1,
          "selectTarget": false
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grizzly Bears",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Bond Beetle",
          "counter": "P1P1",
          "count": 2
        }
      ]
    }
  ]
});
