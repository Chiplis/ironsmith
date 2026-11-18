import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/battle/BattleMultiplayerTest.java",
  "tests": [
    {
      "name": "testRegularCastAndTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Invasion of Belenon",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Charlie"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Invasion of Belenon"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertBattleProtector",
          "player": 0,
          "protector": 2,
          "name": "Invasion of Belenon"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Knight Token",
          "count": 1
        }
      ]
    },
    {
      "name": "testAttackBattle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
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
          "zone": "HAND",
          "player": 0,
          "name": "Invasion of Belenon",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Charlie"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Invasion of Belenon"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": "Invasion of Belenon"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertBattleProtector",
          "player": 0,
          "protector": 2,
          "name": "Invasion of Belenon"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Knight Token",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Grizzly Bears",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 2,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Invasion of Belenon",
          "counter": "DEFENSE",
          "count": 3
        }
      ]
    },
    {
      "name": "testAttackBattleBlock",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
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
          "player": 2,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Invasion of Belenon",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Charlie"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Invasion of Belenon"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": "Invasion of Belenon"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 2,
          "blocker": "Grizzly Bears",
          "attacker": "Grizzly Bears"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertBattleProtector",
          "player": 0,
          "protector": 2,
          "name": "Invasion of Belenon"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Knight Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "Grizzly Bears",
          "count": 0,
          "player": 2
        },
        {
          "op": "assertGraveyardCount",
          "name": "Grizzly Bears",
          "count": 1,
          "player": 2
        },
        {
          "op": "assertLife",
          "player": 2,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Invasion of Belenon",
          "counter": "DEFENSE",
          "count": 5
        }
      ]
    }
  ],
  "turnOrder": [
    0,
    3,
    2,
    1
  ]
});
