use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Expr, ExprRange, RangeLimits};

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

    // Parse the range expression
    let (start, end) = match input {
        Expr::Range(ExprRange {
            start: Some(start),
            limits: RangeLimits::Closed(_),
            end: Some(end),
            ..
        }) => {
            let start_val = match *start {
                Expr::Lit(lit) => {
                    if let syn::Lit::Int(ref int_lit) = lit.lit {
                        int_lit.base10_parse::<u32>().expect("Invalid start value")
                    } else {
                        panic!("Expected integer literal for range start")
                    }
                }
                _ => panic!("Expected integer literal for range start"),
            };

            let end_val = match *end {
                Expr::Lit(lit) => {
                    if let syn::Lit::Int(ref int_lit) = lit.lit {
                        int_lit.base10_parse::<u32>().expect("Invalid end value")
                    } else {
                        panic!("Expected integer literal for range end")
                    }
                }
                _ => panic!("Expected integer literal for range end"),
            };

            (start_val, end_val)
        }
        _ => panic!("Expected a range expression like 0..=32"),
    };

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
                macro_rules! #macro_name {
                    () => { vec![Instruction::#instruction_name.opcode()] };
                }
            });
        } else {
            // Generate the macro with exactly n parameters
            // We need to generate the pattern ($b0:expr, $b1:expr, ..., $b(n-1):expr)
            // and the expansion vec![opcode, $b0, $b1, ..., $b(n-1)]
            
            let param_names: Vec<_> = (0..n)
                .map(|i| format!("b{}", i))
                .collect();
            
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
                macro_rules! {} {{
                    ({}) => {{
                        vec![Instruction::{}.opcode(), {}]
                    }};
                }}
                "#,
                macro_name, param_pattern, instruction_name, byte_list
            );
            
            let macro_tokens: proc_macro2::TokenStream = macro_str.parse().unwrap();
            macro_defs.push(macro_tokens);
        }
    }

    // Combine all macro definitions
    let expanded = quote! {
        #( #macro_defs )*
    };
    
    TokenStream::from(expanded)
}
