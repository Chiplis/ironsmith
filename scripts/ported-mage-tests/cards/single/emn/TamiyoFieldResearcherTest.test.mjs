import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/emn/TamiyoFieldResearcherTest.java",
  "tests": [
    {
      "name": "testFieldResearcherFirstEffectOnGideon",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gideon, Ally of Zendikar",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Create a",
          "source": "Gideon, Ally of Zendikar"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gideon, Ally of Zendikar",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Until end of turn",
          "source": "Gideon, Ally of Zendikar"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Gideon, Ally of Zendikar",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Choose up to two",
          "source": "Tamiyo, Field Researcher",
          "target": {
            "groups": [
              [
                "Knight Ally Token",
                "Gideon, Ally of Zendikar"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Tamiyo, Field Researcher",
          "count": 1,
          "abilityKind": "Activated",
          "targetCount": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Knight Ally Token",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Gideon, Ally of Zendikar",
          "defender": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "source": "Tamiyo, Field Researcher",
          "count": 2,
          "abilityKind": "Triggered",
          "completeChoices": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 13
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gideon, Ally of Zendikar",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Knight Ally Token",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        }
      ]
    },
    {
      "name": "testFieldResearcherFirstEffectSimpleCreatureAttacks",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Choose up to two"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bronze Sable"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bronze Sable",
          "defender": 1
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
          "player": 1,
          "life": 18
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        }
      ]
    },
    {
      "name": "testFieldResearcherFirstEffectSimpleCreaturesAttacks",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sylvan Advocate",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Choose up to two"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bronze Sable^Sylvan Advocate"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bronze Sable",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sylvan Advocate",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Until your next turn"
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
          "player": 1,
          "life": 16
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        }
      ]
    },
    {
      "name": "testFieldResearcherFirstEffectAttackAndBlock",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sylvan Advocate",
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
          "name": "Tamiyo, Field Researcher"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Choose up to two",
          "source": "Tamiyo, Field Researcher",
          "target": {
            "groups": [
              [
                "Sylvan Advocate"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Tamiyo, Field Researcher",
          "count": 1,
          "abilityKind": "Activated",
          "targetCount": 1,
          "objectTarget": "Sylvan Advocate"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sylvan Advocate",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "source": "Tamiyo, Field Researcher",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "resolutions": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Memnite",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Sylvan Advocate",
          "attacker": "Memnite"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "source": "Tamiyo, Field Researcher",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "resolutions": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        }
      ]
    },
    {
      "name": "testFieldResearcherFirstEffectOnlyPersistsUntilYourNextTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sylvan Advocate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Hero's Downfall",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tamiyo, Field Researcher"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Choose up to two",
          "source": "Tamiyo, Field Researcher",
          "target": {
            "groups": [
              [
                "Sylvan Advocate"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Tamiyo, Field Researcher",
          "count": 1,
          "abilityKind": "Activated",
          "targetCount": 1,
          "objectTarget": "Sylvan Advocate"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sylvan Advocate",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "source": "Tamiyo, Field Researcher",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "resolutions": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Memnite",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Sylvan Advocate",
          "attacker": "Memnite"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "source": "Tamiyo, Field Researcher",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "resolutions": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Hero's Downfall",
          "target": "Tamiyo, Field Researcher"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "UPKEEP",
          "source": "Hero's Downfall",
          "count": 1,
          "objectTarget": "Tamiyo, Field Researcher"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "UPKEEP",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "name": "Tamiyo, Field Researcher",
          "player": 0,
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Sylvan Advocate",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Hero's Downfall",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        }
      ]
    },
    {
      "name": "testDrawEffectGetsRemoved",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sylvan Advocate",
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
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Choose up to two",
          "target": "Sylvan Advocate"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sylvan Advocate",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Choose up to two",
          "target": "Pillarfield Ox^Silvercoat Lion"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Pillarfield Ox",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Until your next turn"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 14
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 4
        }
      ]
    },
    {
      "name": "testFieldResearcherFirstAbilityTargetOpponentCreature",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Choose up to two"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bronze Sable"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Bronze Sable",
          "defender": 0
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        }
      ]
    },
    {
      "name": "testFieldResearcherFirstAbilityTargetOpponentCreatures",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tamiyo, Field Researcher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bronze Sable",
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
          "name": "Tamiyo, Field Researcher",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Choose up to two"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bronze Sable^Memnite"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Bronze Sable",
          "defender": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Memnite",
          "defender": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Until your next turn"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        }
      ]
    }
  ]
});
