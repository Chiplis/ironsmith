import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/delayed/NettlingImplTest.java",
  "tests": [
    {
      "name": "testForcedDidAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nettling Imp",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Choose",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nettling Imp",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testForcedDidNotAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nettling Imp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pressure Point",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Choose",
          "target": "Silvercoat Lion",
          "source": "Nettling Imp"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Nettling Imp",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "Silvercoat Lion"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Pressure Point",
          "target": "Silvercoat Lion",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "source": "Pressure Point",
          "count": 1,
          "isSpell": true,
          "objectTarget": "Silvercoat Lion"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertTappedCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1,
          "tapped": true
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "END_TURN",
          "source": "Nettling Imp",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_TURN"
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
          "name": "Nettling Imp",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Pressure Point",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
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
        }
      ]
    }
  ]
});
