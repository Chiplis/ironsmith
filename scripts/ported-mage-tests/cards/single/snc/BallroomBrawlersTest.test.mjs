import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/snc/BallroomBrawlersTest.java",
  "tests": [
    {
      "name": "testSoloFirstStrike",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ballroom Brawlers",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Ballroom Brawlers",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Raging Goblin",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "first strike"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ballroom Brawlers",
          "ability": "Lifelink",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Raging Goblin",
          "ability": "Lifelink",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ballroom Brawlers",
          "ability": "FirstStrike",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Raging Goblin",
          "ability": "FirstStrike",
          "expected": false
        }
      ]
    },
    {
      "name": "testBothLifelink",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ballroom Brawlers",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Ballroom Brawlers",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Raging Goblin",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Raging Goblin"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "lifelink"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ballroom Brawlers",
          "ability": "Lifelink",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Raging Goblin",
          "ability": "Lifelink",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ballroom Brawlers",
          "ability": "FirstStrike",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Raging Goblin",
          "ability": "FirstStrike",
          "expected": false
        }
      ]
    }
  ]
});
