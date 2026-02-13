---
name: rust-macros
description: Expert guidance on Rust macro authorship — both declarative (macro_rules!) and procedural macros (derive, attribute, function-like). Use this skill when writing, reviewing, debugging, or choosing between macro strategies. Covers syn/quote/proc-macro2 patterns, hygiene, testing, and when NOT to use macros.
---

# Rust Macros

Comprehensive patterns for writing correct, maintainable Rust macros. Both declarative and procedural, with concrete code and clear decision criteria.

## Decision Tree: Do You Need a Macro?

Before writing a macro, exhaust these alternatives:

| Goal | Prefer instead |
|------|---------------|
| Reduce boilerplate on structs/enums | `#[derive(...)]` (built-in or `thiserror`, `bon`) |
| Reuse logic across types | Generics + traits |
| Bundle related functions | Module or builder struct |
| Generate repetitive code | Generics with associated types |
| Domain-specific syntax | **Declarative macro is appropriate** |
| Custom derive | **Proc macro (derive) is appropriate** |
| Custom attributes | **Proc macro (attribute) is appropriate** |

**Rule:** If you can express it as a function or generic, do that. Macros add compilation complexity and debugging cost.

---

## Part 1: Declarative Macros (`macro_rules!`)

### When to Use

- You need variadic argument counts (`vec![1,2,3]`)
- You need syntax that can't be expressed as a function call (e.g., `assert_eq!`, `format!`)
- The pattern involves repetition over token trees
- Short, focused transformation — not multi-hundred-line macros

### Anatomy

```rust
macro_rules! my_macro {
    // Pattern 1: match specific syntax
    ($x:expr) => {
        // expand to...
        println!("{}", $x)
    };

    // Pattern 2: variadic with repetition
    ($($x:expr),* $(,)?) => {
        {
            let mut v = Vec::new();
            $( v.push($x); )*
            v
        }
    };
}
```

### Fragment Specifiers

| Specifier | Matches | Example |
|-----------|---------|---------|
| `expr` | Any expression | `1 + 2`, `"foo"`, `x.method()` |
| `ident` | Identifier | `foo`, `my_var` |
| `ty` | Type | `u32`, `Vec<String>` |
| `pat` | Pattern | `Some(x)`, `(a, b)` |
| `stmt` | Statement | `let x = 1;` |
| `block` | Block `{ ... }` | `{ foo(); bar() }` |
| `item` | Item (fn, struct, etc.) | `fn foo() {}` |
| `literal` | Literal only | `42`, `"str"`, `3.14` |
| `meta` | Attribute meta | `cfg(test)`, `derive(Debug)` |
| `tt` | Single token tree | Anything |
| `path` | Path | `std::io::Error`, `crate::Foo` |
| `vis` | Visibility | `pub`, `pub(crate)`, `` (empty) |
| `lifetime` | Lifetime | `'a`, `'static` |

**Prefer `tt` when in doubt** — it's the most flexible and avoids parser ambiguity.

### Repetition Operators

```rust
macro_rules! example {
    // Zero or more, comma-separated, trailing comma ok
    ($($item:expr),* $(,)?) => { ... };

    // One or more
    ($first:expr $(, $rest:expr)+) => { ... };

    // Optional
    ($required:expr $(, $optional:expr)?) => { ... };
}
```

Always add `$(,)?` to accept trailing commas — matches Rust's style and avoids user friction.

### Hygiene: The Key Rule

`macro_rules!` macros are **hygienic** for identifiers introduced inside the macro. They are **NOT** hygienic for identifiers passed in.

```rust
macro_rules! swap {
    ($a:ident, $b:ident) => {
        // `tmp` here won't conflict with any `tmp` at call site
        let tmp = $a;
        $a = $b;
        $b = tmp;
    };
}

// Safe: caller's `tmp` (if any) is unaffected
let mut x = 1;
let mut y = 2;
swap!(x, y);
```

**Anti-pattern — injecting identifiers without `$`:**
```rust
// BAD: introduces `result` into caller's scope — surprising
macro_rules! try_parse {
    ($s:expr) => {
        let result = $s.parse::<i32>();  // leaks `result`
        result.unwrap_or(0)
    };
}
```

