import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/BackupTest.java",
  "tests": [
    {
      "name": "ConclaveSledgeCaptainTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Conclave Sledge-Captain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Raging Goblin",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Conclave Sledge-Captain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Raging Goblin"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Raging Goblin"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Raging Goblin"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Raging Goblin",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Memnite",
          "attacker": "Raging Goblin"
        },
        {
          "op": "setStrictChooseMode",
          "value": false
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
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Raging Goblin",
          "power": 13,
          "toughness": 13
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Conclave Sledge-Captain",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Conclave Sledge-Captain",
          "power": 4,
          "toughness": 4
        }
      ]
    },
    {
      "name": "ConclaveSledgeCaptainSelfTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Conclave Sledge-Captain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fervor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Conclave Sledge-Captain"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Conclave Sledge-Captain",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": false
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
          "name": "Conclave Sledge-Captain",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Conclave Sledge-Captain",
          "power": 14,
          "toughness": 14
        }
      ]
    },
    {
      "name": "ConclaveSledgeCaptainSplitTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Conclave Sledge-Captain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Raging Cougar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Raging Minotaur",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Conclave Sledge-Captain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Raging Goblin"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Raging Cougar"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Raging Minotaur"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Raging Goblin",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Raging Cougar",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Raging Minotaur",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": false
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Raging Goblin",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Raging Cougar",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Raging Minotaur",
          "power": 8,
          "toughness": 8
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Conclave Sledge-Captain",
          "power": 4,
          "toughness": 4
        }
      ]
    },
    {
      "name": "MirrorShieldHopliteStrictTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mirror-Shield Hoplite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Consuming Aetherborn",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Consuming Aetherborn"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Consuming Aetherborn"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mirror-Shield Hoplite"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Mirror-Shield Hoplite",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Consuming Aetherborn",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "MirrorShieldHopliteTriggeredTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mirror-Shield Hoplite",
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
          "name": "Enduring Bondwarden",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Enduring Bondwarden"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mirror-Shield Hoplite"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Mirror-Shield Hoplite"
        },
        {
          "op": "setStrictChooseMode",
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Enduring Bondwarden",
          "power": 4,
          "toughness": 5
        }
      ]
    },
    {
      "name": "MirrorShieldHopliteSourceTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mirror-Shield Hoplite",
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
          "name": "Enduring Bondwarden",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Enduring Bondwarden"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mirror-Shield Hoplite"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Enduring Bondwarden"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Enduring Bondwarden"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mirror-Shield Hoplite"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Mirror-Shield Hoplite",
          "power": 4,
          "toughness": 4
        }
      ]
    },
    {
      "name": "StreetwiseNegotiatorTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Air-Cult Elemental",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Streetwise Negotiator",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Streetwise Negotiator"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Air-Cult Elemental"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Air-Cult Elemental",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after 1\", 1, PhaseStep.POSTCOMBAT_MAIN, playerB, 20 - 1 - 5)"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Air-Cult Elemental",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Streetwise Negotiator",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "20 - (1 + 5) - (1 + 2 + 2)"
        }
      ]
    }
  ]
});
