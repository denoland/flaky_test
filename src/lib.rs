extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
  parse::Parser as _, punctuated::Punctuated, Attribute, ItemFn, Lit, Meta,
  MetaNameValue, Token,
};

struct FlakyTestArgs {
  times: usize,
  runtime: Runtime,
}

enum Runtime {
  Sync,
  #[cfg(feature = "tokio")]
  Tokio,
}

impl Default for FlakyTestArgs {
  fn default() -> Self {
    FlakyTestArgs {
      times: 3,
      runtime: Runtime::Sync,
    }
  }
}

fn parse_attr(attr: TokenStream) -> syn::Result<FlakyTestArgs> {
  let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
  let punctuated = parser.parse(attr)?;

  let mut ret = FlakyTestArgs::default();

  for meta in punctuated {
    match meta {
      #[cfg(feature = "tokio")]
      Meta::Path(path) => {
        if path.is_ident("tokio") {
          ret.runtime = Runtime::Tokio;
        } else {
          return Err(syn::Error::new_spanned(path, "expected `tokio`"));
        }
      }
      Meta::NameValue(MetaNameValue {
        path,
        lit: Lit::Int(lit_int),
        ..
      }) => {
        if path.is_ident("times") {
          ret.times = lit_int.base10_parse::<usize>()?;
        } else {
          return Err(syn::Error::new_spanned(
            path,
            "expected `times = <int>`",
          ));
        }
      }
      _ => {
        return Err(syn::Error::new_spanned(
          meta,
          "expected `times = <int>` or `tokio`",
        ))
      }
    }
  }

  Ok(ret)
}

/// A flaky test will be run multiple times until it passes.
///
/// # Example
///
/// ```rust
/// use flaky_test::flaky_test;
///
/// // By default it will be retried up to 3 times.
/// #[flaky_test]
/// fn test_default() {
///  println!("should pass");
/// }
///
/// // The number of max attempts can be adjusted via `times`.
/// #[flaky_test(times = 5)]
/// fn usage_with_named_args() {
///   println!("should pass");
/// }
///
/// # use std::convert::Infallible;
/// # async fn async_operation() -> Result<i32, Infallible> {
/// #   Ok(42)
/// # }
/// // Async tests can be run by passing `tokio`.
/// // Enabling `tokio` feature flag is needed.
/// #[flaky_test(tokio)]
/// async fn async_test() {
///   let res = async_operation().await.unwrap();
///   assert_eq!(res, 42);
/// }
///
/// // `tokio` and `times` can be combined.
/// #[flaky_test(tokio, times = 5)]
/// async fn async_test_five_times() {
///   let res = async_operation().await.unwrap();
///   assert_eq!(res, 42);
/// }
/// ```
#[proc_macro_attribute]
pub fn flaky_test(attr: TokenStream, input: TokenStream) -> TokenStream {
  let args = match parse_attr(attr) {
    Err(e) => {
      let mut input2 = proc_macro2::TokenStream::from(input);
      input2.extend(e.into_compile_error());
      return input2.into();
    }
    Ok(args) => args,
  };
  let input_fn = syn::parse_macro_input!(input as ItemFn);
  let attrs = input_fn.attrs.clone();

  match args.runtime {
    Runtime::Sync => sync(input_fn, attrs, args.times),
    #[cfg(feature = "tokio")]
    Runtime::Tokio => tokio(input_fn, attrs, args.times),
  }
}

fn sync(input_fn: ItemFn, attrs: Vec<Attribute>, times: usize) -> TokenStream {
  let fn_name = input_fn.sig.ident.clone();

  TokenStream::from(quote! {
    #[test]
    #(#attrs)*
    fn #fn_name() {
      #input_fn

      for i in 0..#times {
        println!("flaky_test retry {}", i);
        let r = std::panic::catch_unwind(|| {
          #fn_name();
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

#[cfg(feature = "tokio")]
fn tokio(input_fn: ItemFn, attrs: Vec<Attribute>, times: usize) -> TokenStream {
  let fn_name = input_fn.sig.ident.clone();

  TokenStream::from(quote! {
    #[tokio::test]
    #(#attrs)*
    async fn #fn_name() {
      #input_fn

      for i in 0..#times {
        println!("flaky_test retry {}", i);
        use ::futures_util::future::FutureExt as _;
        let r = #fn_name().catch_unwind().await;
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
