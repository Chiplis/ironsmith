import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/GwenStacyTest.java",
  "tests": [
    {
      "name": "testGhostSpider",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gwen Stacy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ghost-Spider",
          "resolve": true
        },
        {
          "op": "addObjectCounters",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ghost-Spider",
          "counter": "+1/+1",
          "count": 1
        },
        {
          "op": "addObjectCounters",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ghost-Spider",
          "counter": "Charge",
          "count": 1
        },
        {
          "op": "addObjectCounters",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ghost-Spider",
          "counter": "+0/+1",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Exile the top card",
          "source": "Ghost-Spider"
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose one object to remove counters from",
          "choices": [
            "Ghost-Spider"
          ]
        },
        {
          "op": "setCounterChoice",
          "player": 0,
          "counts": {
            "Charge": 1,
            "+0/+1": 1
          }
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ghost-Spider",
          "counter": "+1/+1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ghost-Spider",
          "counter": "+0/+1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ghost-Spider",
          "counter": "Charge",
          "count": 0
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain"
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
          "name": "Ghost-Spider",
          "power": 6,
          "toughness": 6
        }
      ]
    }
  ]
});
