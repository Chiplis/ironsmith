import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/damage/SpitefulShadowsTest.java",
  "tests": [
    {
      "name": "SpitefulShadowsPoisonTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glistener Elf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spiteful Shadows",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spiteful Shadows",
          "target": "Glistener Elf"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Glistener Elf"
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
          "life": "currentGame.getStartingLife()"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife()"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "POISON",
          "count": 3
        }
      ]
    },
    {
      "name": "SpitefulShadowsRegularTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spiteful Shadows",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spiteful Shadows",
          "target": "Craw Wurm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Craw Wurm"
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
          "life": "currentGame.getStartingLife() - 3"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife()"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "POISON",
          "count": 0
        }
      ]
    },
    {
      "name": "SpitefulShadowsMultiDamageTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Agent of Stromgald",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Chandra's Spitfire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spiteful Shadows",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spiteful Shadows",
          "target": "Craw Wurm"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Craw Wurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Memnite",
          "attacker": "Craw Wurm"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Agent of Stromgald",
          "attacker": "Craw Wurm"
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
          "life": "currentGame.getStartingLife() - 2"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife()"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "POISON",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Chandra's Spitfire",
          "power": 4,
          "toughness": 3
        }
      ]
    }
  ]
});
