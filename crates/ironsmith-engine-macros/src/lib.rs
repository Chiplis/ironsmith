use proc_macro::TokenStream;
use quote::quote;
use syn::{ImplItemFn, parse_macro_input};

/// Generate the scalar EffectExecutor entry point from its output-producing method.
#[proc_macro_attribute]
pub fn project_effect_outcome(args: TokenStream, item: TokenStream) -> TokenStream {
    let method = parse_macro_input!(item as ImplItemFn);
    if !args.is_empty() || method.sig.ident != "execute_with_outputs" {
        return syn::Error::new_spanned(
            method.sig.ident,
            "apply #[project_effect_outcome] to fn execute_with_outputs",
        )
        .into_compile_error()
        .into();
    }

    quote! {
        #method

        fn execute(
            &self,
            game: &mut crate::game_state::GameState,
            ctx: &mut crate::effects::EffectContext<'_>,
        ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
            self.execute_with_outputs(game, ctx)
                .map(crate::effects::CompletedEffectOutputs::into_outcome)
        }
    }
    .into()
}
