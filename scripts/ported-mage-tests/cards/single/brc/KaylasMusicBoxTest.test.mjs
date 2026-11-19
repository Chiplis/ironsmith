import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/brc/KaylasMusicBoxTest.java",
  "tests": [
    {
      "name": "testEffect",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "name": "Silvercoat Lion",
          "zone": "LIBRARY",
          "player": 0,
          "count": 1
        },
        {
          "op": "addCard",
          "name": "Mountain",
          "zone": "LIBRARY",
          "player": 0,
          "count": 1
        },
        {
          "op": "addCard",
          "name": "Plains@land",
          "zone": "LIBRARY",
          "player": 0,
          "count": 1
        },
        {
          "op": "addCard",
          "name": "Mountain",
          "zone": "LIBRARY",
          "player": 0,
          "count": 1
        },
        {
          "op": "addCard",
          "name": "Silvercoat Lion@lion",
          "zone": "LIBRARY",
          "player": 0,
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kayla's Music Box",
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "ability": "Look at the top",
          "source": "Kayla's Music Box"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "END_TURN",
          "source": "Kayla's Music Box",
          "abilityKind": "Activated",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "@lion",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Look at the top",
          "source": "Kayla's Music Box"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Kayla's Music Box",
          "abilityKind": "Activated",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "@land",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "you may play",
          "source": "Kayla's Music Box"
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "playLand",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "@land"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion",
          "objectAlias": "@lion"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Plains",
          "count": 2
        }
      ]
    }
  ]
});