**Fix — use a block to contain introduced names:**
```rust
macro_rules! try_parse {
    ($s:expr) => {{  // double braces = block expression
        let result = $s.parse::<i32>();
        result.unwrap_or(0)
    }};
}
```

### Common Patterns

#### Assertion helper
```rust
macro_rules! assert_matches {
    ($expr:expr, $pat:pat) => {
        assert!(
            matches!($expr, $pat),
            "expected {:?} to match {}",
            $expr,
            stringify!($pat)
        )
    };
    ($expr:expr, $pat:pat if $guard:expr) => {
        assert!(
            matches!($expr, $pat if $guard),
            "expected {:?} to match {} if {}",
            $expr,
            stringify!($pat),
            stringify!($guard)
        )
    };
}
```

#### Builder-style registration
```rust
macro_rules! register_handlers {
    ($router:expr, $( $method:ident => $path:literal => $handler:expr ),* $(,)?) => {{
        let mut r = $router;
        $(
            r = r.$method($path, $handler);
        )*
        r
    }};
}

// Usage:
let router = register_handlers!(
    Router::new(),
    get => "/users" => list_users,
    post => "/users" => create_user,
    delete => "/users/:id" => delete_user,
);
```

#### Counting items at compile time
```rust
macro_rules! count {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
}
```

### Exporting Macros

```rust
// In lib.rs or the defining module:
#[macro_export]
macro_rules! my_macro { ... }

// Consumers use it as:
use my_crate::my_macro;
// or: my_crate::my_macro!(...)
```

**Note:** `#[macro_export]` lifts the macro to the crate root. For internal macros, omit it and use `#[allow(unused_macros)]` if needed.

### Debugging Declarative Macros

```bash
# Expand macros to see generated code
cargo expand                          # requires: cargo install cargo-expand

# Expand a specific item
cargo expand --lib my_module::my_item

# Minimal reproducer: use a small test file
```

Use `dbg!()` inside macro bodies during development. Use `compile_error!()` to emit user-friendly errors:

```rust
macro_rules! only_positive {
    ($n:literal) => {
        if $n <= 0 {
            compile_error!("argument must be a positive literal")
        } else {
            $n
        }
    };
}
```

---

## Part 2: Procedural Macros

### Crate Structure (Mandatory)

Proc macros **must** live in a separate crate with `proc-macro = true`:

```toml
# macros/Cargo.toml
[lib]
proc-macro = true

[dependencies]
syn = { version = "2", features = ["full"] }
quote = "1"
proc-macro2 = "1"
```

```
my-project/
├── Cargo.toml         # workspace
├── src/               # main crate uses macros
└── macros/            # proc-macro crate
    ├── Cargo.toml
    └── src/
        └── lib.rs
```

### The Three Proc Macro Types

| Type | Attribute syntax | Use case |
|------|-----------------|----------|
| `derive` | `#[derive(MyMacro)]` | Add trait impls to structs/enums |
| `attribute` | `#[my_macro(...)]` | Transform any item |
| `function-like` | `my_macro!(...)` | Replace `macro_rules!` with full power |

### Core Crates

- **`proc-macro2`**: Stable token stream type — use this instead of `proc_macro::TokenStream` internally; convert at the boundary
- **`syn`**: Parse Rust syntax from token streams
- **`quote`**: Generate Rust code via `quote!` macro
- **`darling`**: Parse macro attributes ergonomically (optional but recommended)

### Derive Macro Pattern

```rust
// macros/src/lib.rs
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(MyTrait)]
pub fn derive_my_trait(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_my_trait_inner(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn derive_my_trait_inner(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics MyTrait for #name #ty_generics #where_clause {
            fn my_method(&self) -> String {
                stringify!(#name).to_string()
            }
        }
    })
}
```

**Critical pattern:** Always split into a public `#[proc_macro_derive]` shim and a private `_inner` function returning `syn::Result`. This lets you use `?` throughout.

### Derive with Helper Attributes

```rust
#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_builder_inner(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
```

Using `darling` to parse helper attributes:

```rust
use darling::FromField;

#[derive(FromField)]
#[darling(attributes(builder))]
struct FieldOpts {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    #[darling(default)]
    default: Option<syn::Expr>,
    #[darling(default)]
    skip: bool,
}
```

### Attribute Macro Pattern

