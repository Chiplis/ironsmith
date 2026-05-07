import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/PhantasmalImageTest.java",
  "tests": [
    {
      "name": "testCopyCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Craw Wurm"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
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
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        }
      ]
    },
    {
      "name": "testCopyEntersBattlefieldTriggeredAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Howling Banshee",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Howling Banshee"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Howling Banshee",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Howling Banshee",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    },
    {
      "name": "testCopyCreatureWithLevelUpAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Transcendent Master",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 12
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Transcendent Master"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Transcendent Master",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Transcendent Master",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertEquals(\"Power different\", 9, master.getPower().getValue())"
        },
        {
          "op": "unsupported",
          "source": "assertEquals(\"Toughness different\", 9, master.getToughness().getValue())"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(master.getAbilities().contains(LifelinkAbility.getInstance()))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(master.getAbilities().containsRule(IndestructibleAbility.getInstance()))"
        },
        {
          "op": "unsupported",
          "source": "assertEquals(\"Power different\", 3, masterCopied.getPower().getValue())"
        },
        {
          "op": "unsupported",
          "source": "assertEquals(\"Toughness different\", 3, masterCopied.getToughness().getValue())"
        },
        {
          "op": "unsupported",
          "source": "assertFalse(masterCopied.getAbilities().contains(LifelinkAbility.getInstance()))"
        },
        {
          "op": "unsupported",
          "source": "assertFalse(masterCopied.getAbilities().containsRule(IndestructibleAbility.getInstance()))"
        }
      ]
    },
    {
      "name": "testCopyBecomesTargetTriggeredAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Illusionary Servant",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Illusionary Servant"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Illusionary Servant-M10"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Illusionary Servant",
          "count": 3
        }
      ]
    },
    {
      "name": "testCopyAlreadyTransformed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Titanic Growth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Asceticism",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Huntmaster of the Fells",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Ravager of the Fells"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Asceticism"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Titanic Growth",
          "target": "Ravager of the Fells"
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
          "player": 1,
          "life": 18
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ravager of the Fells",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Titanic Growth",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ravager of the Fells",
          "count": 0
        }
      ]
    },
    {
      "name": "testCopyEntersTappedAndEtb",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Geralf's Messenger",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Geralf's Messenger"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    },
    {
      "name": "testCopyAsEntersBattlefieldAbility",
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
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lurebound Scarecrow",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lurebound Scarecrow"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Green"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lurebound Scarecrow"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Red"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lurebound Scarecrow",
          "count": 1
        }
      ]
    },
    {
      "name": "testCopyAsEntersBattlefieldAbility2",
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
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lurebound Scarecrow",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lurebound Scarecrow"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Green"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lurebound Scarecrow"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Red"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lurebound Scarecrow",
          "count": 2
        }
      ]
    },
    {
      "name": "testCopiedFlyingWorks",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fervor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Azure Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Azure Drake"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Azure Drake",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Llanowar Elves",
          "attacker": "Azure Drake"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Azure Drake",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Elite Vanguard",
          "attacker": "Azure Drake"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        }
      ]
    },
    {
      "name": "testCopiedSteelHellkite",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Steel Hellkite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chalice of the Void",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chalice of the Void"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=0"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Steel Hellkite"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Steel Hellkite",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{X}:"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "X=0"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Steel Hellkite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Steel Hellkite",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 15
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Chalice of the Void",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Chalice of the Void",
          "count": 1
        }
      ]
    },
    {
      "name": "testCopiedFrostTitan",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Frost Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Veil of Secrecy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Veil of Secrecy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Frost Titan"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Island"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": 0,
          "count": "Frost Titan"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": 1,
          "count": "Frost Titan"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Veil of Secrecy"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Frost Titan[no copy]"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Veil of Secrecy"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Frost Titan[only copy]"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "When {this} becomes the target of a spell or ability, sacrifice it."
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Frost Titan",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Frost Titan",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": true,
          "count": "2 + 2 * 2 + 2 * 2"
        }
      ]
    },
    {
      "name": "testDiesTriggeredAbilities",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wurmcoil Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Public Execution",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Wurmcoil Engine"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Public Execution"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Wurmcoil Engine[only copy]"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Public Execution",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wurmcoil Engine",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Phyrexian Wurm Token",
          "count": 2
        }
      ]
    },
    {
      "name": "testLeavesTheBattlefieldTriggeredAbilities",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thalakos Seer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Public Execution",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Thalakos Seer"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Public Execution"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Thalakos Seer[only copy]"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Public Execution",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Thalakos Seer",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 2
        }
      ]
    },
    {
      "name": "testPersist",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kitchen Finks",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Public Execution",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Kitchen Finks"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Kitchen Finks"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Public Execution"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Kitchen Finks[only copy]"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Kitchen Finks"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Public Execution",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 24
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kitchen Finks",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kitchen Finks",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Kitchen Finks",
          "power": 2,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testUndying",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Butcher Ghoul",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Public Execution",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Butcher Ghoul"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Butcher Ghoul"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Public Execution"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Butcher Ghoul[only copy]"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Butcher Ghoul"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Public Execution",
          "count": 1
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
          "name": "Butcher Ghoul",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Phantasmal Image",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Butcher Ghoul",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Butcher Ghoul",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testDiesTriggered2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wurmcoil Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Wurmcoil Engine"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Wurmcoil Engine",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Wurmcoil Engine",
          "attacker": "Wurmcoil Engine"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 26
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 26
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Wurmcoil Engine",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phyrexian Wurm Token",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Phyrexian Wurm Token",
          "count": 2
        }
      ]
    },
    {
      "name": "testVoiceOfResurgence",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Voice of Resurgence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Voice of Resurgence"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Voice of Resurgence",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Voice of Resurgence",
          "attacker": "Voice of Resurgence"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Voice of Resurgence",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Elemental Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elemental Token",
          "count": 1
        }
      ]
    },
    {
      "name": "testAnimatedArtifact",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Chimeric Staff",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{X}"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "X=1"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Chimeric Staff"
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
          "source": "assertTrue(\"Phantasmal Image should be an artifact\", staffA.isArtifact(currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Phantasmal Image should not be a creature\", !staffA.isCreature(currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Phantasmal Image should not be an Illusion\", !staffA.hasSubtype(SubType.ILLUSION, currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Phantasmal Image should not be a Construct\", !staffA.hasSubtype(SubType.CONSTRUCT, currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Phantasmal Image should have the sacrifice trigger\", staffA.getAbilities(currentGame).containsClass(BecomesTargetSourceTriggeredAbility.class))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Chimeric Staff should be an artifact\", staffB.isArtifact(currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Chimeric Staff should be a creature\", staffB.isCreature(currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Chimeric Staff should be a Construct\", staffB.hasSubtype(SubType.CONSTRUCT, currentGame))"
        }
      ]
    },
    {
      "name": "testAnimatedKindred",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Cloak and Dagger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Karn's Touch",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Karn's Touch",
          "target": "Cloak and Dagger"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cloak and Dagger"
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
          "source": "assertTrue(\"Phantasmal Image should be an artifact\", cloakA.isArtifact(currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Phantasmal Image should be kindred\", cloakA.isKindred(currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Phantasmal Image should not be a creature\", !cloakA.isCreature(currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Phantasmal Image should be a Rogue\", cloakA.hasSubtype(SubType.ROGUE, currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Phantasmal Image should be an Illusion\", cloakA.hasSubtype(SubType.ILLUSION, currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Phantasmal Image should be an Equipment\", cloakA.hasSubtype(SubType.EQUIPMENT, currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Phantasmal Image should have the sacrifice trigger\", cloakA.getAbilities(currentGame).containsClass(BecomesTargetSourceTriggeredAbility.class))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Cloak and Dagger should be an artifact\", cloakB.isArtifact(currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Cloak and Dagger should be a creature\", cloakB.isCreature(currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Cloak and Dagger should be kindred\", cloakB.isKindred(currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Cloak and Dagger should be a Rogue\", cloakB.hasSubtype(SubType.ROGUE, currentGame))"
        },
        {
          "op": "unsupported",
          "source": "assertTrue(\"Cloak and Dagger should be an Equipment\", cloakB.hasSubtype(SubType.EQUIPMENT, currentGame))"
        }
      ]
    },
    {
      "name": "test_SelfExploit_SidisiUndeadVizier_Normal",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sidisi, Undead Vizier",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Sidisi, Undead Vizier"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Sidisi, Undead Vizier[only copy]"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Sidisi, Undead Vizier[only copy]"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mountain"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Mountain",
          "count": 1
        }
      ]
    },
    {
      "name": "test_SelfExploit_SidisiUndeadVizier_Exile",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Image",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sidisi, Undead Vizier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rest in Peace",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phantasmal Image"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Sidisi, Undead Vizier"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Sidisi, Undead Vizier[only copy]"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Sidisi, Undead Vizier[only copy]"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mountain"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Mountain",
          "count": 1
        }
      ]
    }
  ]
});
