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
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bear Cub"
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
