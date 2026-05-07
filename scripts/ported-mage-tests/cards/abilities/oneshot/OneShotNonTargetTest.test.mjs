import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/OneShotNonTargetTest.java",
  "tests": [
    {
      "name": "YorionChooseAfterTriggerTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Yorion, Sky Nomad",
          "count": 1
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
          "name": "Resolute Reinforcements",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Yorion, Sky Nomad"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null,
          "once": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Yorion, Sky Nomad",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Resolute Reinforcements"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Resolute Reinforcements"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_SKIP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "unsupported",
          "source": "assertTokenCount(playerA, \"Soldier Token\", 2)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Yorion, Sky Nomad",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Resolute Reinforcements",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Plains",
          "tapped": true,
          "count": 7
        }
      ]
    },
    {
      "name": "NonTargetAdjusterTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Temporal Firestorm",
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
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Python",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Watchwolf",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Temporal Firestorm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Squire^Python"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Squire",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Python",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Watchwolf",
          "count": 1
        }
      ]
    },
    {
      "name": "ModeSelectionTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "SOLDIER Military Program",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "SOLDIER Military Program"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "2"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Squire"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "2"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Squire"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Soldier Token"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Squire",
          "power": 3,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Soldier Token",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "MuseVesselTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Muse Vessel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 30
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vizier of Tumbling Sands",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Alpha Myr",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Void Snare",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Corridor Monitor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Muse Vessel"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}, {T}: Target player",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Island"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Untap",
          "target": "Muse Vessel"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}, {T}: Target player",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Corridor Monitor"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Untap",
          "target": "Muse Vessel"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}, {T}: Target player",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Squire"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}: Choose"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Island"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}: Choose"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Squire"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Squire"
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Island"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Void Snare",
          "target": "Muse Vessel"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Muse Vessel"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{1}: Choose"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{3}, {T}: Target player",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Alpha Myr"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{1}: Choose"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Alpha Myr"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Alpha Myr",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Alpha Myr",
          "expected": false
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "{1}: Choose"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Alpha Myr"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Alpha Myr"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Alpha Myr",
          "count": 1
        }
      ]
    }
  ]
});
