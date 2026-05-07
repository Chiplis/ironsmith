import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ala/DeathBaronTest.java",
  "tests": [
    {
      "name": "testDoesntNormallyAffectSelf",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Death Baron",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Drudge Skeletons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scathe Zombies",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Black Knight",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Drudge Skeletons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Scathe Zombies",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Death Baron",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Death Baron",
          "ability": "deathtouch",
          "expected": false
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Drudge Skeletons",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Scathe Zombies",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Black Knight",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Drudge Skeletons",
          "ability": "deathtouch",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Scathe Zombies",
          "ability": "deathtouch",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Black Knight",
          "ability": "deathtouch",
          "expected": false
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Drudge Skeletons",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Scathe Zombies",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Drudge Skeletons",
          "ability": "deathtouch",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Scathe Zombies",
          "ability": "deathtouch",
          "expected": false
        }
      ]
    },
    {
      "name": "testBecomeSkeleton",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Death Baron",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Amoeboid Changeling",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target creature gains",
          "target": "Death Baron"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Death Baron",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Death Baron",
          "ability": "Deathtouch",
          "expected": true
        }
      ]
    }
  ]
});
