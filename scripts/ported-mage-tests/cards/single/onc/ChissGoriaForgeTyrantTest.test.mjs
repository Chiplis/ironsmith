import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/onc/ChissGoriaForgeTyrantTest.java",
  "tests": [
    {
      "name": "testCastArtifact",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 15
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Marble Chalice",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chiss-Goria, Forge Tyrant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Blightsteel Colossus@colossus",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chiss-Goria, Forge Tyrant",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Chiss-Goria, Forge Tyrant",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Mountain",
          "tapped": true,
          "count": 3
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Chiss-Goria, Forge Tyrant",
          "defender": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "@colossus",
          "count": 1
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "zone": "exile",
          "name": "Blightsteel Colossus",
          "objectAlias": "@colossus",
          "methods": [
            "play_from"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Blightsteel Colossus",
          "holdStack": true,
          "permissionSource": "Chiss-Goria, Forge Tyrant",
          "objectAlias": "@colossus"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Blightsteel Colossus",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Mountain",
          "tapped": true,
          "count": 9
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blightsteel Colossus",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Chiss-Goria, Forge Tyrant",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 9
        }
      ]
    }
  ]
});
