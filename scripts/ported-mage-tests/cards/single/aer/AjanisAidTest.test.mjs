import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/aer/AjanisAidTest.java",
  "tests": [
    {
      "name": "test_DamageOnCreature_Prevent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ajani's Aid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Caelorna, Coral Tyrant",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Piker"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Goblin Piker",
          "defender": 0
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Caelorna, Coral Tyrant",
          "attacker": "Goblin Piker"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Caelorna, Coral Tyrant\", 0)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Goblin Piker\", true)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ajani's Aid",
          "count": 1
        }
      ]
    },
    {
      "name": "test_DamageOnYou_Prevent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ajani's Aid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Piker"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Goblin Piker",
          "defender": 0
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Goblin Piker\", true)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ajani's Aid",
          "count": 1
        }
      ]
    },
    {
      "name": "test_DamageNonCombat_NoPrevent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ajani's Aid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Prodigal Pyromancer",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Prodigal Pyromancer"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: ",
          "target": 0
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
          "op": "assertLife",
          "player": 0,
          "life": 19
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ajani's Aid",
          "count": 1
        }
      ]
    }
  ]
});
