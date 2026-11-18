import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c17/CurseOfOpulenceTest.java",
  "tests": [
    {
      "name": "controllerAttacks",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curse of Opulence",
          "count": 1
        },
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
          "name": "Banehound",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of Opulence",
          "target": 3
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Curse of Opulence",
          "attachedPlayer": 3,
          "expected": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Banehound",
          "defender": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "source": "Curse of Opulence",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
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
          "player": 3,
          "life": 19
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gold Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "Gold Token",
          "count": 0,
          "player": 3
        },
        {
          "op": "assertPermanentCount",
          "name": "Gold Token",
          "count": 0,
          "player": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gold Token",
          "count": 0
        }
      ]
    },
    {
      "name": "opponentAttacks",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curse of Opulence",
          "count": 1
        },
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
          "player": 1,
          "name": "Banehound",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of Opulence",
          "target": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Curse of Opulence",
          "attachedPlayer": 0,
          "expected": true
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Banehound",
          "defender": 0
        },
        {
          "op": "assertStackObject",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS",
          "source": "Curse of Opulence",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS"
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
          "name": "Gold Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "Gold Token",
          "count": 0,
          "player": 3
        },
        {
          "op": "assertPermanentCount",
          "name": "Gold Token",
          "count": 0,
          "player": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gold Token",
          "count": 1
        }
      ]
    },
    {
      "name": "rangeOfInfluence",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curse of Opulence",
          "count": 1
        },
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
          "player": 2,
          "name": "Banehound",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of Opulence",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Curse of Opulence",
          "attachedPlayer": 1,
          "expected": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 2,
          "attacker": "Banehound",
          "defender": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "source": "Curse of Opulence",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS"
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
          "life": 19
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gold Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "Gold Token",
          "count": 0,
          "player": 3
        },
        {
          "op": "assertPermanentCount",
          "name": "Gold Token",
          "count": 0,
          "player": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gold Token",
          "count": 0
        }
      ]
    },
    {
      "name": "doesNotTriggerMoreThanOnce",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curse of Opulence",
          "count": 1
        },
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
          "name": "Banehound",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of Opulence",
          "target": 3
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Curse of Opulence",
          "attachedPlayer": 3,
          "expected": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Banehound",
          "defender": 3
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Banehound",
          "defender": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "source": "Curse of Opulence",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
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
          "player": 3,
          "life": 18
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gold Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "Gold Token",
          "count": 0,
          "player": 3
        },
        {
          "op": "assertPermanentCount",
          "name": "Gold Token",
          "count": 0,
          "player": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gold Token",
          "count": 0
        }
      ]
    }
  ],
  "turnOrder": [
    0,
    3,
    2,
    1
  ],
  "rangeOfInfluence": 1
});
