import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/enters/BanisherPriestTest.java",
  "tests": [
    {
      "name": "testDoNotExileIfBanisherPriestLeavesBattlefieldBeforeResolve",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Banisher Priest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Incinerate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Rockslide Elemental",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Banisher Priest"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Rockslide Elemental"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Incinerate",
          "target": "Banisher Priest"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
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
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Rockslide Elemental",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Banisher Priest",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Rockslide Elemental",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testReturningTargetDoesNotTriggerDieEventOfBanisherPriest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Banisher Priest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Incinerate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Rockslide Elemental",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Banisher Priest"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Rockslide Elemental"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Incinerate",
          "target": "Banisher Priest"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "count": 0
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
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Rockslide Elemental",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Banisher Priest",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Rockslide Elemental",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testBanisherPriestToken",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Banisher Priest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Seance",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Yes"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Banisher Priest"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Banisher Priest",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Banisher Priest",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    }
  ]
});
