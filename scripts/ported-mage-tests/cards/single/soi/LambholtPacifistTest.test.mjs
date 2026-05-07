import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/soi/LambholtPacifistTest.java",
  "tests": [
    {
      "name": "uncrewedVehicle_LambholtCannotAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lambholt Pacifist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Heart of Kiran",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lambholt Pacifist"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Ornithopter"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Lambholt Pacifist",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); } catch (Throwable e) { if (!e.getMessage().contains(\"Player PlayerA must have 0 actions but found 1\")) { Assert.fail(\"Should have had error about not being able to attack, but got:\\n\" + e.getMessage()); } } assertPermanentCount(playerB, orni, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(lambholt, false)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    }
  ]
});
