import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/eld/RobberOfTheRichTest.java",
  "tests": [
    {
      "name": "testRobber",
      "operations": [
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
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
          "player": 0,
          "name": "Robber of the Rich",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Whitesun's Passage@passage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "@passage",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Robber of the Rich",
          "defender": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "source": "Robber of the Rich",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertExileCount",
          "name": "@passage",
          "player": 1,
          "count": 1
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "zone": "exile",
          "owner": 1,
          "name": "Whitesun's Passage",
          "objectAlias": "@passage",
          "methods": [
            "play_from"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Whitesun's Passage",
          "holdStack": true,
          "objectAlias": "@passage",
          "permissionSource": "Robber of the Rich",
          "castingMethod": "play_from"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Whitesun's Passage",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "life": 25
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "@passage",
          "count": 1
        }
      ]
    },
    {
      "name": "testRobberWithOtherRogue",
      "operations": [
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
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
          "player": 0,
          "name": "Robber of the Rich",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bane Alley Blackguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Whitesun's Passage",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Whitesun's Passage@passage",
          "count": 1,
          "note": "Added last so the aliased copy is the library top that Robber exiles (CR 401.2: the source adds five identical copies; only the top one is exiled)."
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "@passage",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Robber of the Rich",
          "defender": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "source": "Robber of the Rich",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertExileCount",
          "name": "@passage",
          "player": 1,
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Bane Alley Blackguard",
          "defender": 1
        },
        {
          "op": "assertCastMethods",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "zone": "exile",
          "owner": 1,
          "name": "Whitesun's Passage",
          "objectAlias": "@passage",
          "methods": [
            "play_from"
          ]
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Whitesun's Passage",
          "holdStack": true,
          "objectAlias": "@passage",
          "permissionSource": "Robber of the Rich",
          "castingMethod": "play_from"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Whitesun's Passage",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 25
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "@passage",
          "count": 1
        }
      ]
    }
  ]
});
