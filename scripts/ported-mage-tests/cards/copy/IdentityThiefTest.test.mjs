import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/IdentityThiefTest.java",
  "tests": [
    {
      "name": "testCopyCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Molten Sentry",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Identity Thief",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Molten Sentry",
          "coinFaces": [
            "heads"
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Molten Sentry",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Molten Sentry",
          "power": 5,
          "toughness": 2
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": {
            "groups": [
              [
                "Molten Sentry"
              ]
            ]
          }
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "boolean",
          "description": "Exile another target nontoken creature",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Identity Thief",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "source": "Identity Thief",
          "count": 1,
          "abilityKind": "Triggered",
          "objectTarget": "Molten Sentry"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "resolutions": 1
        },
        {
          "op": "assertExileCount",
          "name": "Molten Sentry",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Identity Thief",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "name": "Molten Sentry",
          "player": 1,
          "power": 5,
          "toughness": 2,
          "turn": 2,
          "phase": "DECLARE_ATTACKERS"
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
          "op": "assertExileCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertExileCount",
          "name": "Molten Sentry",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Identity Thief",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Molten Sentry",
          "count": 1
        }
      ],
      "seed": 3
    },
    {
      "name": "testCopyPrimalClay",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Primal Clay",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Identity Thief",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Primal Clay"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "3/3"
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Primal Clay",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": {
            "groups": [
              [
                "Primal Clay"
              ]
            ]
          }
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "boolean",
          "description": "Exile another target nontoken creature",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Identity Thief",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "source": "Identity Thief",
          "count": 1,
          "abilityKind": "Triggered",
          "objectTarget": "Primal Clay"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "resolutions": 1
        },
        {
          "op": "assertExileCount",
          "name": "Primal Clay",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Identity Thief",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "name": "Primal Clay",
          "player": 1,
          "power": 3,
          "toughness": 3,
          "turn": 2,
          "phase": "DECLARE_ATTACKERS"
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Primal Clay",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Identity Thief",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Primal Clay",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Primal Clay",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "testShouldNotCopyP1P1Counters",
      "operations": [
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
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Identity Thief",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Battlegrowth"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Sylvan Advocate"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Identity Thief",
          "defender": 0
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Sylvan Advocate"
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
          "op": "assertExileCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertExileCount",
          "name": "Sylvan Advocate",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Identity Thief",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Sylvan Advocate",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Sylvan Advocate",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Sylvan Advocate",
          "power": 2,
          "toughness": 3
        }
      ]
    }
  ]
});
