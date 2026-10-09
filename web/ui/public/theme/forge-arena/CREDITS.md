# Moonlit sanctuary assets

## Third-party 3D models

Downloaded from Poly Haven's public asset API on 2026-10-08. Both models and
textures are licensed **CC0 1.0**; they may be bundled and redistributed.
License: https://polyhaven.com/license
Legal text: https://creativecommons.org/publicdomain/zero/1.0/legalcode

- **Rock Face 02** — Dario Barresi; processing by Rico Cilliers.
  https://polyhaven.com/a/rock_face_02
  Source manifest: https://api.polyhaven.com/files/rock_face_02
- **Stone Fire Pit** — Sebastian Platen.
  https://polyhaven.com/a/stone_fire_pit
  Source manifest: https://api.polyhaven.com/files/stone_fire_pit

The original glTF geometry, buffers, and 1K JPEG texture maps are bundled here.
Downloads were checked against the API's MD5 checksums. Only the glTF entry
filenames were shortened; content is unchanged. Runtime instances share geometry
and textures and are positioned, rotated, and scaled by Three.js. The game makes
no requests to Poly Haven. Asset credit is retained even though CC0 does not
require attribution.

## Terrain artwork

`terrain.png` was generated with the built-in image generation tool on 2026-10-08.
It is original background artwork, not a screenshot or an extracted MTGA asset.
The exact generation prompt is saved in `terrain-prompt.txt`.
The terrain is a painted background plane. The imported rock faces and fire pits
are actual 3D glTF meshes; moonwell ripples, lighting, and silver motes are rendered
in Three.js. This distinction matters when replacing the art or adding assets.
