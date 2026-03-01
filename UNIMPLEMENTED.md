# Unimplemented mechanics (not truly implemented)

Source: `cargo run --quiet --bin audit_parsed_mechanics -- --cards /Users/chiplis/ironsmith/cards.json --json-out /tmp/parsed_mechanics.json`

Generated: Tue Feb 17 22:34:12 -03 2026

Scope: parse-success cards whose abilities are represented as placeholder markers (`StaticAbilityId::KeywordMarker`, `StaticAbilityId::RuleTextPlaceholder`, or `StaticAbilityId::UnsupportedParserLine`) (i.e., mechanics not backed by dedicated runtime/static implementation).

## Summary

Total parse-success cards with unimplemented markers: 1176
Unique unimplemented marker mechanics: 451



## Canonical family grouping

| # | Mechanic family | Cards | Ability instances | Variant count | Example variants |
|-:|---|---:|---:|---:|---|
| 1 | `flashback` | 116 | 116 | 63 | `flashback {3}{r}`, `flashback {2}{r}`, `flashback {5}{r}` |
| 2 | `crew` | 84 | 84 | 6 | `crew 2`, `crew 1`, `crew 3` |
| 3 | `unearth` | 46 | 46 | 30 | `unearth {2}{b}`, `unearth {3}{b}`, `unearth {1}{b}` |
| 4 | `suspend` | 45 | 45 | 31 | `suspend 2 {g}`, `suspend 4 {1}{r}`, `suspend 3 {0}` |
| 5 | `echo` | 44 | 44 | 25 | `echo {2}{r}`, `echo {2}{g}`, `echo {2}{r}{r}` |
| 6 | `cumulative upkeep` | 36 | 36 | 22 | `cumulative upkeep {1}`, `cumulative upkeep {u}`, `cumulative upkeep pay 1 life` |
| 7 | `bestow` | 30 | 30 | 26 | `bestow {2}{w}`, `bestow {3}{w}`, `bestow {4}{b}` |
| 8 | `daybound` | 29 | 29 | 1 | `daybound` |
| 9 | `buyback` | 27 | 27 | 10 | `buyback {3}`, `buyback {4}`, `buyback—sacrifice a land` |
| 10 | `cascade` | 26 | 30 | 1 | `cascade` |
| 11 | `partner` | 25 | 25 | 1 | `partner` |
| 12 | `ninjutsu` | 24 | 24 | 14 | `ninjutsu {1}{b}`, `ninjutsu {1}{u}`, `ninjutsu {2}{u}` |
| 13 | `disturb` | 23 | 23 | 13 | `disturb {1}{u}`, `disturb {1}{w}{u}`, `disturb {2}{u}` |
| 14 | `soulshift` | 22 | 23 | 8 | `soulshift 3`, `soulshift 4`, `soulshift 5` |
| 15 | `disguise` | 22 | 22 | 20 | `disguise {1}{w}`, `disguise {4}`, `disguise {1}{r}{w}` |
| 16 | `saddle` | 22 | 22 | 5 | `saddle 1`, `saddle 2`, `saddle 3` |
| 17 | `banding` | 21 | 21 | 1 | `banding` |
| 18 | `rebound` | 21 | 21 | 1 | `rebound` |
| 19 | `modular` | 19 | 19 | 5 | `modular 1`, `modular 2`, `modular 3` |
| 20 | `dash` | 18 | 18 | 12 | `dash {1}{b}`, `dash {1}{r}`, `dash {2}{b}` |
| 21 | `reveal-on-enter` | 18 | 18 | 18 | `as this land enters, you may reveal a faerie card from your hand. if you don't, this land enters tapped`, `as this land enters, you may reveal a forest or island card from your hand. if you don't, this land enters tapped`, `as this land enters, you may reveal a forest or plains card from your hand. if you don't, this land enters tapped` |
| 22 | `splice onto arcane` | 18 | 18 | 12 | `splice onto arcane {1}{u}`, `splice onto arcane`, `splice onto arcane {1}{r}` |
| 23 | `myriad` | 17 | 18 | 1 | `myriad` |
| 24 | `backup` | 17 | 17 | 2 | `backup 1`, `backup 2` |
| 25 | `plot` | 17 | 17 | 12 | `plot {1}{r}`, `plot {3}{u}`, `plot {2}{r}` |
| 26 | `renown` | 17 | 17 | 3 | `renown 1`, `renown 2`, `renown 6` |
| 27 | `evolve` | 16 | 16 | 1 | `evolve` |
| 28 | `mentor` | 15 | 15 | 1 | `mentor` |
| 29 | `fabricate` | 14 | 14 | 3 | `fabricate 1`, `fabricate 2`, `fabricate 3` |
| 30 | `split second` | 14 | 14 | 1 | `split second` |
| 31 | `assist` | 13 | 13 | 1 | `assist` |
| 32 | `extort` | 13 | 13 | 1 | `extort` |
| 33 | `fuse` | 13 | 13 | 1 | `fuse` |
| 34 | `unleash` | 13 | 13 | 1 | `unleash` |
| 35 | `vanishing` | 13 | 13 | 4 | `vanishing 3`, `vanishing 2`, `vanishing 4` |
| 36 | `battle cry` | 12 | 12 | 1 | `battle cry` |
| 37 | `scavenge` | 12 | 12 | 11 | `scavenge {4}{g}{g}`, `scavenge {0}`, `scavenge {1}{b}{g}` |
| 38 | `cipher` | 11 | 11 | 1 | `cipher` |
| 39 | `outlast` | 11 | 11 | 8 | `outlast {w}`, `outlast {1}{w}`, `outlast {1}{b}` |
| 40 | `sunburst` | 11 | 11 | 1 | `sunburst` |
| 41 | `conspire` | 10 | 10 | 1 | `conspire` |
| 42 | `devour` | 10 | 10 | 3 | `devour 2`, `devour 1`, `devour 3` |
| 43 | `enlist` | 10 | 10 | 1 | `enlist` |
| 44 | `riot` | 10 | 10 | 1 | `riot` |
| 45 | `squad` | 10 | 10 | 3 | `squad {2}`, `squad {1}{g}`, `squad {3}` |
| 46 | `graft` | 9 | 9 | 5 | `graft 2`, `graft 1`, `graft 3` |
| 47 | `afterlife` | 8 | 8 | 3 | `afterlife 1`, `afterlife 2`, `afterlife 3` |
| 48 | `skulk` | 8 | 8 | 1 | `skulk` |
| 49 | `casualty` | 7 | 7 | 3 | `casualty 1`, `casualty 2`, `casualty 3` |
| 50 | `haunt` | 7 | 7 | 1 | `haunt` |
| 51 | `ingest` | 7 | 7 | 1 | `ingest` |
| 52 | `training` | 7 | 7 | 1 | `training` |
| 53 | `ward` | 7 | 7 | 3 | `ward—pay 2 life`, `ward—pay 3 life`, `ward—pay 7 life` |
| 54 | `combat: can't attack/block alone` | 6 | 6 | 1 | `this creature can't attack or block alone` |
| 55 | `dethrone` | 6 | 6 | 1 | `dethrone` |
| 56 | `fading` | 6 | 6 | 3 | `fading 2`, `fading 3`, `fading 4` |
| 57 | `optional untap` | 6 | 6 | 3 | `you may choose not to untap this creature during your untap step`, `you may choose not to untap this artifact during your untap step`, `you may choose not to untap this during your untap step` |
| 58 | `ravenous` | 6 | 6 | 1 | `ravenous` |
| 59 | `global cast limit` | 5 | 5 | 1 | `each player can't cast more than one spell each turn` |
| 60 | `provoke` | 5 | 5 | 1 | `provoke` |
| 61 | `soulbond` | 5 | 5 | 1 | `soulbond` |
| 62 | `combat: can't attack alone` | 4 | 4 | 1 | `this creature can't attack alone` |
| 63 | `undaunted` | 4 | 4 | 1 | `undaunted` |
| 64 | `ascend` | 3 | 3 | 1 | `ascend` |
| 65 | `bolster` | 2 | 2 | 2 | `bolster 3`, `bolster 4` |
| 66 | `conditional cost modifier` | 2 | 2 | 2 | `activated abilities cost an additional "sacrifice a swamp" to activate for each black mana symbol in their activation costs`, `activated abilities of nontoken rebels cost an additional "sacrifice a land" to activate` |
| 67 | `creature flying restriction` | 2 | 2 | 1 | `creatures without flying can't attack` |
| 68 | `land untap lock` | 2 | 2 | 1 | `lands don't untap during their controllers' untap steps` |
| 69 | `shroud` | 2 | 2 | 1 | `you have shroud` |
| 70 | `untap this artifact during each other players untap step` | 2 | 2 | 1 | `untap this artifact during each other players untap step` |
| 71 | `you can't cast creature spells` | 2 | 2 | 1 | `you can't cast creature spells` |
| 72 | `bloodthirst` | 1 | 1 | 1 | `bloodthirst` |
| 73 | `cumulative upkeep—add {r}` | 1 | 1 | 1 | `cumulative upkeep—add {r}` |
| 74 | `once each turn, you may play a card from exile with a collection counter on it if it was exiled by an ability you controlled, and you may spend mana as though it were mana of any color to cast it` | 1 | 1 | 1 | `once each turn, you may play a card from exile with a collection counter on it if it was exiled by an ability you controlled, and you may spend mana as though it were mana of any color to cast it` |
| 75 | `populate` | 1 | 1 | 1 | `populate` |
| 76 | `spectacle {1}{b}` | 1 | 1 | 1 | `spectacle {1}{b}` |
| 77 | `spectacle {2}{b}` | 1 | 1 | 1 | `spectacle {2}{b}` |
| 78 | `spectacle {2}{r}` | 1 | 1 | 1 | `spectacle {2}{r}` |
| 79 | `spectacle {b}` | 1 | 1 | 1 | `spectacle {b}` |
| 80 | `spectacle {b}{r}` | 1 | 1 | 1 | `spectacle {b}{r}` |
| 81 | `spectacle {r}` | 1 | 1 | 1 | `spectacle {r}` |
| 82 | `spells cost an additional "sacrifice a swamp" to cast for each black mana symbol in their mana costs` | 1 | 1 | 1 | `spells cost an additional "sacrifice a swamp" to cast for each black mana symbol in their mana costs` |
| 83 | `this cost is reduced by {1} for each instant and sorcery card in your graveyard` | 1 | 1 | 1 | `this cost is reduced by {1} for each instant and sorcery card in your graveyard` |
| 84 | `this cost is reduced by {2} for each basic land type among lands you control` | 1 | 1 | 1 | `this cost is reduced by {2} for each basic land type among lands you control` |
| 85 | `this creature can't attack or block` | 1 | 1 | 1 | `this creature can't attack or block` |
| 86 | `untap all archers you control during each other players untap step` | 1 | 1 | 1 | `untap all archers you control during each other players untap step` |
| 87 | `untap all artifacts you control during each other players untap step` | 1 | 1 | 1 | `untap all artifacts you control during each other players untap step` |
| 88 | `untap all creatures you control during each other players untap step` | 1 | 1 | 1 | `untap all creatures you control during each other players untap step` |
| 89 | `untap all green and or blue creatures you control during each other players untap step` | 1 | 1 | 1 | `untap all green and or blue creatures you control during each other players untap step` |
| 90 | `untap all permanents you control during each other players untap step` | 1 | 1 | 1 | `untap all permanents you control during each other players untap step` |
| 91 | `untap each creature you control with a +1/+1 counter on it during each other players untap step` | 1 | 1 | 1 | `untap each creature you control with a +1/+1 counter on it during each other players untap step` |
| 92 | `untap this creature during each other players untap step` | 1 | 1 | 1 | `untap this creature during each other players untap step` |
## Mechanic list

