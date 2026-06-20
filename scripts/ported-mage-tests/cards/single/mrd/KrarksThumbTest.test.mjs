import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mrd/KrarksThumbTest.java",
  "tests": [
    {
      "name": "test_NoThumb",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Traprunner",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"Test Trigger\", playerA, new KrarksThumbTestTriggeredAbility())"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Goblin Traprunner",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever you flip a coin, you gain 1 life"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        }
      ]
    },
    {
      "name": "test_OneThumb",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Traprunner",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Krark's Thumb",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"Test Trigger\", playerA, new KrarksThumbTestTriggeredAbility())"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, false)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Goblin Traprunner",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever you flip a coin, you gain 2 life"
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
          "op": "assertLife",
          "player": 0,
          "life": "20 + 3 * 2"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 2
        }
      ]
    },
    {
      "name": "test_TwoThumb",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Traprunner",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mirror Box",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Krark's Thumb",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"Test Trigger\", playerA, new KrarksThumbTestTriggeredAbility())"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Krark's Thumb"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, false)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Krark's Thumb"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Krark's Thumb"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, false)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Goblin Traprunner",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever you flip a coin, you gain 3 life"
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
          "op": "assertLife",
          "player": 0,
          "life": "20 + 3 * 3"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 2
        }
      ]
    }
  ]
});
