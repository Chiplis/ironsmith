// Bit flags shared by the keyword aura shader and the spec builder.
export const FX = Object.freeze({
  flying: 2,
  hexproof: 4,
  shroud: 8,
  ward: 16,
  indestructible: 32,
  protection: 64,
  attacking: 128,
  blocking: 256,
});

// Effects drawn behind the card face (outside its silhouette).
export const BACK_FX = FX.flying | FX.attacking | FX.blocking;
