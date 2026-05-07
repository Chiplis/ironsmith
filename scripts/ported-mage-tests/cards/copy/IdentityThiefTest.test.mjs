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
          "name": "Molten Sentry"
        },
        {
          "op": "unsupported",
          "source": "setFlipCoinResult(playerA, true)"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Identity Thief",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Molten Sentry"
        },
        {
          "op": "setChoice",
          "player": 1,
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
          "op": "assertExileCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
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
      ]
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
          "op": "setChoice",
          "player": 0,
          "value": "a 3/3 artifact creature"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Identity Thief",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Yes"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Primal Clay"
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
          "defender": 1
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
          "count": 0,
          "name": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
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
