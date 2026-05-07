import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/GoblinGoliathTest.java",
  "tests": [
    {
      "name": "test_DoubleDamage",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 20
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Goliath",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Barktooth Warbeard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"normal damage to opponent\", 1, PhaseStep.PRECOMBAT_MAIN, playerB, 20 - 3)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}{R}"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"double damage to opponent\", 1, PhaseStep.END_COMBAT, playerB, 20 - 3 - 3 * 2)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Barktooth Warbeard"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 1,
          "name": "Barktooth Warbeard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"normal damage to yourself\", 1, PhaseStep.END_COMBAT, playerA, 20 - 3)"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 4
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "20 - 3 - 3 * 2"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Barktooth Warbeard",
          "count": 0
        }
      ]
    }
  ]
});
