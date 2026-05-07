import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/SakashimaTheImpostorTest.java",
  "tests": [
    {
      "name": "copySpellStutterTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spellstutter Sprite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Sakashima the Impostor",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Sakashima the Impostor"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Spellstutter Sprite"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Spellstutter Sprite",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Sakashima the Impostor",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "copyDiesTriggeredTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pawn of Ulamog",
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
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Sakashima the Impostor",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Sakashima the Impostor"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Pawn of Ulamog"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Sakashima the Impostor",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 4,
          "player": 0,
          "blocker": "Silvercoat Lion",
          "attacker": "Sakashima the Impostor"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Sakashima the Impostor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Eldrazi Spawn Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Eldrazi Spawn Token",
          "count": 1
        }
      ]
    }
  ]
});
