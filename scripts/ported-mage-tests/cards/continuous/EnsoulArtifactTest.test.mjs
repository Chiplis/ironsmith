import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/EnsoulArtifactTest.java",
  "tests": [
    {
      "name": "test_Boost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Citadel",
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
          "name": "Ensoul Artifact",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ensoul Artifact",
          "target": "Darksteel Citadel"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Darksteel Citadel",
          "ability": "Indestructible",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Darksteel Citadel\", CardType.CREATURE, true)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Darksteel Citadel",
          "power": 5,
          "toughness": 5
        }
      ]
    },
    {
      "name": "test_BoostDisappearedOnBlink",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Citadel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Momentary Blink",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ensoul Artifact",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ensoul Artifact",
          "target": "Darksteel Citadel"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Momentary Blink",
          "target": "Darksteel Citadel"
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
          "name": "Momentary Blink",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Citadel",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Darksteel Citadel",
          "power": 0,
          "toughness": 0
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Darksteel Citadel\", CardType.CREATURE, false)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Darksteel Citadel",
          "ability": "Indestructible",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ensoul Artifact",
          "count": 0
        }
      ]
    }
  ]
});
