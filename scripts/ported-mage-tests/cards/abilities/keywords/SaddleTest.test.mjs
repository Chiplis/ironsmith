import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/SaddleTest.java",
  "tests": [
    {
      "name": "testNoSaddle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Quilled Charger",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Quilled Charger",
          "defender": 1
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
          "op": "unsupported",
          "source": "assertTapped(charger, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSaddled(charger, false)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Quilled Charger",
          "ability": false,
          "expected": false
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    },
    {
      "name": "testSaddle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Quilled Charger",
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Saddle"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Quilled Charger",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertSaddled(charger, false)"
        }
      ]
    },
    {
      "name": "testSaddledThisTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rambling Possum",
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
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Saddle"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_SKIP"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Rambling Possum",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(lion, false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(possum, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSaddled(possum, true)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    },
    {
      "name": "testSaddledThisTurnFail",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rambling Possum",
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
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Arbor Elf",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Saddle"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears^Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_SKIP"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Rambling Possum",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Arbor Elf"
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
          "op": "unsupported",
          "source": "try { execute(); } catch (AssertionError e) { if (!e.getMessage().contains(\"Select creatures that saddled it this turn (selected 0)\")) { Assert.fail(\"Lion can't be targeted, but catch another error:\\n\" + e.getMessage()); } } assertTapped(bear, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(lion, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(elf, false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(possum, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSaddled(possum, true)"
        }
      ]
    }
  ]
});
