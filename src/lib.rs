extern crate proc_macro;
extern crate syn;
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn flaky_test(_attr: TokenStream, input: TokenStream) -> TokenStream {
  let input_fn = syn::parse_macro_input!(input as syn::ItemFn);
  let name = input_fn.sig.ident.clone();
  TokenStream::from(quote! {
    #[test]
    fn #name() {
      #input_fn

      for i in 0..3 {
        println!("flaky_test retry {}", i);
        let r = std::panic::catch_unwind(|| {
          #name();
        });
        if r.is_ok() {
          return;
        }
        if i == 2 {
          std::panic::resume_unwind(r.unwrap_err());
        }
      }
    }
  })
}
