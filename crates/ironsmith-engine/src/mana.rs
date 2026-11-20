pub use ironsmith_core::mana::*;

/// Expand a price in printed order using the caller's resolved X and life-payment permission.
pub(crate) fn unit_payment_pips(
    cost: &ManaCost,
    x_value: u32,
    allow_black_life: bool,
) -> Vec<Vec<ManaSymbol>> {
    let mut pips = Vec::new();
    for pip in cost.pips() {
        let units = match pip.as_slice() {
            [ManaSymbol::Generic(n)] => u32::from(*n),
            [ManaSymbol::X] => x_value,
            [ManaSymbol::Black] if allow_black_life => {
                pips.push(vec![ManaSymbol::Black, ManaSymbol::Life(2)]);
                continue;
            }
            // K'rrik, Son of Yawgmoth: "for each {B} in a cost" includes a
            // hybrid symbol with a black half ({W/B}, {2/B}); paying 2 life
            // for it is one more alternative for that one symbol.
            alternatives
                if allow_black_life
                    && alternatives.contains(&ManaSymbol::Black)
                    && !alternatives
                        .iter()
                        .any(|symbol| matches!(symbol, ManaSymbol::Life(_))) =>
            {
                let mut alternatives = alternatives.to_vec();
                alternatives.push(ManaSymbol::Life(2));
                pips.push(alternatives);
                continue;
            }
            _ => {
                pips.push(pip.clone());
                continue;
            }
        };
        pips.extend((0..units).map(|_| vec![ManaSymbol::Generic(1)]));
    }
    pips
}