```rust
#[proc_macro_attribute]
pub fn my_attribute(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as syn::AttributeArgs);  // syn 1.x
    // syn 2.x: parse_macro_input!(attr as syn::Meta)
    let item = parse_macro_input!(item as syn::ItemFn);

    my_attribute_inner(attr, item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn my_attribute_inner(
    _attr: syn::Meta,
    item: syn::ItemFn,
) -> syn::Result<TokenStream2> {
    let fn_name = &item.sig.ident;
    let fn_block = &item.block;
    let fn_sig = &item.sig;
    let fn_vis = &item.vis;

    Ok(quote! {
        #fn_vis #fn_sig {
            println!("entering {}", stringify!(#fn_name));
            let result = (|| #fn_block)();
            println!("leaving {}", stringify!(#fn_name));
            result
        }
    })
}
```

### Function-Like Macro Pattern

```rust
#[proc_macro]
pub fn my_dsl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as MyDslInput);
    generate_code(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

// Custom parse type:
struct MyDslInput {
    name: syn::Ident,
    items: Vec<syn::Expr>,
}

impl syn::parse::Parse for MyDslInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse::<syn::Ident>()?;
        input.parse::<syn::Token![:]>()?;
        let items = input.parse_terminated(syn::Expr::parse, syn::Token![,])?
            .into_iter()
            .collect();
        Ok(Self { name, items })
    }
}
```

### Span Handling: Error Messages That Point Correctly

```rust
// BAD: error points at the macro call site, not the actual problem
return Err(syn::Error::new(proc_macro2::Span::call_site(), "bad field"));

// GOOD: error points at the specific field that's wrong
return Err(syn::Error::new_spanned(&field.ty, "this type is not supported"));

// GOOD: combine multiple errors (shows all errors at once, not just first)
let mut errors = Vec::new();
for field in &fields {
    if let Err(e) = validate_field(field) {
        errors.push(e);
    }
}
let mut combined = errors.remove(0);
for e in errors {
    combined.combine(e);
}
return Err(combined);
```

### Working with `syn` Data Structures

```rust
use syn::{Data, Fields, DeriveInput};

fn extract_fields(input: &DeriveInput) -> syn::Result<&syn::FieldsNamed> {
    match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(fields) => Ok(fields),
            Fields::Unnamed(_) => Err(syn::Error::new_spanned(
                input,
                "only named-field structs are supported",
            )),
            Fields::Unit => Err(syn::Error::new_spanned(
                input,
                "unit structs are not supported",
            )),
        },
        Data::Enum(_) => Err(syn::Error::new_spanned(input, "enums are not supported")),
        Data::Union(_) => Err(syn::Error::new_spanned(input, "unions are not supported")),
    }
}
```

### Generating Unique Identifiers

```rust
use quote::format_ident;

let field_name = &field.ident;
let getter_name = format_ident!("get_{}", field_name.as_ref().unwrap());
let setter_name = format_ident!("set_{}", field_name.as_ref().unwrap());

quote! {
    pub fn #getter_name(&self) -> &Self::Field { &self.#field_name }
    pub fn #setter_name(&mut self, v: Self::Field) { self.#field_name = v; }
}
```

### Handling Generics Correctly

```rust
fn add_trait_bounds(mut generics: syn::Generics, bound: syn::TypeParamBound) -> syn::Generics {
    for param in &mut generics.params {
        if let syn::GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(bound.clone());
        }
    }
    generics
}

// In derive impl:
let mut generics = input.generics.clone();
// Add `T: MyTrait` bound to all type params
let bound: syn::TypeParamBound = syn::parse_quote!(MyTrait);
let generics = add_trait_bounds(generics, bound);
let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

quote! {
    impl #impl_generics MyTrait for #name #ty_generics #where_clause { ... }
}
```

### Testing Proc Macros

**Option 1: `trybuild` — recommended for error messages**

```toml
[dev-dependencies]
trybuild = "1"
```

```rust
// tests/integration_test.rs
#[test]
fn test_compile_pass() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
}

#[test]
fn test_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
}
```

```rust
// tests/ui/pass/basic_derive.rs
use my_crate::MyTrait;

#[derive(MyTrait)]
struct Foo { x: u32 }

fn main() {
    let f = Foo { x: 1 };
    assert_eq!(f.my_method(), "Foo");
}
```

