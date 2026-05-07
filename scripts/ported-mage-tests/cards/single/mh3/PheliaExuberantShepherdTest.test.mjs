import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/PheliaExuberantShepherdTest.java",
  "tests": [
    {
      "name": "test_Simple_Opponent",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phelia, Exuberant Shepherd",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Memnite",
          "count": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Simple_Owned",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phelia, Exuberant Shepherd",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Killed_OnTrigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
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
          "zone": "HAND",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phelia, Exuberant Shepherd",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Doom Blade",
          "target": "Phelia, Exuberant Shepherd"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Killed_AfterTrigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
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
          "zone": "HAND",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phelia, Exuberant Shepherd",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "name": "Doom Blade",
          "target": "Phelia, Exuberant Shepherd"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Blink_OnTrigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ephemerate",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phelia, Exuberant Shepherd",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Ephemerate",
          "target": "Phelia, Exuberant Shepherd"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Blink_AfterTrigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ephemerate",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phelia, Exuberant Shepherd",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "name": "Ephemerate",
          "target": "Phelia, Exuberant Shepherd"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Stifle",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Stifle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phelia, Exuberant Shepherd",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "END_TURN",
          "player": 1,
          "name": "Stifle",
          "target": "stack ability (At the beginning"
        },
        {
          "op": "assertExileCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Phelia, Exuberant Shepherd",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ornithopter"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Phelia, Exuberant Shepherd",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ornithopter",
          "count": 1
        }
      ]
    }
  ]
});