| # | Mechanic | Cards | Ability instances |
|-:|---|---:|---:|
| 1 | `crew 2` | 31 | 31 |
| 2 | `daybound` | 29 | 29 |
| 3 | `crew 1` | 28 | 28 |
| 4 | `cascade` | 26 | 30 |
| 5 | `partner` | 25 | 25 |
| 6 | `rebound` | 21 | 21 |
| 7 | `banding` | 21 | 21 |
| 8 | `crew 3` | 17 | 17 |
| 9 | `myriad` | 17 | 18 |
| 10 | `evolve` | 16 | 16 |
| 11 | `mentor` | 15 | 15 |
| 12 | `split second` | 14 | 14 |
| 13 | `backup 1` | 14 | 14 |
| 14 | `unleash` | 13 | 13 |
| 15 | `fuse` | 13 | 13 |
| 16 | `extort` | 13 | 13 |
| 17 | `assist` | 13 | 13 |
| 18 | `renown 1` | 12 | 12 |
| 19 | `battle cry` | 12 | 12 |
| 20 | `sunburst` | 11 | 11 |
| 21 | `cipher` | 11 | 11 |
| 22 | `buyback {3}` | 11 | 11 |
| 23 | `riot` | 10 | 10 |
| 24 | `enlist` | 10 | 10 |
| 25 | `conspire` | 10 | 10 |
| 26 | `saddle 1` | 9 | 9 |
| 27 | `fabricate 1` | 9 | 9 |
| 28 | `squad {2}` | 8 | 8 |
| 29 | `skulk` | 8 | 8 |
| 30 | `training` | 7 | 7 |
| 31 | `modular 2` | 7 | 7 |
| 32 | `modular 1` | 7 | 7 |
| 33 | `ingest` | 7 | 7 |
| 34 | `haunt` | 7 | 7 |
| 35 | `cumulative upkeep {1}` | 7 | 7 |
| 36 | `this creature can't attack or block alone` | 6 | 6 |
| 37 | `soulshift 3` | 6 | 6 |
| 38 | `saddle 2` | 6 | 6 |
| 39 | `ravenous` | 6 | 6 |
| 40 | `flashback {3}{r}` | 6 | 6 |
| 41 | `dethrone` | 6 | 6 |
| 42 | `vanishing 3` | 5 | 5 |
| 43 | `unearth {2}{b}` | 5 | 5 |
| 44 | `soulbond` | 5 | 5 |
| 45 | `provoke` | 5 | 5 |
| 46 | `flashback {5}{r}` | 5 | 5 |
| 47 | `flashback {2}{r}` | 5 | 5 |
| 48 | `echo {2}{r}` | 5 | 5 |
| 49 | `each player can't cast more than one spell each turn` | 5 | 5 |
| 50 | `devour 2` | 5 | 5 |
| 51 | `crew 4` | 5 | 5 |
| 52 | `soulshift 4` | 5 | 6 |
| 53 | `you may choose not to untap this creature during your untap step` | 4 | 4 |
| 54 | `unearth {3}{b}` | 4 | 4 |
| 55 | `undaunted` | 4 | 4 |
| 56 | `this creature can't attack alone` | 4 | 4 |
| 57 | `soulshift 5` | 4 | 4 |
| 58 | `renown 2` | 4 | 4 |
| 59 | `ninjutsu {1}{b}` | 4 | 4 |
| 60 | `flashback {w}` | 4 | 4 |
| 61 | `flashback {g}` | 4 | 4 |
| 62 | `flashback {5}{u}{u}` | 4 | 4 |
| 63 | `flashback {3}{u}` | 4 | 4 |
| 64 | `fabricate 2` | 4 | 4 |
| 65 | `devour 1` | 4 | 4 |
| 66 | `cumulative upkeep {u}` | 4 | 4 |
| 67 | `afterlife 1` | 4 | 4 |
| 68 | `ward—pay 3 life` | 3 | 3 |
| 69 | `ward—pay 2 life` | 3 | 3 |
| 70 | `vanishing 4` | 3 | 3 |
| 71 | `vanishing 2` | 3 | 3 |
| 72 | `unearth {1}{r}` | 3 | 3 |
| 73 | `unearth {1}{b}` | 3 | 3 |
| 74 | `suspend 4 {1}{r}` | 3 | 3 |
| 75 | `suspend 2 {g}` | 3 | 3 |
| 76 | `splice onto arcane {1}{u}` | 3 | 3 |
| 77 | `saddle 4` | 3 | 3 |
| 78 | `saddle 3` | 3 | 3 |
| 79 | `plot {3}{u}` | 3 | 3 |
| 80 | `plot {1}{r}` | 3 | 3 |
| 81 | `outlast {w}` | 3 | 3 |
| 82 | `ninjutsu {3}{b}` | 3 | 3 |
| 83 | `ninjutsu {2}{u}` | 3 | 3 |
| 84 | `ninjutsu {1}{u}` | 3 | 3 |
| 85 | `graft 2` | 3 | 3 |
| 86 | `flashback {6}{g}{g}` | 3 | 3 |
| 87 | `flashback {5}{b}{b}` | 3 | 3 |
| 88 | `flashback {4}{u}` | 3 | 3 |
| 89 | `flashback {3}{w}` | 3 | 3 |
| 90 | `flashback {2}{u}` | 3 | 3 |
| 91 | `flashback {1}{u}` | 3 | 3 |
| 92 | `flashback {1}{g}` | 3 | 3 |
| 93 | `flashback {1}{b}` | 3 | 3 |
| 94 | `fading 2` | 3 | 3 |
| 95 | `echo {4}{r}{r}` | 3 | 3 |
| 96 | `echo {3}{r}` | 3 | 3 |
| 97 | `echo {2}{r}{r}` | 3 | 3 |
| 98 | `echo {2}{g}` | 3 | 3 |
| 99 | `disturb {3}{w}` | 3 | 3 |
| 100 | `disturb {2}{u}` | 3 | 3 |
| 101 | `disturb {1}{w}{u}` | 3 | 3 |
| 102 | `disturb {1}{u}` | 3 | 3 |
| 103 | `cumulative upkeep {2}` | 3 | 3 |
| 104 | `cumulative upkeep pay 1 life` | 3 | 3 |
| 105 | `casualty 1` | 3 | 3 |
| 106 | `buyback—sacrifice a land` | 3 | 3 |
| 107 | `buyback {4}` | 3 | 3 |
| 108 | `backup 2` | 3 | 3 |
| 109 | `ascend` | 3 | 3 |
| 110 | `afterlife 2` | 3 | 3 |
| 111 | `you have shroud` | 2 | 2 |
| 112 | `you can't cast creature spells` | 2 | 2 |
| 113 | `vanishing 1` | 2 | 2 |
| 114 | `untap this artifact during each other players untap step` | 2 | 2 |
| 115 | `unearth {u}` | 2 | 2 |
| 116 | `unearth {3}{r}{r}` | 2 | 2 |
| 117 | `unearth {3}{b}{b}` | 2 | 2 |
| 118 | `unearth {2}{u}` | 2 | 2 |
| 119 | `unearth {2}` | 2 | 2 |
| 120 | `suspend 6 {1}{u}` | 2 | 2 |
| 121 | `suspend 5 {w}` | 2 | 2 |
| 122 | `suspend 5 {1}{w}` | 2 | 2 |
| 123 | `suspend 4 {u}` | 2 | 2 |
| 124 | `suspend 4 {g}` | 2 | 2 |
| 125 | `suspend 4 {1}{u}` | 2 | 2 |
| 126 | `suspend 3 {w}` | 2 | 2 |
| 127 | `suspend 3 {2}{u}` | 2 | 2 |
| 128 | `suspend 3 {2}{b}` | 2 | 2 |
| 129 | `suspend 3 {0}` | 2 | 2 |
| 130 | `splice onto arcane {w}` | 2 | 2 |
| 131 | `splice onto arcane {3}{u}` | 2 | 2 |
| 132 | `splice onto arcane {1}{r}` | 2 | 2 |
| 133 | `splice onto arcane` | 2 | 2 |
| 134 | `soulshift 7` | 2 | 2 |
| 135 | `soulshift 6` | 2 | 2 |
| 136 | `scavenge {4}{g}{g}` | 2 | 2 |
| 137 | `plot {2}{r}` | 2 | 2 |
| 138 | `outlast {1}{w}` | 2 | 2 |
| 139 | `ninjutsu {1}{g}` | 2 | 2 |
| 140 | `modular 4` | 2 | 2 |
| 141 | `modular 3` | 2 | 2 |
| 142 | `lands don't untap during their controllers' untap steps` | 2 | 2 |
| 143 | `graft 3` | 2 | 2 |
| 144 | `graft 1` | 2 | 2 |
| 145 | `flashback {u}` | 2 | 2 |
| 146 | `flashback {7}{u}` | 2 | 2 |
| 147 | `flashback {6}{g}` | 2 | 2 |
| 148 | `flashback {5}{b}` | 2 | 2 |
| 149 | `flashback {4}{r}{r}` | 2 | 2 |
| 150 | `flashback {4}{r}` | 2 | 2 |
| 151 | `flashback {4}{g}` | 2 | 2 |
| 152 | `flashback {4}{b}` | 2 | 2 |
| 153 | `flashback {3}{g}` | 2 | 2 |
| 154 | `flashback {3}{b}` | 2 | 2 |
| 155 | `flashback {2}{g}` | 2 | 2 |
| 156 | `flashback {1}{w}` | 2 | 2 |
| 157 | `fading 3` | 2 | 2 |
| 158 | `echo {r}` | 2 | 2 |
| 159 | `echo {5}{r}` | 2 | 2 |
| 160 | `echo {4}` | 2 | 2 |
| 161 | `echo {3}` | 2 | 2 |
| 162 | `echo {1}{r}` | 2 | 2 |
| 163 | `echo {1}{g}{g}` | 2 | 2 |
| 164 | `echo {1}{g}` | 2 | 2 |
| 165 | `disturb {w}{u}` | 2 | 2 |
| 166 | `disturb {1}{w}` | 2 | 2 |
| 167 | `disguise {4}` | 2 | 2 |
| 168 | `disguise {1}{w}` | 2 | 2 |
| 169 | `dash {3}{r}` | 2 | 2 |
| 170 | `dash {3}{b}` | 2 | 2 |
| 171 | `dash {2}{r}` | 2 | 2 |
| 172 | `dash {2}{b}` | 2 | 2 |
| 173 | `dash {1}{r}` | 2 | 2 |
| 174 | `dash {1}{b}` | 2 | 2 |
| 175 | `cumulative upkeep {g}` | 2 | 2 |
| 176 | `crew 6` | 2 | 2 |
| 177 | `creatures without flying can't attack` | 2 | 2 |
| 178 | `casualty 3` | 2 | 2 |
| 179 | `casualty 2` | 2 | 2 |
| 180 | `buyback—discard two cards` | 2 | 2 |
| 181 | `buyback {2}{b}{b}` | 2 | 2 |
| 182 | `buyback {2}` | 2 | 2 |
| 183 | `bestow {4}{u}` | 2 | 2 |
| 184 | `bestow {4}{b}` | 2 | 2 |
| 185 | `bestow {3}{w}` | 2 | 2 |
| 186 | `bestow {2}{w}` | 2 | 2 |
| 187 | `you may choose not to untap this during your untap step` | 1 | 1 |
| 188 | `you may choose not to untap this artifact during your untap step` | 1 | 1 |
| 189 | `ward—pay 7 life` | 1 | 1 |
| 190 | `untap this creature during each other players untap step` | 1 | 1 |
| 191 | `untap each creature you control with a +1/+1 counter on it during each other players untap step` | 1 | 1 |
| 192 | `untap all permanents you control during each other players untap step` | 1 | 1 |
| 193 | `untap all green and or blue creatures you control during each other players untap step` | 1 | 1 |
| 194 | `untap all creatures you control during each other players untap step` | 1 | 1 |
| 195 | `untap all artifacts you control during each other players untap step` | 1 | 1 |
| 196 | `untap all archers you control during each other players untap step` | 1 | 1 |
| 197 | `unearth {w}` | 1 | 1 |
| 198 | `unearth {u}{b}{r}` | 1 | 1 |
| 199 | `unearth {r}` | 1 | 1 |
| 200 | `unearth {g}{g}` | 1 | 1 |
| 201 | `unearth {b}{r}` | 1 | 1 |
| 202 | `unearth {b}` | 1 | 1 |
| 203 | `unearth {8}` | 1 | 1 |
| 204 | `unearth {7}` | 1 | 1 |
| 205 | `unearth {6}{u}` | 1 | 1 |
| 206 | `unearth {6}{r}{r}` | 1 | 1 |
| 207 | `unearth {5}{r}` | 1 | 1 |
| 208 | `unearth {5}{b}{b}{b}` | 1 | 1 |
| 209 | `unearth {4}{r}` | 1 | 1 |
| 210 | `unearth {4}{b}` | 1 | 1 |
| 211 | `unearth {3}{w}{b}` | 1 | 1 |
| 212 | `unearth {3}{w}` | 1 | 1 |
| 213 | `unearth {3}{b}{r}` | 1 | 1 |
| 214 | `unearth {2}{w}` | 1 | 1 |
| 215 | `unearth {2}{r}` | 1 | 1 |
| 216 | `unearth {2}{g}{g}` | 1 | 1 |
| 217 | `unearth {1}{u}{b}` | 1 | 1 |
| 218 | `this creature can't attack or block` | 1 | 1 |
| 219 | `this cost is reduced by {2} for each basic land type among lands you control` | 1 | 1 |
| 220 | `this cost is reduced by {1} for each instant and sorcery card in your graveyard` | 1 | 1 |
| 221 | `suspend 5 {g}` | 1 | 1 |
| 222 | `suspend 5 {b}` | 1 | 1 |
| 223 | `suspend 4 {r}` | 1 | 1 |
| 224 | `suspend 4 {b}` | 1 | 1 |
| 225 | `suspend 4 {3}{r}{w}` | 1 | 1 |
| 226 | `suspend 4 {1}{g}` | 1 | 1 |
| 227 | `suspend 3 {2}{r}` | 1 | 1 |
| 228 | `suspend 3 {2}{g}` | 1 | 1 |
| 229 | `suspend 3 {1}{w}{w}` | 1 | 1 |
| 230 | `suspend 3 {1}{w}` | 1 | 1 |
| 231 | `suspend 3 {1}{u}` | 1 | 1 |
| 232 | `suspend 3 {1}` | 1 | 1 |
| 233 | `suspend 2 {2}` | 1 | 1 |
| 234 | `suspend 2 {1}{w}` | 1 | 1 |
| 235 | `suspend 2 {1}{u}` | 1 | 1 |
| 236 | `suspend 2 {1}{r}{r}` | 1 | 1 |
| 237 | `suspend 2 {1}{b}` | 1 | 1 |
| 238 | `suspend 10 {w}` | 1 | 1 |
| 239 | `suspend 1 {r}` | 1 | 1 |
| 240 | `squad {3}` | 1 | 1 |
| 241 | `squad {1}{g}` | 1 | 1 |
| 242 | `splice onto arcane {u}` | 1 | 1 |
| 243 | `splice onto arcane {g}` | 1 | 1 |
| 244 | `splice onto arcane {3}{g}` | 1 | 1 |
| 245 | `splice onto arcane {3}{b}{b}` | 1 | 1 |
| 246 | `splice onto arcane {2}{b}` | 1 | 1 |
| 247 | `splice onto arcane {1}{g}` | 1 | 1 |
| 248 | `splice onto arcane {1}{b}` | 1 | 1 |
| 249 | `spells cost an additional "sacrifice a swamp" to cast for each black mana symbol in their mana costs` | 1 | 1 |
| 250 | `spectacle {r}` | 1 | 1 |
| 251 | `spectacle {b}{r}` | 1 | 1 |
| 252 | `spectacle {b}` | 1 | 1 |
| 253 | `spectacle {2}{r}` | 1 | 1 |
| 254 | `spectacle {2}{b}` | 1 | 1 |
| 255 | `spectacle {1}{b}` | 1 | 1 |
| 256 | `soulshift 8` | 1 | 1 |
| 257 | `soulshift 2` | 1 | 1 |
| 258 | `soulshift 1` | 1 | 1 |
| 259 | `scavenge {6}{b}` | 1 | 1 |
| 260 | `scavenge {5}{g}{g}` | 1 | 1 |
| 261 | `scavenge {5}{g}` | 1 | 1 |
| 262 | `scavenge {4}{b}` | 1 | 1 |
| 263 | `scavenge {3}{g}{g}` | 1 | 1 |
| 264 | `scavenge {3}{b}{g}` | 1 | 1 |
| 265 | `scavenge {2}{b}{b}` | 1 | 1 |
| 266 | `scavenge {2}{b}` | 1 | 1 |
| 267 | `scavenge {1}{b}{g}` | 1 | 1 |
| 268 | `scavenge {0}` | 1 | 1 |
| 269 | `saddle 5` | 1 | 1 |
| 270 | `renown 6` | 1 | 1 |
| 271 | `populate` | 1 | 1 |
| 272 | `plot {r}` | 1 | 1 |
| 273 | `plot {4}{u}{u}` | 1 | 1 |
| 274 | `plot {4}{u}` | 1 | 1 |
| 275 | `plot {3}{w}` | 1 | 1 |
| 276 | `plot {3}{r}` | 1 | 1 |
| 277 | `plot {3}{g}` | 1 | 1 |
| 278 | `plot {2}{g}` | 1 | 1 |
| 279 | `plot {2}{b}` | 1 | 1 |
| 280 | `plot {1}{g}` | 1 | 1 |
| 281 | `outlast {g}` | 1 | 1 |
| 282 | `outlast {b}` | 1 | 1 |
| 283 | `outlast {2}{w}` | 1 | 1 |
| 284 | `outlast {2}` | 1 | 1 |
| 285 | `outlast {1}{g}` | 1 | 1 |
| 286 | `outlast {1}{b}` | 1 | 1 |
| 287 | `once each turn, you may play a card from exile with a collection counter on it if it was exiled by an ability you controlled, and you may spend mana as though it were mana of any color to cast it` | 1 | 1 |
| 288 | `ninjutsu {u}{b}` | 1 | 1 |
| 289 | `ninjutsu {u}` | 1 | 1 |
| 290 | `ninjutsu {b}` | 1 | 1 |
| 291 | `ninjutsu {3}{w}` | 1 | 1 |
| 292 | `ninjutsu {3}{g}` | 1 | 1 |
| 293 | `ninjutsu {3}{b}{b}` | 1 | 1 |
| 294 | `ninjutsu {2}{u}{u}` | 1 | 1 |
| 295 | `ninjutsu {2}{b}` | 1 | 1 |
| 296 | `ninjutsu {1}{r}` | 1 | 1 |
| 297 | `modular 6` | 1 | 1 |
| 298 | `graft 6` | 1 | 1 |
| 299 | `graft 5` | 1 | 1 |
| 300 | `flashback {x}{r}{r}{r}` | 1 | 1 |
| 301 | `flashback {r}{w}` | 1 | 1 |
| 302 | `flashback {r}` | 1 | 1 |
| 303 | `flashback {b}` | 1 | 1 |
| 304 | `flashback {9}{w}{w}{w}` | 1 | 1 |
| 305 | `flashback {9}{r}` | 1 | 1 |
| 306 | `flashback {9}{g}{g}{g}` | 1 | 1 |
| 307 | `flashback {8}{r}` | 1 | 1 |
| 308 | `flashback {7}{b}{b}{b}` | 1 | 1 |
| 309 | `flashback {7}{b}{b}` | 1 | 1 |
| 310 | `flashback {6}{b}{r}` | 1 | 1 |
| 311 | `flashback {6}{b}` | 1 | 1 |
| 312 | `flashback {5}{w}` | 1 | 1 |
| 313 | `flashback {5}{r}{r}` | 1 | 1 |
| 314 | `flashback {5}{g}{g}` | 1 | 1 |
| 315 | `flashback {5}{b}{g}` | 1 | 1 |
| 316 | `flashback {4}{w}{w}` | 1 | 1 |
| 317 | `flashback {4}{r}{w}` | 1 | 1 |
| 318 | `flashback {3}{g}{w}` | 1 | 1 |
| 319 | `flashback {3}{g}{u}` | 1 | 1 |
| 320 | `flashback {2}{w}{w}` | 1 | 1 |
| 321 | `flashback {2}{w}{b}` | 1 | 1 |
| 322 | `flashback {2}{r}{w}` | 1 | 1 |
| 323 | `flashback {2}{r}{r}` | 1 | 1 |
| 324 | `flashback {2}{g}{w}` | 1 | 1 |
| 325 | `flashback {2}{g}{g}{g}` | 1 | 1 |
| 326 | `flashback {2}{g}{g}` | 1 | 1 |
| 327 | `flashback {2}{b}{b}` | 1 | 1 |
| 328 | `flashback {1}{w}{u}` | 1 | 1 |
| 329 | `flashback {1}{w}, pay 3 life` | 1 | 1 |
| 330 | `flashback {1}{u}, pay 3 life` | 1 | 1 |
| 331 | `flashback {1}{r}, behold three elementals` | 1 | 1 |
| 332 | `flashback {1}{r}` | 1 | 1 |
| 333 | `flashback {1}{g}, pay 3 life` | 1 | 1 |
| 334 | `flashback {1}{b}{r}` | 1 | 1 |
| 335 | `flashback {1}{b}, pay 3 life` | 1 | 1 |
| 336 | `fading 4` | 1 | 1 |
| 337 | `fabricate 3` | 1 | 1 |
| 338 | `echo {g}{g}` | 1 | 1 |
| 339 | `echo {g}` | 1 | 1 |
| 340 | `echo {6}` | 1 | 1 |
| 341 | `echo {5}{g}` | 1 | 1 |
| 342 | `echo {4}{g}` | 1 | 1 |
| 343 | `echo {3}{w}{w}` | 1 | 1 |
| 344 | `echo {3}{w}` | 1 | 1 |
| 345 | `echo {3}{g}{g}` | 1 | 1 |
| 346 | `echo {3}{g}` | 1 | 1 |
| 347 | `echo {2}{w}{w}` | 1 | 1 |
| 348 | `echo {2}{b}` | 1 | 1 |
| 349 | `echo {1}{r}{r}` | 1 | 1 |
| 350 | `echo {1}{b}` | 1 | 1 |
| 351 | `disturb {4}{w}{w}` | 1 | 1 |
| 352 | `disturb {4}{w}` | 1 | 1 |
| 353 | `disturb {4}{u}` | 1 | 1 |
| 354 | `disturb {3}{w}{w}` | 1 | 1 |
| 355 | `disturb {3}{u}{u}` | 1 | 1 |
| 356 | `disturb {3}{u}` | 1 | 1 |
| 357 | `disturb {2}{w}` | 1 | 1 |
| 358 | `disguise {w}` | 1 | 1 |
| 359 | `disguise {g}` | 1 | 1 |
| 360 | `disguise {b}{r}` | 1 | 1 |
| 361 | `disguise {b}{b}` | 1 | 1 |
| 362 | `disguise {5}{r}` | 1 | 1 |
| 363 | `disguise {5}{g}{g}` | 1 | 1 |
| 364 | `disguise {5}{g}` | 1 | 1 |
| 365 | `disguise {4}{w}` | 1 | 1 |
| 366 | `disguise {4}{r}` | 1 | 1 |
| 367 | `disguise {4}{b}{b}` | 1 | 1 |
| 368 | `disguise {4}{b}` | 1 | 1 |
| 369 | `disguise {3}{r}{r}` | 1 | 1 |
| 370 | `disguise {3}` | 1 | 1 |
| 371 | `disguise {2}{w}` | 1 | 1 |
| 372 | `disguise {2}{r}` | 1 | 1 |
| 373 | `disguise {2}{g}{g}` | 1 | 1 |
| 374 | `disguise {2}` | 1 | 1 |
| 375 | `disguise {1}{r}{w}` | 1 | 1 |
| 376 | `devour 3` | 1 | 1 |
| 377 | `dash {r}` | 1 | 1 |
| 378 | `dash {4}{r}{w}` | 1 | 1 |
| 379 | `dash {3}{b}{r}` | 1 | 1 |
| 380 | `dash {2}{r}{r}` | 1 | 1 |
| 381 | `dash {2}{b}{b}` | 1 | 1 |
| 382 | `dash {1}{g}` | 1 | 1 |
| 383 | `cumulative upkeep—add {r}` | 1 | 1 |
| 384 | `cumulative upkeep {u}{r}` | 1 | 1 |
| 385 | `cumulative upkeep {s}` | 1 | 1 |
| 386 | `cumulative upkeep {g}{g}` | 1 | 1 |
| 387 | `cumulative upkeep {g} or {w}` | 1 | 1 |
| 388 | `cumulative upkeep {b}` | 1 | 1 |
| 389 | `cumulative upkeep {1}{u}` | 1 | 1 |
| 390 | `cumulative upkeep say this quickly` | 1 | 1 |
| 391 | `cumulative upkeep sacrifice a land` | 1 | 1 |
| 392 | `cumulative upkeep sacrifice a creature` | 1 | 1 |
| 393 | `cumulative upkeep put two cards from a single graveyard on the bottom of their owners library` | 1 | 1 |
| 394 | `cumulative upkeep put a -1/-1 counter on this creature` | 1 | 1 |
| 395 | `cumulative upkeep put a +1/+1 counter on a creature an opponent controls` | 1 | 1 |
| 396 | `cumulative upkeep have an opponent create a 1/1 red survivor creature token` | 1 | 1 |
| 397 | `cumulative upkeep gain control of a land you dont control` | 1 | 1 |
| 398 | `cumulative upkeep exile the top card of your library` | 1 | 1 |
| 399 | `cumulative upkeep draw a card` | 1 | 1 |
| 400 | `cumulative upkeep an opponent gains 1 life` | 1 | 1 |
| 401 | `crew 8` | 1 | 1 |
| 402 | `buyback {5}` | 1 | 1 |
| 403 | `buyback {2}{w}{w}` | 1 | 1 |
| 404 | `buyback {2}{u}` | 1 | 1 |
| 405 | `buyback {1}{u}` | 1 | 1 |
| 406 | `bolster 4` | 1 | 1 |
| 407 | `bolster 3` | 1 | 1 |
| 408 | `bloodthirst` | 1 | 1 |
| 409 | `bestow {x}{g}{g}` | 1 | 1 |
| 410 | `bestow {6}{w}` | 1 | 1 |
| 411 | `bestow {6}{r}` | 1 | 1 |
| 412 | `bestow {5}{w}{w}` | 1 | 1 |
| 413 | `bestow {5}{w}` | 1 | 1 |
| 414 | `bestow {5}{u}` | 1 | 1 |
| 415 | `bestow {5}{r}` | 1 | 1 |
| 416 | `bestow {5}{g}` | 1 | 1 |
| 417 | `bestow {5}{b}` | 1 | 1 |
| 418 | `bestow {4}{w}` | 1 | 1 |
| 419 | `bestow {4}{g}` | 1 | 1 |
| 420 | `bestow {3}{u}{u}` | 1 | 1 |
| 421 | `bestow {3}{g}{w}{u}` | 1 | 1 |
| 422 | `bestow {3}{g}{g}` | 1 | 1 |
| 423 | `bestow {3}{g}` | 1 | 1 |
| 424 | `bestow {3}{b}{b}` | 1 | 1 |
| 425 | `bestow {3}{b}` | 1 | 1 |
| 426 | `bestow {2}{w}{w}` | 1 | 1 |
| 427 | `bestow {2}{w}{u}{b}{r}{g}` | 1 | 1 |
| 428 | `bestow {2}{r}` | 1 | 1 |
| 429 | `bestow {1}{w}` | 1 | 1 |
| 430 | `bestow {1}{r}` | 1 | 1 |
| 431 | `as this land enters, you may reveal an island or swamp card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 432 | `as this land enters, you may reveal an island or mountain card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 433 | `as this land enters, you may reveal an elf card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 434 | `as this land enters, you may reveal an elemental card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 435 | `as this land enters, you may reveal a treefolk card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 436 | `as this land enters, you may reveal a swamp or mountain card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 437 | `as this land enters, you may reveal a swamp or forest card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 438 | `as this land enters, you may reveal a soldier card from your hand. this land enters tapped unless you revealed a soldier card this way or you control a soldier` | 1 | 1 |
| 439 | `as this land enters, you may reveal a plains or swamp card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 440 | `as this land enters, you may reveal a plains or island card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 441 | `as this land enters, you may reveal a mountain or plains card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 442 | `as this land enters, you may reveal a mountain or forest card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 443 | `as this land enters, you may reveal a merfolk card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 444 | `as this land enters, you may reveal a goblin card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 445 | `as this land enters, you may reveal a giant card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 446 | `as this land enters, you may reveal a forest or plains card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 447 | `as this land enters, you may reveal a forest or island card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 448 | `as this land enters, you may reveal a faerie card from your hand. if you don't, this land enters tapped` | 1 | 1 |
| 449 | `afterlife 3` | 1 | 1 |
| 450 | `activated abilities of nontoken rebels cost an additional "sacrifice a land" to activate` | 1 | 1 |
| 451 | `activated abilities cost an additional "sacrifice a swamp" to activate for each black mana symbol in their activation costs` | 1 | 1 |
