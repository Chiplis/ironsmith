import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/control/TargetOpponentGainsControlTest.java",
  "tests": [
    {
      "name": "testPermanentControlEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Treacherous Pit-Dweller@dw",
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
          "op": "castSpell",
          "name": "Lightning Bolt",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "@dw"
        },
        {
          "op": "assertStackObject",
          "source": "Lightning Bolt",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Treacherous Pit-Dweller"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@dw",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Treacherous Pit-Dweller",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@dw"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Treacherous Pit-Dweller",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered",
          "completeTargets": true,
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@dw"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "@dw",
          "count": 1,
          "counter": "+1/+1"
        }
      ]
    },
    {
      "name": "testChangeControlEffectFromTwoCards",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unhallowed Pact",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Treacherous Pit-Dweller@dw",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
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
          "name": "Lightning Bolt",
          "player": 0,
          "turn": 1,
          "phase": "UPKEEP",
          "target": "@dw"
        },
        {
          "op": "assertStackObject",
          "source": "Lightning Bolt",
          "count": 1,
          "turn": 1,
          "phase": "UPKEEP",
          "isSpell": true,
          "objectTarget": "Treacherous Pit-Dweller"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@dw",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Treacherous Pit-Dweller",
          "count": 1,
          "turn": 1,
          "phase": "UPKEEP",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@dw"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Treacherous Pit-Dweller",
          "count": 1,
          "turn": 1,
          "phase": "UPKEEP",
          "abilityKind": "Triggered",
          "completeTargets": true,
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@dw"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "@dw",
          "count": 1,
          "counter": "+1/+1"
        },
        {
          "op": "castSpell",
          "name": "Unhallowed Pact",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "@dw"
        },
        {
          "op": "assertStackObject",
          "source": "Unhallowed Pact",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Treacherous Pit-Dweller"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "\"Unhallowed Pact\"",
          "targetPlayer": 1,
          "target": "@dw",
          "expected": true
        },
        {
          "op": "castSpell",
          "name": "Lightning Bolt",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "target": "@dw"
        },
        {
          "op": "assertStackObject",
          "source": "Lightning Bolt",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Treacherous Pit-Dweller"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Lightning Bolt",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "target": "@dw"
        },
        {
          "op": "assertStackObject",
          "source": "Lightning Bolt",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Treacherous Pit-Dweller"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@dw",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Unhallowed Pact",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Treacherous Pit-Dweller",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Triggered",
          "completeTargets": true,
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Unhallowed Pact",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@dw",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@dw"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "@dw",
          "count": 0,
          "counter": "+1/+1"
        }
      ]
    }
  ]
});
