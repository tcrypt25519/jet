use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Expr, ExprRange, RangeLimits, parse_macro_input};

/// Generates PUSH macros for a range of values.
///
/// # Example
///
/// ```ignore
/// use jet_push_macros::generate_push_macros;
/// generate_push_macros!(0..=32);
/// ```
///
/// This will generate macros PUSH0, PUSH1, ..., PUSH32.
/// Each PUSHN macro takes exactly N byte arguments and generates the appropriate bytecode.
#[proc_macro]
pub fn generate_push_macros(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Expr);
    generate_push_macros_inner(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn generate_push_macros_inner(input: Expr) -> syn::Result<TokenStream2> {
    // Parse the range expression
    let (start, end, start_expr, end_expr) = match input {
        Expr::Range(ExprRange {
            start: Some(start),
            limits: RangeLimits::Closed(_),
            end: Some(end),
            ..
        }) => {
            let start_val = match *start.clone() {
                Expr::Lit(ref lit) => {
                    if let syn::Lit::Int(ref int_lit) = lit.lit {
                        int_lit.base10_parse::<u32>().map_err(|_| {
                            syn::Error::new_spanned(&lit.lit, "range start must be a valid u32")
                        })?
                    } else {
                        return Err(syn::Error::new_spanned(
                            &lit.lit,
                            "range start must be an integer literal",
                        ));
                    }
                }
                ref expr => {
                    return Err(syn::Error::new_spanned(
                        expr,
                        "range start must be an integer literal",
                    ));
                }
            };

            let end_val = match *end.clone() {
                Expr::Lit(ref lit) => {
                    if let syn::Lit::Int(ref int_lit) = lit.lit {
                        int_lit.base10_parse::<u32>().map_err(|_| {
                            syn::Error::new_spanned(&lit.lit, "range end must be a valid u32")
                        })?
                    } else {
                        return Err(syn::Error::new_spanned(
                            &lit.lit,
                            "range end must be an integer literal",
                        ));
                    }
                }
                ref expr => {
                    return Err(syn::Error::new_spanned(
                        expr,
                        "range end must be an integer literal",
                    ));
                }
            };

            (start_val, end_val, start, end)
        }
        ref expr => {
            return Err(syn::Error::new_spanned(
                expr,
                "expected a closed range expression (e.g., 0..=32)",
            ));
        }
    };

    // Validate the range
    if start > end {
        return Err(syn::Error::new_spanned(
            &*end_expr,
            format!("range end ({}) must be >= range start ({})", end, start),
        ));
    }

    // Generate the macro definitions
    let mut macro_defs = Vec::new();

    for n in start..=end {
        let macro_name = syn::Ident::new(&format!("PUSH{}", n), proc_macro2::Span::call_site());
        let instruction_name =
            syn::Ident::new(&format!("PUSH{}", n), proc_macro2::Span::call_site());

        if n == 0 {
            // PUSH0 takes no arguments
            macro_defs.push(quote! {
                #[allow(non_snake_case)]
                #[allow(unused_macros)]
                macro_rules! #macro_name {
                    () => { vec![Instruction::#instruction_name.opcode()] };
                }
            });
        } else {
            // Generate the macro with exactly n parameters
            // We need to generate the pattern ($b0:expr, $b1:expr, ..., $b(n-1):expr)
            // and the expansion vec![opcode, $b0, $b1, ..., $b(n-1)]

            let param_names: Vec<_> = (0..n).map(|i| format!("b{}", i)).collect();

            // Create the parameter pattern string
            let param_pattern = param_names
                .iter()
                .map(|name| format!("${}:expr", name))
                .collect::<Vec<_>>()
                .join(", ");

            // Create the byte list string
            let byte_list = param_names
                .iter()
                .map(|name| format!("${}", name))
                .collect::<Vec<_>>()
                .join(", ");

            // Build the macro as a string and parse it
            let macro_str = format!(
                r#"
                #[allow(non_snake_case)]
                #[allow(unused_macros)]
                macro_rules! {} {{
                    ({}) => {{
                        vec![Instruction::{}.opcode(), {}]
                    }};
                }}
                "#,
                macro_name, param_pattern, instruction_name, byte_list
            );

            let macro_tokens: proc_macro2::TokenStream = macro_str.parse().map_err(|e| {
                syn::Error::new_spanned(
                    &*start_expr,
                    format!("failed to generate macro for PUSH{}: {}", n, e),
                )
            })?;
            macro_defs.push(macro_tokens);
        }
    }

    // Combine all macro definitions
    Ok(quote! {
        #( #macro_defs )*
    })
}
