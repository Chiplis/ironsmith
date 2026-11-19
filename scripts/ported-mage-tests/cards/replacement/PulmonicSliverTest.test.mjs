import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/replacement/PulmonicSliverTest.java",
  "tests": [
    {
      "name": "testKillSpellOnOtherSliver",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pulmonic Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Venom Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Doom Blade"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Venom Sliver"
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Venom Sliver",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Venom Sliver",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Venom Sliver",
          "count": 1
        }
      ]
    },
    {
      "name": "testDamnationOnSlivers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pulmonic Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Venom Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Damnation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Damnation",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Damnation",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "setOptionChoices",
          "kind": "replacement_effect",
          "description": "Choose which replacement effect to apply",
          "choices": [
            "Pulmonic Sliver"
          ],
          "player": 0
        },
        {
          "op": "setOptionChoices",
          "kind": "replacement_effect",
          "description": "Choose which replacement effect to apply",
          "choices": [
            "Venom Sliver"
          ],
          "player": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Venom Sliver",
          "count": 0
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Damnation",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Venom Sliver",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Venom Sliver",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Pulmonic Sliver",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Pulmonic Sliver",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Pulmonic Sliver",
          "count": 1
        }
      ]
    },
    {
      "name": "testExileOnSliver",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pulmonic Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Venom Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Path to Exile",
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
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Path to Exile"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Venom Sliver"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Path to Exile",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Venom Sliver",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Venom Sliver",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Venom Sliver",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "name": "Venom Sliver",
          "count": 1
        }
      ]
    }
  ]
});
