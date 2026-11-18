import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dka/HollowhengeSpiritTest.java",
  "tests": [
    {
      "name": "testCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hollowhenge Spirit",
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
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Craw Wurm",
          "defender": 0
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "name": "Hollowhenge Spirit"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "source": "Hollowhenge Spirit",
          "count": 1,
          "onTop": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Craw Wurm"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "source": "Hollowhenge Spirit",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "objectTarget": "Craw Wurm"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
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
          "name": "Hollowhenge Spirit",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Hollowhenge Spirit",
          "tapped": false,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 1,
          "name": "Craw Wurm",
          "tapped": true,
          "count": 1
        }
      ]
    },
    {
      "name": "testCard1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hollowhenge Spirit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Craw Wurm",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "name": "Hollowhenge Spirit"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "source": "Hollowhenge Spirit",
          "count": 1,
          "onTop": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Craw Wurm"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "source": "Hollowhenge Spirit",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "objectTarget": "Craw Wurm"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
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
          "player": 0,
          "name": "Hollowhenge Spirit",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Hollowhenge Spirit",
          "tapped": false,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Craw Wurm",
          "tapped": true,
          "count": 1
        }
      ]
    },
    {
      "name": "testCard2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hollowhenge Spirit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hollowhenge Spirit"
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
          "player": 0,
          "name": "Hollowhenge Spirit",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Craw Wurm\", false)"
        }
      ]
    }
  ]
});
