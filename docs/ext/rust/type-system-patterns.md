Forward: I wrote Atlas well after working on Jet, and I had learned a lot more about Rust as a result. And I utilized a lot more of the features, especially of the type system that Rust has to offer. And the result was a much more solid, reliable codebase. Now that we're coming back around to work on Jet some more,  I think it would be wise to apply some of the things that we learned where it fits. So to that end, we've put this document together. I hope that some, if not many, of these ideas will find a useful home in the Jet codebase.

# Type System Patterns in Atlas: A Comprehensive Guide

**Date:** 2026-02-06
**Authors:** Atlas Team
**Purpose:** Documentation of type-driven design patterns used in Atlas to achieve safety, clarity, and performance

---

## Executive Summary

Atlas leverages Rust's powerful type system to eliminate entire categories of bugs at compile time. This document analyzes the sophisticated type-driven design patterns employed throughout the codebase, demonstrating how strong typing transforms domain constraints into compiler-enforced guarantees.

**Key Achievement:** Zero runtime overhead from type safety. All type-level guarantees monomorphize away at compile time, leaving only the minimal machine code needed for business logic.

---

## Table of Contents

1. [The Phantom Data Pattern: Type-Safe IDs](#1-the-phantom-data-pattern-type-safe-ids)
2. [Newtype Pattern: Domain-Constrained Primitives](#2-newtype-pattern-domain-constrained-primitives)
3. [Compile-Time Dispatch: Report Schema Macro](#3-compile-time-dispatch-report-schema-macro)
4. [Zero-Cost Abstractions: Generic Collections](#4-zero-cost-abstractions-generic-collections)
5. [Enum-Driven State Machines](#5-enum-driven-state-machines)
6. [Type-Level Indexing with PhantomData](#6-type-level-indexing-with-phantomdata)
7. [Design by Information](#7-design-by-information)
8. [Performance Characteristics](#8-performance-characteristics)

---

## 1. The Phantom Data Pattern: Type-Safe IDs

### The Problem

In many systems, identifiers (IDs) are represented as simple strings or UUIDs:

```rust
// ❌ Dangerous: All IDs have the same type
let person_id: Ulid = Ulid::new();
let company_id: Ulid = Ulid::new();
let report_id: Ulid = Ulid::new();

// Compiles successfully but is semantically wrong!
if person_id == company_id { /* ... */ }
database.update_person(company_id); // Runtime bug!
```

This creates two critical issues:

1. **Type confusion:** Mixing up IDs of different entity types compiles but causes runtime bugs
2. **Semantic clarity:** Code doesn't express what kind of ID a variable holds

### The Atlas Solution: PhantomData

Atlas uses the `PhantomData` pattern to create type-safe, zero-cost ID wrappers:

```rust
// From crates/das_modell/src/lib.rs
pub struct Id<T>(Ulid, PhantomData<T>);

impl<T> Id<T> {
    pub fn new() -> Self {
        Self(Ulid::new(), PhantomData)
    }
    
    pub const fn ulid(&self) -> Ulid {
        self.0
    }
}
```

**Usage:**

```rust
struct Person;
struct Company;

let person_id: Id<Person> = Id::new();
let company_id: Id<Company> = Id::new();

// ✅ Compile-time error: mismatched types!
// if person_id == company_id { }

// ✅ Type-safe function signatures
fn get_person(id: Id<Person>) -> Person { /* ... */ }
fn get_company(id: Id<Company>) -> Company { /* ... */ }

// ✅ Impossible to mix up
get_person(person_id);   // OK
// get_person(company_id);  // Compile error!
```

### Key Benefits

1. **Zero Runtime Cost:** `PhantomData<T>` is a zero-sized type (ZST). At runtime, `Id<Person>` is identical in size and layout to a bare `Ulid` (16 bytes).

2. **Compile-Time Type Safety:** The type parameter `T` exists only at compile time to provide type checking. All checks are done during compilation and erased before code generation.

3. **Ergonomic Conversions:** The type provides safe conversions while maintaining type safety:

```rust
// From das_modell/src/lib.rs
impl<T> From<Ulid> for Id<T> {
    fn from(ulid: Ulid) -> Self {
        Self::from_ulid(ulid)
    }
}

impl<T> From<Id<T>> for Ulid {
    fn from(id: Id<T>) -> Self {
        id.ulid()
    }
}
```

### Real-World Application

In `atlas-core/src/models/ids.rs`, the same pattern is used with additional features:

```rust
pub struct Id<T>(Ulid, #[serde(skip)] PhantomData<T>);
```

The `#[serde(skip)]` attribute ensures the phantom data doesn't affect serialization, allowing `Id<T>` to serialize as a transparent ULID string.

---

## 2. Newtype Pattern: Domain-Constrained Primitives

### The Problem

Primitive types like `u16`, `u8`, and `String` don't encode domain constraints:

```rust
// ❌ No compile-time guarantees about validity
fn days_in_month(year: u16, month: u8) -> u8 {
    // What if month is 0? Or 13? Or 255?
    // These are representable but semantically invalid!
    DAYS_IN_MONTH[month as usize]  // Possible panic or wrong index
}
```

### The Atlas Solution: Smart Newtypes

Atlas uses the newtype pattern with non-zero types to encode domain constraints:

```rust
// From crates/fuzzy_date/src/types.rs

/// Year: guaranteed to be 1..=9999
#[repr(transparent)]
pub struct Year(NonZeroU16);

impl Year {
    pub fn new(value: u16) -> Result<Self, ParseError> {
        let non_zero = NonZeroU16::new(value)
            .ok_or(ParseError::InvalidYear(value))?;
        if value > MAX_YEAR {
            return Err(ParseError::InvalidYear(value));
        }
        Ok(Self(non_zero))
    }
    
    pub const fn get(self) -> u16 {
        self.0.get()
    }
}
```

### Cascading Type Safety

The newtype pattern creates a cascade of type-level guarantees:

```rust
/// Month: guaranteed to be 1..=12
#[repr(transparent)]
pub struct Month(NonZeroU8);

/// Day: guaranteed to be valid for the given year/month
#[repr(transparent)]  
pub struct Day(NonZeroU8);

impl Day {
    pub fn new(value: u8, year: u16, month: u8) -> Result<Self, ParseError> {
        let non_zero = NonZeroU8::new(value)
            .ok_or(ParseError::InvalidDay { month, day: value, year })?;
        
        let max_day = days_in_month(year, month);
        if value > max_day {
            return Err(ParseError::InvalidDay { month, day: value, year });
        }
        
        Ok(Self(non_zero))
    }
}
```

**Once constructed, these types guarantee validity:**

```rust
// From crates/fuzzy_date/src/lib.rs
pub enum FuzzyDate {
    /// Full date - if this variant exists, the date is VALID
    Day {
        year: Year,   // Guaranteed: 1..=9999
        month: Month, // Guaranteed: 1..=12
        day: Day,     // Guaranteed: valid for this year/month
    },
    Month { year: Year, month: Month },
    Year { year: Year },
}
```

### Benefits of This Approach

1. **Impossible States are Unrepresentable:** You cannot construct an invalid `Day` (e.g., February 30th).

2. **Error Handling at Boundaries:** Validation happens once at construction. Internal functions can use these types without defensive checks:

```rust
// ✅ No validation needed - types guarantee correctness
pub const fn days_in_month(year: u16, month: u8) -> u8 {
    debug_assert!(month != 0 && month <= MAX_MONTH);
    // Safe to index because Month type enforces constraints
    if month == FEBRUARY && is_leap_year(year) {
        FEBRUARY_DAYS_LEAP
    } else {
        DAYS_IN_MONTH[month as usize]
    }
}
```

1. **Self-Documenting Code:** Function signatures communicate constraints:

```rust
fn format_date(year: Year, month: Month, day: Day) -> String {
    // Caller must provide validated types
    format!("{:04}-{:02}-{:02}", year.get(), month.get(), day.get())
}
```

1. **Compiler-Enforced Conversions:** The type system prevents accidental bypass:

```rust
// ❌ Can't accidentally use raw u8 where Month expected
let raw_month: u8 = 15;
format_date(year, raw_month, day);  // Compile error!

// ✅ Must go through validation
let month = Month::new(15)?;  // Runtime error caught
```

### NonZero Optimization

Using `NonZeroU8` and `NonZeroU16` provides an additional benefit: `Option<Year>`, `Option<Month>`, and `Option<Day>` have the same size as their inner types due to niche optimization. Zero becomes the discriminant for `None`.

---

## 3. Compile-Time Dispatch: Report Schema Macro

### The Problem

Traditional approaches to mapping sections to extractors involve runtime lookups or brittle manual associations:

```rust
// ❌ Runtime lookup with possibility for errors
fn get_extractor(section: &str) -> Option<ExtractorFn> {
    match section {
        "Contact" => Some(extract_contact),
        "Family" => Some(extract_family),
        // Easy to forget a section or misspell names!
        _ => None,
    }
}
```

This approach has several issues:

- **No compile-time verification** that all sections have extractors
- **Name synchronization:** Section names hardcoded in multiple places
- **Typo-prone:** String matching is fragile

### The Atlas Solution: Deterministic Derivation

Atlas eliminates all manual mappings using a procedural macro that derives everything from a single source of truth:

```rust
// From crates/atlas-v5/src/report_schema.rs
crate::report_schema! {
    report_type: ReportType::SpokeoPerson,
    pub enum SpkPersonSections {
        Header,
        Contact,
        LocationHistory,
        Family,
        Social,
        // ... more sections
    }
}
```

The `report_schema!` macro (in `crates/report_schema_derive/src/lib.rs`) generates:

1. **Section enum with strum derives** for parsing and iteration
2. **Extractor function names** derived deterministically from the enum
3. **Trait implementations** that wire everything together

### Deterministic Name Derivation

The macro eliminates all possibility of mismatched names:

```rust
// From report_schema_derive/src/lib.rs (simplified)

// Extract vendor abbreviation from ReportType
let (vendor_abbrev, subject_type, schema_name) = 
    extract_report_type_info(&input.report_type)?;
// "SpokeoPerson" → ("spk", "person", "SpkPersonSchema")

// Generate extractor function names deterministically
let extractor_fns: Vec<_> = non_header_variants
    .iter()
    .map(|variant| {
        let variant_lower = variant.to_string().to_lowercase();
        // "Contact" → "extract_spk_person_contact"
        format_ident!("extract_{}_{}_{}", 
            vendor_abbrev, subject_type, variant_lower)
    })
    .collect();
```

**Result:** Zero possibility of mismatched names. The compiler ensures:

1. Every section enum variant has exactly one corresponding extractor function
2. The extractor function name is mechanically derived from the section name
3. Changes to section names automatically update all generated code

### Generated Code

The macro generates type-safe trait implementations:

```rust
// Generated by the macro
impl SectionHeader for SpkPersonSections {
    type Subject = atlas_core::models::Person;
    type SubjectInfo = crate::extractors::PersonHeaderInfo;
    
    fn extract(
        &self,
        acc: &mut crate::extractors::Accumulator<Self::Subject>,
        lines: &[&str],
    ) -> crate::extractors::ExtractResult<()> {
        match self {
            Self::Header => Ok(()),
            // Generated match arms - names guaranteed to align
            Self::Contact => crate::extractors::extract_spk_person_contact(acc, lines),
            Self::LocationHistory => crate::extractors::extract_spk_person_locationhistory(acc, lines),
            Self::Family => crate::extractors::extract_spk_person_family(acc, lines),
            // ... etc
        }
    }
}
```

### Multi-Form Section Matching

The macro also handles the complex problem of section header aliases:

```rust
// From report_schema_derive/src/lib.rs
fn all_parse_forms(ident: &Ident) -> Vec<String> {
    let title = ident.to_string().to_title_case();
    let raw = ident.to_string();
    
    let mut forms = Vec::new();
    forms.push(raw);           // "PossibleRelativesAndAssociates"
    forms.push(title.clone()); // "Possible Relatives And Associates"
    
    // Apply transform rules
    if let Some(transformed) = rule_and_to_ampersand(&title) {
        forms.push(transformed); // "Possible Relatives & Associates"
    }
    if let Some(transformed) = rule_strip_possible(&title) {
        forms.push(transformed); // "Relatives And Associates"
    }
    
    forms
}
```

This generates strum attributes that allow parsing from multiple forms:

```rust
// Generated
#[strum(serialize = "PossibleRelativesAndAssociates")]
#[strum(serialize = "Possible Relatives And Associates")]
#[strum(serialize = "Possible Relatives & Associates")]
#[strum(serialize = "Relatives And Associates")]
PossibleRelativesAndAssociates,
```

### Compile-Time Verification

The macro enforces constraints at compile time:

```rust
// From crates/atlas-v5/src/report_schema.rs
const _: () = {
    use static_assertions::const_assert;
    
    const_assert!(crate::sections::MAX_SECTIONS == 32);
    
    // Verify each section enum fits within the limit
    const_assert!(SPK_PERSON_SECTIONS_COUNT <= crate::sections::MAX_SECTIONS);
    const_assert!(SPK_ADDRESS_SECTIONS_COUNT <= crate::sections::MAX_SECTIONS);
    const_assert!(BV_PERSON_SECTIONS_COUNT <= crate::sections::MAX_SECTIONS);
    const_assert!(BV_ADDRESS_SECTIONS_COUNT <= crate::sections::MAX_SECTIONS);
};
```

Any violation of these constraints causes a **compile-time error**, not a runtime panic.

---

## 4. Zero-Cost Abstractions: Generic Collections

### The Problem

Generic collections often come with overhead:

- HashMap requires heap allocation and hashing
- Vec may over-allocate
- BTreeMap adds pointer indirection

For small, bounded collections with known maximum size, these costs are unnecessary.

### The Atlas Solution: Stack-Allocated Generic Arrays

Atlas implements generic collections backed by stack-allocated arrays:

```rust
// From crates/atlas-v5/src/sections/map.rs

pub struct SectionMap<S: SectionIndex, V, const N: usize = MAX_SECTIONS> {
    storage: [Option<V>; N],
    _phantom: PhantomData<S>,
}
```

**Key characteristics:**

1. **Fixed-size array on the stack:** No heap allocations for the map structure
2. **Generic over section type:** Works with any section enum via the `SectionIndex` trait
3. **Compile-time size:** The size `N` is a const generic, checked at compile time

### Type-Safe Indexing

The section enum itself becomes the key type:

```rust
impl<S: SectionIndex, V, const N: usize> SectionMap<S, V, N> {
    pub fn insert(&mut self, section: S, value: V) {
        let idx = usize::from(section.index());
        debug_assert!(idx < S::count(), "section index out of bounds");
        self.storage[idx] = Some(value);
    }
    
    pub fn get(&self, section: S) -> Option<&V> {
        let idx = usize::from(section.index());
        debug_assert!(idx < S::count(), "section index out of bounds");
        self.storage[idx].as_ref()
    }
}
```

**Benefits:**

- **O(1) access:** Direct array indexing based on enum discriminant
- **Type safety:** Can't use wrong section type as key
- **No hashing overhead:** Enum discriminants are the indices
- **Cache-friendly:** Sequential memory layout

### Generic Bitflags

Similarly, section state is tracked using generic bitflags:

```rust
// From crates/atlas-v5/src/sections/bit_flags.rs

pub struct BitFlags<T> {
    bits: u32,
    _phantom: PhantomData<T>,
}

impl<T: SectionIndex> BitFlags<T> {
    pub fn set_section(&mut self, section: T) {
        self.set(section.to_bitmask());
    }
    
    pub fn is_section_set(&self, section: T) -> bool {
        self.is_set(section.to_bitmask())
    }
}
```

**Usage:**

```rust
// From crates/atlas-v5/src/sections/active_set.rs

pub struct ActiveSetBitflags<S: SectionIndex> {
    inner: BitFlags<S>,
    _phantom: PhantomData<S>,
}

impl<S: SectionIndex> ActiveSet<S> for ActiveSetBitflags<S> {
    fn skip(&mut self, section: S) {
        self.inner.clear_section(section);
    }
    
    fn check(&self, section: S) -> bool {
        self.inner.is_section_set(section)
    }
}
```

This pattern allows tracking up to 32 sections in a single `u32`, with full type safety.

### Monomorphization at Work

Because these structures are generic over the section type, the compiler generates specialized versions for each concrete type:

```rust
// Source code (generic)
let mut map: SectionMap<SpkPersonSections, Vec<String>> = SectionMap::new();
map.insert(SpkPersonSections::Contact, vec!["phone".to_string()]);

// What the compiler generates (monomorphized)
// - Specialized version of SectionMap for SpkPersonSections
// - All generic parameters resolved to concrete types
// - No runtime type checking or dispatching
// - Direct array indexing with known offsets
```

The generic code is a compile-time abstraction. At runtime, only specialized, fully optimized machine code remains.

---

## 5. Enum-Driven State Machines

### The Problem

Tracking parser state with booleans or integers is error-prone:

```rust
// ❌ Fragile state tracking
struct ParserState {
    in_header: bool,
    in_contact: bool,
    header_complete: bool,
    current_section: usize,
}
```

This approach allows invalid states:

- Both `in_header` and `in_contact` could be true
- `header_complete` could be true while `in_header` is also true
- `current_section` could be out of bounds

### The Atlas Solution: Type-Level State

Atlas uses enums and the type system to make invalid states impossible:

```rust
// From crates/atlas-v5/src/sections/engine.rs

pub struct State<S: SectionIndex, AS: ActiveSet<S> = ActiveSetBitflags<S>> {
    current_idx: u8,           // Current section index
    remaining: u8,             // Sections left to process
    history: Vec<(usize, S)>,  // (line_num, section) history
    active: AS,                // Which sections are active
    _phantom: PhantomData<S>,
}
```

**State transitions are enforced by the type system:**

```rust
impl<S, AS> State<S, AS>
where
    S: SectionIndex,
    AS: ActiveSet<S>,
{
    pub fn advance_to(
        &mut self, 
        target: S, 
        line_num: usize
    ) -> Result<(), TransitionError<S>> {
        let target_idx = target.index();
        
        // Enforce forward-only progression
        if target_idx < self.current_idx {
            return Err(TransitionError::out_of_order(
                self.current(),
                target,
                line_num,
            ));
        }
        
        // Check if target section is disabled
        if !self.active.check(target) {
            return Err(TransitionError::disabled_section(
                target,
                line_num,
            ));
        }
        
        // Update state
        self.current_idx = target_idx;
        self.remaining = S::count() as u8 - target_idx - 1;
        self.history.push((line_num, target));
        
        Ok(())
    }
}
```

### Invariants Maintained by Types

1. **Forward-only progression:** The `advance_to` method returns an error if attempting to move backward

2. **Exhaustive section coverage:** The `remaining` counter tracks unprocessed sections

3. **Type-safe current state:** The `current()` method returns the current section as a strongly-typed value

```rust
pub fn current(&self) -> S {
    // SAFETY: current_idx is always valid due to invariants
    // maintained by State construction and advance_to
    unsafe { S::from_u8_unchecked(self.current_idx) }
}
```

### Error Types Carry State Information

Errors preserve the state for debugging:

```rust
// From crates/atlas-v5/src/sections/mod.rs

pub enum TransitionError<S> {
    OutOfOrder {
        expected: S,
        actual: S,
        line_num: usize,
        backtrace: CapturedBacktrace,
    },
    DisabledSection {
        target: S,
        line_num: usize,
        backtrace: CapturedBacktrace,
    },
    Incomplete {
        current: S,
        remaining: usize,
        backtrace: CapturedBacktrace,
    },
    // ... more variants
}
```

The error type is generic over `S`, ensuring errors can only be constructed with valid section values.

---

## 6. Type-Level Indexing with PhantomData

### The Pattern

Throughout Atlas, PhantomData enables type-level associations without runtime cost:

```rust
// SectionMap: Type-safe indexing
pub struct SectionMap<S: SectionIndex, V, const N: usize> {
    storage: [Option<V>; N],
    _phantom: PhantomData<S>,  // Associates storage with section type
}

// BitFlags: Type-safe bit manipulation
pub struct BitFlags<T> {
    bits: u32,
    _phantom: PhantomData<T>,  // Associates bits with entity type
}

// ActiveSetBitflags: Type-safe state tracking
pub struct ActiveSetBitflags<S: SectionIndex> {
    inner: BitFlags<S>,
    _phantom: PhantomData<S>,
}
```

### Why This Matters

Without PhantomData, these collections would be untyped:

```rust
// ❌ Without PhantomData - no type safety
pub struct UnsafeSectionMap<V, const N: usize> {
    storage: [Option<V>; N],
}

// Dangerous: Can mix incompatible section types
let mut map: UnsafeSectionMap<String, 32> = UnsafeSectionMap::new();
map.insert(spokeo_section.index(), "data");
map.get(beenverified_section.index());  // Wrong section enum!
```

With PhantomData, the compiler prevents mixing incompatible types:

```rust
// ✅ With PhantomData - type safe
let mut spk_map: SectionMap<SpkPersonSections, String> = SectionMap::new();
spk_map.insert(SpkPersonSections::Contact, "data");

let mut bv_map: SectionMap<BvPersonSections, String> = SectionMap::new();
// spk_map = bv_map;  // Compile error: incompatible types!
```

---

## 7. Design by Information

Atlas follows a philosophy of **design by information:** encode all known domain facts in the type system.

### Principle: Make Invalid States Unrepresentable

Rather than validate at runtime, use types to prevent invalid states from being constructed:

```rust
// ❌ Runtime validation required
fn process_date(year: u16, month: u8, day: u8) -> Result<(), Error> {
    if month == 0 || month > 12 {
        return Err(Error::InvalidMonth);
    }
    // ... more validation
}

// ✅ Validation at boundaries only
fn process_date(year: Year, month: Month, day: Day) {
    // Types guarantee validity - no checks needed
    let timestamp = compute_timestamp(year.get(), month.get(), day.get());
}
```

### Principle: Exploit Compile-Time Facts

Use const generics and associated types to push computations to compile time:

```rust
// Array size is compile-time constant
pub struct SectionMap<S: SectionIndex, V, const N: usize = MAX_SECTIONS> {
    storage: [Option<V>; N],
    _phantom: PhantomData<S>,
}

// Compiler verifies size matches at compile time
impl<S: SectionIndex, V, const N: usize> SectionMap<S, V, N> {
    pub fn new() -> Self {
        debug_assert!(
            S::count() <= N,
            "Section enum has {} variants, but SectionMap size is {}",
            S::count(), N
        );
        // In release builds, this is a no-op since N is compile-time constant
        // and the assertion is optimized away if true
        Self {
            storage: std::array::from_fn(|_| None),
            _phantom: PhantomData,
        }
    }
}
```

### Principle: Store Information Once

The report_schema! macro embodies this principle: section names are written once, and all other information is derived:

```rust
// Single source of truth
report_schema! {
    report_type: ReportType::SpokeoPerson,
    pub enum SpkPersonSections {
        Contact,        // <- Only place section name appears
        LocationHistory,
        Family,
    }
}

// Everything else is derived:
// - Enum variant names
// - String serializations ("Contact", "contact", "CONTACT")
// - Extractor function names (extract_spk_person_contact)
// - Display implementations
// - FromStr implementations
// - Array indices
// - Bitmask positions
```

---

## 8. Performance Characteristics

### Zero-Cost Abstractions

All type-level abstractions in Atlas have zero runtime cost:

| Abstraction | Runtime Cost | Benefit |
| --- | --- | --- |
| `Id<T>` with PhantomData | 0 bytes overhead | Type-safe IDs |
| `Year`, `Month`, `Day` newtypes | 0 bytes overhead (same as inner type) | Domain constraints |
| `SectionMap<S, V, N>` | 0 bytes overhead vs raw array | Type-safe indexing |
| `BitFlags<T>` | 0 bytes overhead vs raw u32 | Type-safe bit manipulation |
| Generic monomorphization | 0 runtime dispatch | Type-specific optimizations |

### Monomorphization Benefits

Generic code is specialized for each concrete type at compile time:

```rust
// Generic source
fn segment<S: SectionIndex>(text: &str) -> SectionMap<S, Vec<String>> {
    // ... parsing logic
}

// Compiled output (conceptual)
fn segment_spokeo_person(text: &str) -> SectionMap_SpkPersonSections_VecString_9 {
    // Specialized for SpkPersonSections with 9 variants
    // All abstractions resolved to direct machine code
}

fn segment_bv_address(text: &str) -> SectionMap_BvAddressSections_VecString_16 {
    // Specialized for BvAddressSections with 16 variants
    // Completely separate specialization
}
```

Benefits:

- **No virtual dispatch:** Direct function calls
- **Inlining opportunities:** Small specialized functions can be inlined
- **SIMD optimizations:** Compiler can vectorize specialized code
- **Dead code elimination:** Unreachable paths removed per specialization

### Memory Layout Optimization

Repr transparent and NonZero optimizations:

```rust
#[repr(transparent)]
pub struct Year(NonZeroU16);

// Size and alignment identical to NonZeroU16
assert_eq!(size_of::<Year>(), size_of::<NonZeroU16>());
assert_eq!(size_of::<Year>(), size_of::<u16>());

// Option<Year> has same size as Year due to niche optimization
assert_eq!(size_of::<Option<Year>>(), size_of::<Year>());
```

This is critical for dense data structures:

```rust
pub enum FuzzyDate {
    Day { year: Year, month: Month, day: Day },  // 4 bytes + discriminant
    Month { year: Year, month: Month },           // 3 bytes + discriminant
    Year { year: Year },                          // 2 bytes + discriminant
}
// Compared to Option<u16> + Option<u8> + Option<u8> = 6 bytes (with padding)
```

### Compile-Time Verification

Static assertions catch errors at compile time:

```rust
const _: () = {
    use static_assertions::const_assert;
    
    const_assert!(SPK_PERSON_SECTIONS_COUNT <= MAX_SECTIONS);
    // If this fails, compilation stops with clear error
};
```

No runtime checks needed - impossible to violate constraints.

---

## Conclusion

Atlas demonstrates that sophisticated type-level programming in Rust provides:

1. **Safety:** Entire categories of bugs impossible to write
2. **Clarity:** Types document invariants and domain constraints
3. **Performance:** All abstractions compile to minimal machine code

The techniques shown here—PhantomData, newtypes, const generics, procedural macros, and careful trait design—combine to create a system where the compiler acts as a domain expert, enforcing correctness without runtime overhead.

### Key Takeaways for Other Projects

1. **Use PhantomData for type-safe handles:** IDs, indices, tokens - anything that should not be mixed between contexts

2. **Encode constraints in newtypes:** Don't validate repeatedly; validate once at boundaries and use types that guarantee correctness

3. **Leverage procedural macros for mechanical derivation:** Eliminate manual synchronization between related concepts

4. **Generic collections with const generics:** Stack-allocated, zero-overhead alternatives to heap collections

5. **Design by information:** Make invalid states unrepresentable; exploit compile-time facts

The result is code that is simultaneously safer, clearer, and faster than traditional approaches.

---

## Further Reading

- [Rust API Guidelines - Type Safety](https://rust-lang.github.io/api-guidelines/type-safety.html)
- [The Rustonomicon - PhantomData](https://doc.rust-lang.org/nomicon/phantom-data.html)
- [Rust Reference - Type Layout](https://doc.rust-lang.org/reference/type-layout.html)
- [Atlas Architecture Documentation](architecture.md)
- [Atlas Design by Information Guide](pragmatic_rust_guidelines.md) 
