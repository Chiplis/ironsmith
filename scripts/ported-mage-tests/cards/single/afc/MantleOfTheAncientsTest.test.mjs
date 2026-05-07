import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/afc/MantleOfTheAncientsTest.java",
  "tests": [
    {
      "name": "testCardReturnsCorrectAttachments",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mantle of the Ancients",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Skylasher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grim Guardian",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Konda's Banner",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "O-Naginata",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Aether Tunnel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Reprobation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Indestructibility",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Abundant Growth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mantle of the Ancients",
          "target": "Skylasher"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "<i>Constellation"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Konda's Banner^O-Naginata^Aether Tunnel^Reprobation^Indestructibility^Abundant Growth"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "<i>Constellation"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Gate Smasher",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Aether Tunnel",
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mantle of the Ancients",
          "target": "Skylasher"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "<i>Constellation</i>"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Gate Smasher^Aether Tunnel"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
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
          "op": "unsupported",
          "source": "assertAttachedTo(playerA, \"Konda's Banner\", creature, false)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Konda's Banner",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertAttachedTo(playerA, \"O-Naginata\", creature, true)"
        },
        {
          "op": "unsupported",
          "source": "assertAttachedTo(playerA, \"Aether Tunnel\", creature, true)"
        },
        {
          "op": "unsupported",
          "source": "assertAttachedTo(playerA, \"Reprobation\", creature, true)"
        },
        {
          "op": "unsupported",
          "source": "assertAttachedTo(playerA, \"Indestructibility\", creature, true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Abundant Growth",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mantle of the Ancients",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Skylasher",
          "power": 16,
          "toughness": 13
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    }
  ]
});
