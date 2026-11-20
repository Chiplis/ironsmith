import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/snc/VampireScrivenerTest.java",
  "tests": [
    {
      "name": "test_LoseLife_Twice",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vampire Scrivener",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Battlefield Forge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {R}"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 0
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
          "life": 16
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vampire Scrivener",
          "counter": "P1P1",
          "count": 2
        }
      ]
    },
    {
      "name": "test_RakdosCharm",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vampire Scrivener",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kobolds of Kher Keep",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Badlands",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rakdos Charm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rakdos Charm",
          "holdStack": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "modes",
          "description": "Choose mode for Rakdos Charm",
          "choices": [
            "Each creature deals 1 damage to its controller."
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Rakdos Charm",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Vampire Scrivener",
          "note": "CR 120.3a, 119.9 and 702.15e: damage from each creature is a separate life-loss event (Wizards ruling on Vampire Scrivener: it triggers once per life-losing event; each damage source makes its own event), so four creatures dealing 1 damage each yield four triggers and four counters; XMage batches them into one.",
          "count": 4,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "life": 16
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vampire Scrivener",
          "note": "CR 120.3a, 119.9 and 702.15e: damage from each creature is a separate life-loss event (Wizards ruling on Vampire Scrivener: it triggers once per life-losing event; each damage source makes its own event), so four creatures dealing 1 damage each yield four triggers and four counters; XMage batches them into one.",
          "counter": "P1P1",
          "count": 4
        }
      ]
    },
    {
      "name": "test_RakdosCharm_NotYourTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vampire Scrivener",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kobolds of Kher Keep",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Badlands",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rakdos Charm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rakdos Charm",
          "holdStack": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "modes",
          "description": "Choose mode for Rakdos Charm",
          "choices": [
            "Each creature deals 1 damage to its controller."
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Rakdos Charm",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Vampire Scrivener",
          "count": 0,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 16
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vampire Scrivener",
          "counter": "P1P1",
          "count": 0
        }
      ]
    }
  ]
});