```
// tests/ui/fail/unit_struct.stderr  (golden file — update with TRYBUILD=overwrite)
error: unit structs are not supported
 --> tests/ui/fail/unit_struct.rs:3:1
  |
3 | struct Empty;
  | ^^^^^^^^^^^^^
```

**Option 2: `macro_rules!` wrapper for unit testing**

```rust
// In tests, wrap the expansion for quick smoke tests
#[cfg(test)]
mod tests {
    use my_macros::MyTrait;

    #[derive(Debug, MyTrait)]
    struct TestStruct { name: String }

    #[test]
    fn basic_works() {
        let s = TestStruct { name: "hello".into() };
        assert_eq!(s.my_method(), "TestStruct");
    }
}
```

**Option 3: `cargo expand` for development**

```bash
# See exactly what your macro generates
cargo expand --test integration_test
```

---

## Part 3: Hygiene in Proc Macros

Proc macros use `Span::call_site()` by default — **unhygienic** (identifiers are visible to caller). Use `Span::mixed_site()` for `macro_rules!`-like hygiene:

```rust
// Generates a local `__result` that won't conflict with caller code
let local_var = syn::Ident::new("__result", proc_macro2::Span::mixed_site());
quote! {
    let #local_var = compute();
    #local_var + 1
}
```

---

## Part 4: Patterns and Anti-Patterns

### Patterns

**Recursive macros for tree-like data:**
```rust
macro_rules! json {
    (null) => { Json::Null };
    (true) => { Json::Bool(true) };
    (false) => { Json::Bool(false) };
    ($n:literal) => { Json::Number($n as f64) };
    ($s:literal) => { Json::Str($s.to_string()) };
    ([ $($val:tt),* $(,)? ]) => {
        Json::Array(vec![ $( json!($val) ),* ])
    };
    ({ $($key:literal : $val:tt),* $(,)? }) => {
        Json::Object(vec![ $( ($key.to_string(), json!($val)) ),* ])
    };
}
```

**Internal rules to avoid repetition:**
```rust
macro_rules! my_macro {
    // Public rule
    ($x:expr) => { my_macro!(@internal $x, default_config()) };

    // Internal rule (convention: prefix with @)
    (@internal $x:expr, $cfg:expr) => {
        process_with($x, $cfg)
    };
}
```

### Anti-Patterns

| Anti-Pattern | Problem | Fix |
|-------------|---------|-----|
| `unwrap()` in proc macro | Panics with cryptic ICE message | Return `syn::Error` via `?` |
| `Span::call_site()` for generated names | Name collisions in caller | `Span::mixed_site()` or `__prefixed` names |
| Huge `macro_rules!` (100+ lines) | Unmaintainable, hard to debug | Use proc macro instead |
| Not accepting trailing commas | Annoying for callers | Add `$(,)?` |
| Emitting `compile_error!` as string | No span information | Use `syn::Error::new_spanned` |
| Parsing `TokenStream` manually | Fragile, misses edge cases | Use `syn` |
| One huge match arm | Hard to reason about | Split into helper functions |

---

## Part 5: Common Crate Ecosystem

| Crate | Purpose | When to use |
|-------|---------|-------------|
| `syn` | Parse Rust token streams | All proc macros |
| `quote` | Generate token streams | All proc macros |
| `proc-macro2` | Stable token types | All proc macros |
| `darling` | Parse macro attributes | Derive macros with `#[attr(...)]` helpers |
| `trybuild` | Test error messages | UI/compile-fail tests |
| `proc-macro-error` | Better error reporting | Alternative to manual `syn::Error` (older style) |

---

## Part 6: Checklist Before Shipping a Macro

**Declarative:**
- [ ] Trailing comma accepted: `$(,)?`
- [ ] Double-braced blocks to contain introduced names
- [ ] `#[macro_export]` only if public API
- [ ] Test with edge cases: empty input, single item, many items
- [ ] `cargo expand` output reviewed

**Procedural:**
- [ ] Separate `_inner` function returning `syn::Result`
- [ ] Errors use `new_spanned` pointing at problem token
- [ ] Multiple errors combined, not just first
- [ ] Generics handled via `split_for_impl()`
- [ ] `trybuild` tests for both pass and fail cases
- [ ] `cargo expand` output reviewed for generated code quality
- [ ] Hygiene: introduced identifiers use `Span::mixed_site()` or `__` prefix
