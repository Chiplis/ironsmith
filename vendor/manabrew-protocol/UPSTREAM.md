# Local protocol version 3

Vendored verbatim from witchesofthehill/manabrew commit
`1bb698e06d090322a1d26e2e314c199f98c90f2c`, directory
`manabrew-rs/crates/manabrew-protocol`, on 2026-10-05.
The upstream crate is version 2.0.0 and licensed AGPL-3.0-or-later;
its full license is included here.

Local modifications: the package is versioned 3.0.0; workspace dependency,
license, and repository inheritance is made explicit; the typed
`PaymentResourceKind::Waterbend` variant is appended. All other wire shapes
remain the pinned version-2 shapes. This is a local protocol extension, not
an assertion that upstream published these exact version-3 sources.

Upstream later added the same Waterbend discriminant in commit
`cb9fd3033273ecad3c5af265e0a120b0da02f9f9`, then at protocol 4.1.0.
Using that whole dependency would also introduce unrelated DTO changes.
No source generation, builds or tests were run for this source checkpoint.
