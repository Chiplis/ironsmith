import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/unf/NameStickerGoblinTest.java",
  "tests": [
    {
      "name": "testBasicETB",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Name Sticker\" Goblin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 1)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "\"Name Sticker\" Goblin"
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"Mana\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"R\", 4)"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testGraveyardETB",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "\"Name Sticker\" Goblin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unearth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unearth",
          "target": "\"Name Sticker\" Goblin"
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"No Mana\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"R\", 0)"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testExileETB",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "\"Name Sticker\" Goblin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cloudshift",
          "target": "\"Name Sticker\" Goblin"
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"No mana\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"R\", 0)"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testNineETB",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "\"Name Sticker\" Goblin",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Name Sticker\" Goblin",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 15)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "\"Name Sticker\" Goblin"
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"Mana\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"R\", 6)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "\"Name Sticker\" Goblin"
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"No extra mana\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"R\", 3)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "\"Name Sticker\" Goblin"
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"No extra mana\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"R\", 0)"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
