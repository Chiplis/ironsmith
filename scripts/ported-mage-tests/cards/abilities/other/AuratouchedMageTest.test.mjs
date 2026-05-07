import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/AuratouchedMageTest.java",
  "tests": [
    {
      "name": "testAuratouchedMageEffectHasMadeIntoTypeArtifact",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Auratouched Mage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Argent Mutation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Relic Ward",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Auratouched Mage"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"prepare etb\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"When {this} enters\", 1)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Argent Mutation",
          "target": "Auratouched Mage"
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"prepare artifact\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"When {this} enters\", 1)"
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"prepare artifact\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Cast Argent Mutation\", 1)"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Relic Ward"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Auratouched Mage",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Relic Ward",
          "count": 1
        }
      ]
    },
    {
      "name": "testGainsLegalAura",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Auratouched Mage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Brainwash",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Auratouched Mage"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Brainwash"
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
          "name": "Auratouched Mage",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Brainwash",
          "count": 1
        }
      ]
    },
    {
      "name": "testAuratouchedMageNotOnBattlefield",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Auratouched Mage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Swords to Plowshares",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Brainwash",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Auratouched Mage"
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
          "player": 0,
          "name": "Swords to Plowshares",
          "target": "Auratouched Mage"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Brainwash"
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
          "name": "Auratouched Mage",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Brainwash",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Brainwash",
          "count": 1
        }
      ]
    }
  ]
});
