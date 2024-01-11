extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser as _;
use syn::punctuated::Punctuated;
use syn::Attribute;
use syn::ItemFn;
use syn::Lit;
use syn::Meta;
#[cfg(feature = "tokio")]
use syn::MetaList;
use syn::MetaNameValue;
#[cfg(feature = "tokio")]
use syn::NestedMeta;
use syn::Token;

struct FlakyTestArgs {
  times: usize,
  runtime: Runtime,
}

enum Runtime {
  Sync,
  #[cfg(feature = "tokio")]
  Tokio(Option<Punctuated<NestedMeta, Token![,]>>),
}

impl Default for FlakyTestArgs {
  fn default() -> Self {
    FlakyTestArgs {
      times: 3,
      runtime: Runtime::Sync,
    }
  }
}

fn parse_attr(attr: proc_macro2::TokenStream) -> syn::Result<FlakyTestArgs> {
  let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
  let punctuated = parser.parse2(attr)?;

  let mut ret = FlakyTestArgs::default();

  for meta in punctuated {
    match meta {
      #[cfg(feature = "tokio")]
      Meta::Path(path) => {
        if path.is_ident("tokio") {
          ret.runtime = Runtime::Tokio(None);
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
      #[cfg(feature = "tokio")]
      Meta::List(MetaList { path, nested, .. }) => {
        if path.is_ident("tokio") {
          ret.runtime = Runtime::Tokio(Some(nested));
        } else {
          return Err(syn::Error::new_spanned(path, "expected `tokio`"));
        }
      }
      _ => {
        let msg = if cfg!(feature = "tokio") {
          "expected `times = <int>` or `tokio`"
        } else {
          "expected `times = <int>"
        };

        return Err(syn::Error::new_spanned(meta, msg));
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
///
/// // Any arguments that `#[tokio::test]` supports can be specified.
/// #[flaky_test(tokio(flavor = "multi_thraed", worker_threads = 2))]
/// async fn async_test_complex() {
///   let res = async_operation().await.unwrap();
///   assert_eq!(res, 42);
/// }
/// ```
#[proc_macro_attribute]
pub fn flaky_test(attr: TokenStream, input: TokenStream) -> TokenStream {
  let attr = proc_macro2::TokenStream::from(attr);
  let mut input = proc_macro2::TokenStream::from(input);

  match inner(attr, input.clone()) {
    Err(e) => {
      input.extend(e.into_compile_error());
      input.into()
    }
    Ok(t) => t.into(),
  }
}

fn inner(
  attr: proc_macro2::TokenStream,
  input: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
  let args = parse_attr(attr)?;
  let input_fn: ItemFn = syn::parse2(input)?;
  let attrs = input_fn.attrs.clone();

  match args.runtime {
    Runtime::Sync => sync(input_fn, attrs, args.times),
    #[cfg(feature = "tokio")]
    Runtime::Tokio(tokio_args) => {
      tokio(input_fn, attrs, args.times, tokio_args)
    }
  }
}

fn sync(
  input_fn: ItemFn,
  attrs: Vec<Attribute>,
  times: usize,
) -> syn::Result<proc_macro2::TokenStream> {
  let fn_name = input_fn.sig.ident.clone();

  Ok(quote! {
    #[test]
    #(#attrs)*
    fn #fn_name() {
      #input_fn

      for i in 0..#times {
        println!("flaky_test retry {}", i);
        let r = ::std::panic::catch_unwind(|| {
          #fn_name();
        });
        if r.is_ok() {
          return;
        }
        if i == #times - 1 {
          ::std::panic::resume_unwind(r.unwrap_err());
        }
      }
    }
  })
}

#[cfg(feature = "tokio")]
fn tokio(
  input_fn: ItemFn,
  attrs: Vec<Attribute>,
  times: usize,
  tokio_args: Option<Punctuated<NestedMeta, Token![,]>>,
) -> syn::Result<proc_macro2::TokenStream> {
  if input_fn.sig.asyncness.is_none() {
    return Err(syn::Error::new_spanned(input_fn.sig, "must be `async fn`"));
  }

  let fn_name = input_fn.sig.ident.clone();
  let tokio_macro = match tokio_args {
    Some(args) => quote! { #[::tokio::test(#args)] },
    None => quote! { #[::tokio::test] },
  };

  Ok(quote! {
    #tokio_macro
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
          ::std::panic::resume_unwind(r.unwrap_err());
        }
      }
    }
  })
}
