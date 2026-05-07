import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/WardTest.java",
  "tests": [
    {
      "name": "wardMultipleAbilities",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Solitude",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Healer's Hawk",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Waterfall Aerialist",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Solitude"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Evoke alternative cost: Exile a white card from your hand (source: Solitude"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Healer's Hawk"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When {this} enters, exile up to one other target creature"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Waterfall Aerialist"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "No"
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Healer's Hawk",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Solitude",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Waterfall Aerialist",
          "count": 1
        }
      ]
    },
    {
      "name": "wardPanharmonicon",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Panharmonicon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Young Red Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Roaming Throne",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Scourge of Valkas",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Dragon"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Scourge of Valkas"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever {this} or another Dragon"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Roaming Throne"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Roaming Throne"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "ward {2}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "No"
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
          "player": 1,
          "name": "Roaming Throne",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Roaming Throne\", 2)"
        }
      ]
    },
    {
      "name": "wardPanharmoniconCounter",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Panharmonicon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Young Red Dragon",
          "count": 1
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
          "player": 1,
          "name": "Roaming Throne",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Scourge of Valkas",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Dragon"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Scourge of Valkas"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever {this} or another Dragon"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Roaming Throne"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Roaming Throne"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "ward {2}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "No"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "No"
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
          "player": 1,
          "name": "Roaming Throne",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Roaming Throne\", 0)"
        }
      ]
    },
    {
      "name": "wardRecastAuraNoPay",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Owlin Shieldmage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kenrith's Transformation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Beast Within",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Regrowth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Kenrith's Transformation",
          "target": "Owlin Shieldmage"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
          "name": "Regrowth",
          "target": "Kenrith's Transformation"
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
          "name": "Kenrith's Transformation",
          "target": "Owlin Shieldmage"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Kenrith's Transformation",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Regrowth",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Beast Within",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Owlin Shieldmage",
          "ability": "Flying",
          "expected": true
        }
      ]
    },
    {
      "name": "wardRecastAuraPaySecond",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Owlin Shieldmage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kenrith's Transformation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Beast Within",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Regrowth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Kenrith's Transformation",
          "target": "Owlin Shieldmage"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
          "name": "Regrowth",
          "target": "Kenrith's Transformation"
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
          "name": "Kenrith's Transformation",
          "target": "Owlin Shieldmage"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Kenrith's Transformation",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Regrowth",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Beast Within",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "unsupported",
          "source": "assertAttachedTo(playerB, aura, creature, true)"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Owlin Shieldmage",
          "ability": "Flying",
          "expected": false
        }
      ]
    },
    {
      "name": "wardRecastAuraPayBoth",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Owlin Shieldmage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kenrith's Transformation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Beast Within",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Regrowth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Kenrith's Transformation",
          "target": "Owlin Shieldmage"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "name": "Beast Within",
          "target": "Kenrith's Transformation"
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
          "name": "Regrowth",
          "target": "Kenrith's Transformation"
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
          "name": "Kenrith's Transformation",
          "target": "Owlin Shieldmage"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 14
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Beast Within",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Regrowth",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "unsupported",
          "source": "assertAttachedTo(playerB, aura, creature, true)"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Owlin Shieldmage",
          "ability": "Flying",
          "expected": false
        }
      ]
    }
  ]
});
