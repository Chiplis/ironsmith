import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/UlamogTheInfiniteGyreTest.java",
  "tests": [
    {
      "name": "testDisabledEffectOnChangeZone",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ulamog, the Infinite Gyre",
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
          "name": "Counterspell",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Ulamog, the Infinite Gyre",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Island"
        },
        {
          "op": "assertStackObject",
          "source": "Ulamog, the Infinite Gyre",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered",
          "objectTarget": "Island",
          "completeTargets": true
        },
        {
          "op": "castSpell",
          "name": "Counterspell",
          "player": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Ulamog, the Infinite Gyre"
        },
        {
          "op": "assertStackObject",
          "source": "Counterspell",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Ulamog, the Infinite Gyre"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Ulamog, the Infinite Gyre",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered",
          "objectTarget": "Island"
        },
        {
          "op": "assertStackObject",
          "source": "Ulamog, the Infinite Gyre",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Ulamog, the Infinite Gyre",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ulamog, the Infinite Gyre",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Counterspell",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Island",
          "count": 1
        }
      ]
    },
    {
      "name": "testControlledByOtherPlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 15
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kozilek, Butcher of Truth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unholy Hunger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Control Magic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Kozilek, Butcher of Truth",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Kozilek, Butcher of Truth",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "count": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": null,
          "count": 5
        },
        {
          "op": "castSpell",
          "name": "Control Magic",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "target": "Kozilek, Butcher of Truth"
        },
        {
          "op": "assertStackObject",
          "source": "Control Magic",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Kozilek, Butcher of Truth"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kozilek, Butcher of Truth",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kozilek, Butcher of Truth",
          "count": 0
        },
        {
          "op": "castSpell",
          "name": "Unholy Hunger",
          "player": 0,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "target": "Kozilek, Butcher of Truth"
        },
        {
          "op": "assertStackObject",
          "source": "Unholy Hunger",
          "count": 1,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "objectTarget": "Kozilek, Butcher of Truth"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Kozilek, Butcher of Truth",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Kozilek, Butcher of Truth",
          "count": 1,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Triggered",
          "controller": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kozilek, Butcher of Truth",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kozilek, Butcher of Truth",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Control Magic",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Control Magic",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Kozilek, Butcher of Truth",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Kozilek, Butcher of Truth",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Unholy Hunger",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 4
        }
      ]
    },
    {
      "name": "testFaceDownToGraveyard",
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
          "name": "Kozilek, Butcher of Truth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ixidron",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Wrath of God",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Ixidron",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Ixidron",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertFaceDownPermanentView",
          "player": 0,
          "viewer": 0,
          "visible": true,
          "originalName": "Kozilek, Butcher of Truth"
        },
        {
          "op": "castSpell",
          "name": "Wrath of God",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Wrath of God",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Kozilek, Butcher of Truth",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ixidron",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Kozilek, Butcher of Truth",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered",
          "controller": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ixidron",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Wrath of God",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Kozilek, Butcher of Truth",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Ixidron",
          "count": 1
        }
      ]
    }
  ]
});
