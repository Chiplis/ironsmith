import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/MasterThiefTest.java",
  "tests": [
    {
      "name": "testMasterThief_GetControlOnEnterBattlefield",
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
          "zone": "HAND",
          "player": 0,
          "name": "Master Thief",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Accorder's Shield",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Master Thief"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Master Thief",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Accorder's Shield",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 0
        }
      ]
    },
    {
      "name": "testMasterThief_LostControlOnSacrifice",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Master Thief",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Accorder's Shield",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bearer of the Heavens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ashnod's Altar",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Master Thief"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Accorder's Shield"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice a creature"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Master Thief"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Island",
          "count": 10
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Master Thief",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Accorder's Shield",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ashnod's Altar",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bearer of the Heavens",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bearer of the Heavens",
          "power": 10,
          "toughness": 10
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Accorder's Shield",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 1
        }
      ]
    },
    {
      "name": "testMasterThief_LostControlOnSacrificeButArtifactAttached",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Master Thief",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bearer of the Heavens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ashnod's Altar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Accorder's Shield",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Master Thief"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Accorder's Shield"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Accorder's Shield",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Equip {3}"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bearer of the Heavens"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "END_TURN",
          "ability": 0,
          "expected": "Bearer of the Heavens"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice a creature"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Master Thief"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Island",
          "count": 10
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Master Thief",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Accorder's Shield",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ashnod's Altar",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bearer of the Heavens",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bearer of the Heavens",
          "power": 10,
          "toughness": 13
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Accorder's Shield",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 1
        }
      ]
    }
  ]
});
