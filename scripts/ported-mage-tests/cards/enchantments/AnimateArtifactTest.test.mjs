import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/enchantments/AnimateArtifactTest.java",
  "tests": [
    {
      "name": "testAnimateArtifact",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Crucible of Worlds",
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
          "name": "Animate Artifact",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Animate Artifact",
          "target": "Crucible of Worlds"
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
          "name": "Crucible of Worlds",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Animate Artifact",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Crucible of Worlds\", CardType.CREATURE, null)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Crucible of Worlds",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "testAnimateArtifactCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Juggernaut",
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
          "name": "Animate Artifact",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Animate Artifact",
          "target": "Juggernaut"
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
          "name": "Juggernaut",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Animate Artifact",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Juggernaut\", CardType.CREATURE, SubType.JUGGERNAUT)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Juggernaut",
          "power": 5,
          "toughness": 3
        }
      ]
    }
  ]
});
