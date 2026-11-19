import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/TheSpotLivingPortalTest.java",
  "tests": [
    {
      "name": "testTheSpotLivingPortal",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addCustomEffect_TargetDestroy(playerB)"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Spot, Living Portal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scrubland",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Spot, Living Portal"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bear Cub"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Fugitive Wizard"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "target destroy"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "The Spot, Living Portal"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "The Spot, Living Portal",
          "count": 1
        }
      ]
    },
    {
      "name": "testOnlyGraveyard",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Murder"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Spot, Living Portal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scrubland",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Spot, Living Portal",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Spot, Living Portal",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [],
              [
                "Bear Cub"
              ]
            ]
          }
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Spot, Living Portal",
          "count": 1,
          "abilityKind": "Triggered",
          "completeTargets": true,
          "objectTarget": "Bear Cub"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Murder",
          "holdStack": true,
          "target": "The Spot, Living Portal"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Murder",
          "count": 1,
          "isSpell": true,
          "objectTarget": "The Spot, Living Portal"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "source": "The Spot, Living Portal",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "The Spot, Living Portal",
          "count": 1
        }
      ]
    },
    {
      "name": "testOnlyBattlefield",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addCustomEffect_TargetDestroy(playerB)"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Spot, Living Portal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scrubland",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Spot, Living Portal"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Fugitive Wizard"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "target destroy"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "The Spot, Living Portal"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "The Spot, Living Portal",
          "count": 1
        }
      ]
    }
  ]
});
