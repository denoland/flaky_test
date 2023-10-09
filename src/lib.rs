extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Lit, Meta, MetaNameValue, NestedMeta, ItemFn};

struct FlakyTestArgs {
  times: usize,
}

impl Default for FlakyTestArgs {
  fn default() -> Self {
    FlakyTestArgs { times: 3 }
  }
}

impl syn::parse::Parse for FlakyTestArgs {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    if input.is_empty() {
      return Ok(FlakyTestArgs::default());
    }

    let nested: NestedMeta = input.parse()?;

    match nested {
      NestedMeta::Lit(Lit::Int(lit_int)) => {
        let times = lit_int.base10_parse::<usize>()?;
        Ok(FlakyTestArgs { times })
      }
      NestedMeta::Meta(Meta::NameValue(MetaNameValue { path, lit: Lit::Int(lit_int), .. })) => {
        if path.is_ident("times") {
          let times = lit_int.base10_parse::<usize>()?;
          Ok(FlakyTestArgs { times })
        } else {
          Err(syn::Error::new_spanned(path, "expected `times`"))
        }
      }
      _ => Err(syn::Error::new_spanned(nested, "expected `times = INT` or `INT`")),
    }
  }
}

/// A flaky test will be run multiple times until it passes.
///
/// # Example
/// ```rust
/// use flaky_test::flaky_test;
///
/// #[flaky_test]
/// fn test_default() {
///  println!("should pass");
/// }
///
/// #[flaky_test(5)]
/// fn usage_with_args() {
///   println!("should pass");
/// }
///
/// #[flaky_test(times = 5)]
/// fn usage_with_named_args() {
///   println!("should pass");
/// }
/// ```
#[proc_macro_attribute]
pub fn flaky_test(attr: TokenStream, input: TokenStream) -> TokenStream {
  let args = syn::parse_macro_input!(attr as FlakyTestArgs);
  let input_fn = syn::parse_macro_input!(input as ItemFn);
  let name = input_fn.sig.ident.clone();
  let attrs = input_fn.attrs.clone();
  let times = args.times;

  TokenStream::from(quote! {
    #[test]
    #(#attrs)*
    fn #name() {
      #input_fn

      for i in 0..#times {
        println!("flaky_test retry {}", i);
        let r = std::panic::catch_unwind(|| {
          #name();
        });
        if r.is_ok() {
          return;
        }
        if i == #times - 1 {
          std::panic::resume_unwind(r.unwrap_err());
        }
      }
    }
  })
}
