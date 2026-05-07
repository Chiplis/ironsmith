import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/EvolveTest.java",
  "tests": [
    {
      "name": "testCreatureComesIntoPlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cloudfin Raptor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mindeye Drake",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mindeye Drake"
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
          "player": 0,
          "name": "Cloudfin Raptor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mindeye Drake",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Cloudfin Raptor",
          "power": 1,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Mindeye Drake",
          "power": 2,
          "toughness": 5
        }
      ]
    },
    {
      "name": "testCreatureComesIntoPlayNoCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Experiment One",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kird Ape",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Kird Ape"
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
          "player": 0,
          "name": "Experiment One",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kird Ape",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Experiment One",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kird Ape",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testCreatureComesStrongerIntoPlayCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Experiment One",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kird Ape",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Kird Ape"
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
          "player": 0,
          "name": "Experiment One",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kird Ape",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Experiment One",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kird Ape",
          "power": 2,
          "toughness": 3
        }
      ]
    },
    {
      "name": "testEvolveWithMasterBiomance",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Experiment One",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Master Biomancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Experiment One",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Experiment One"
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
          "player": 0,
          "name": "Experiment One",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Master Biomancer",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Experiment One",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Experiment One",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "testMultipleCreaturesComeIntoPlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Judge's Familiar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cloudfin Raptor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mizzium Mortars",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Banisher Priest",
          "count": 2
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Banisher Priest"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Cloudfin Raptor"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Banisher Priest"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Judge's Familiar"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mizzium Mortars with overload"
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
          "name": "Banisher Priest",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Banisher Priest",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cloudfin Raptor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Judge's Familiar",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Cloudfin Raptor",
          "power": 1,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testMultipleCreaturesComeIntoPlaySuddenDisappearance",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Battering Krasis",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Crocanura",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Sudden Disappearance",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Sudden Disappearance",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Evolve"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Battering Krasis",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Crocanura",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Battering Krasis",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Crocanura",
          "power": 2,
          "toughness": 4
        }
      ]
    },
    {
      "name": "testRenegadeKrasis",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Renegade Krasis",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 16
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ivy Lane Denizen",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Adaptive Snapjaw",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ivy Lane Denizen"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Renegade Krasis"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ivy Lane Denizen"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ivy Lane Denizen"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Renegade Krasis"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever another green creature"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Adaptive Snapjaw"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Adaptive Snapjaw"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Adaptive Snapjaw"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Evolve"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Renegade Krasis",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ivy Lane Denizen",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Adaptive Snapjaw",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Adaptive Snapjaw",
          "power": 9,
          "toughness": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Renegade Krasis",
          "power": 6,
          "toughness": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ivy Lane Denizen",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ivy Lane Denizen",
          "power": 5,
          "toughness": 6
        }
      ]
    }
  ]
});
