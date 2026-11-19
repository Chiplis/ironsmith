import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/DefiantVanguardTest.java",
  "tests": [
    {
      "name": "testAllDestroyed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Defiant Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bane Alley Blackguard",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Bane Alley Blackguard",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Defiant Vanguard",
          "attacker": "Bane Alley Blackguard"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "source": "Defiant Vanguard",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "END_COMBAT",
          "source": "Defiant Vanguard",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_COMBAT",
          "resolutions": 1
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Defiant Vanguard",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Bane Alley Blackguard",
          "count": 1
        }
      ]
    },
    {
      "name": "testSaveCreatureWithCloudshift",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Defiant Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bane Alley Blackguard@blackguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Bane Alley Blackguard",
          "defender": 0
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Defiant Vanguard",
          "attacker": "Bane Alley Blackguard"
        },
        {
          "op": "assertDamageReceived",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Defiant Vanguard",
          "damage": 1,
          "seatIndependent": true
        },
        {
          "op": "assertDamageReceived",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "player": 1,
          "name": "Bane Alley Blackguard",
          "damage": 2,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "player": 1,
          "name": "Cloudshift",
          "target": {
            "groups": [
              [
                "@blackguard"
              ]
            ]
          },
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "source": "Cloudshift",
          "count": 1,
          "isSpell": true,
          "objectTarget": "@blackguard"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertReenteredFixtures",
          "player": 1,
          "fixtures": [
            "@blackguard"
          ],
          "counts": [
            1
          ]
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Defiant Vanguard",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Bane Alley Blackguard",
          "count": 1
        }
      ]
    }
  ]
});
