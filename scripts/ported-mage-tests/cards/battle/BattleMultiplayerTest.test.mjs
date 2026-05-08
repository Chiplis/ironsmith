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
          "name": "belenon",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "playerC.getName()"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "belenon"
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
          "op": "unsupported",
          "source": "assertBattle(playerA, playerC, belenon)"
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
          "name": "bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "belenon",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "playerC.getName()"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "belenon"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "bear",
          "defender": "belenon"
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
          "op": "unsupported",
          "source": "assertBattle(playerA, playerC, belenon)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Knight Token",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(bear, true)"
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "belenon",
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
          "name": "bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "belenon",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "playerC.getName()"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "belenon"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "bear",
          "defender": "belenon"
        },
        {
          "op": "block",
          "turn": 1,
          "player": "playerC",
          "blocker": "bear",
          "attacker": "bear"
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
          "op": "unsupported",
          "source": "assertBattle(playerA, playerC, belenon)"
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
          "name": "bear",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "bear",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "bear",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "name": "bear",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "belenon",
          "counter": "DEFENSE",
          "count": 5
        }
      ]
    }
  ]
});
