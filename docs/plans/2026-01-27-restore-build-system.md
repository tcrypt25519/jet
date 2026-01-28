# Restore Build System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Restore the Jet project to a buildable and testable state after reconstruction from archived pieces.

**Architecture:** This is a Rust workspace project with two crates (`jet` and `jet_runtime`) that uses LLVM via the Inkwell wrapper to compile and execute EVM bytecode. The project needs to upgrade from LLVM 18.0 to LLVM 21 and update all dependencies.

**Tech Stack:** Rust (edition 2024), LLVM 21, Inkwell 0.8.0, Cargo workspace

---

## Phase 1: Enumerate All Build Issues

### Task 1: Create Issue Tracking System

**Files:**
- Create: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Write the issue tracking document template**

Create a markdown file to track all discovered issues:

```markdown
# Build Restoration Issue Tracker

**Status:** In Progress
**Last Updated:** 2026-01-27

## Critical Issues (Blocks Build)
- [ ] Issue ID: CRIT-001
  - Description:
  - Location:
  - Action Required:

## High Priority (Blocks Tests)
- [ ] Issue ID: HIGH-001
  - Description:
  - Location:
  - Action Required:

## Medium Priority (Warnings/Quality)
- [ ] Issue ID: MED-001
  - Description:
  - Location:
  - Action Required:

## Low Priority (Nice to Have)
- [ ] Issue ID: LOW-001
  - Description:
  - Location:
  - Action Required:

## Resolved Issues
- [x] Issue ID:
  - Description:
  - Resolution:
```

**Step 2: Commit the tracking document**

```bash
git add docs/build-restoration/00-issue-tracker.md
git commit -m "docs: add build restoration issue tracker"
```

### Task 2: Attempt Initial Build and Capture Errors

**Files:**
- Modify: `docs/build-restoration/00-issue-tracker.md`
- Create: `docs/build-restoration/01-initial-build-output.txt`

**Step 1: Run cargo check and capture output**

Run: `cargo check 2>&1 | tee docs/build-restoration/01-initial-build-output.txt`
Expected: FAIL with compilation errors related to missing dependencies, API changes, etc.

**Step 2: Analyze and categorize errors**

Review the output and categorize each unique error:
- Dependency resolution failures
- Missing types/functions (API changes)
- Feature flag mismatches
- Edition compatibility issues
- LLVM version mismatches

**Step 3: Update issue tracker with discovered errors**

For each error found, add an entry to the issue tracker with:
- Unique ID (CRIT-XXX, HIGH-XXX, etc.)
- Full error message
- Affected files
- Root cause hypothesis
- Required action

**Step 4: Commit the captured output and updated tracker**

```bash
git add docs/build-restoration/01-initial-build-output.txt docs/build-restoration/00-issue-tracker.md
git commit -m "docs: capture initial build errors and categorize issues"
```

### Task 3: Check for Missing Source Files

**Files:**
- Create: `docs/build-restoration/02-source-file-audit.md`
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Create source file inventory**

```markdown
# Source File Audit

## Expected Structure (from Cargo.toml)

### crates/jet
- lib: src/lib.rs
  - modules: builder, engine, instructions
- bin: src/bin/jetdbg.rs (or jetdbg/main.rs)

### crates/jet_runtime
- lib: src/lib.rs
  - modules: binding, builtins, exec, symbols

## File Status

| Path | Exists | Type | Issues |
|------|--------|------|--------|
| crates/jet/src/lib.rs | ✓ | File | - |
| crates/jet/src/builder | ? | ? | Check if file or dir |
| crates/jet/src/engine | ? | ? | Check if file or dir |
| crates/jet/src/instructions.rs | ✓ | File | - |
| crates/jet/src/bin/jetdbg.rs | ? | ? | Binary entry point |
```

**Step 2: Run filesystem checks**

```bash
find crates/ -type f -name "*.rs" | sort > docs/build-restoration/actual-files.txt
find crates/ -type d -name "src" -o -name "bin" | sort >> docs/build-restoration/actual-dirs.txt
```

**Step 3: Compare expected vs actual structure**

Update the audit document with findings and add any missing file issues to the issue tracker.

**Step 4: Check for orphaned or duplicate files**

Look for files like `lib_2.rs`, `exec_2.rs` mentioned in git status that might be incomplete reconstructions.

**Step 5: Commit the audit**

```bash
git add docs/build-restoration/02-source-file-audit.md docs/build-restoration/*.txt docs/build-restoration/00-issue-tracker.md
git commit -m "docs: audit source file structure"
```

### Task 4: Check LLVM Installation and Version

**Files:**
- Create: `docs/build-restoration/03-llvm-environment.md`
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Document LLVM environment requirements**

```markdown
# LLVM Environment Setup

## Target Configuration
- LLVM Version: 21.x
- Inkwell Version: 0.8.0
- Feature Flags: llvm21-1, llvm-sys-210

## Current System

### LLVM Installation Check
```bash
# Check for LLVM installations
which llvm-config
llvm-config --version

# Check for multiple LLVM versions
ls -la /usr/local/opt/ | grep llvm
ls -la /usr/lib/ | grep llvm
```

### Environment Variables
```bash
echo $LLVM_SYS_210_PREFIX
echo $LLVM_SYS_180_PREFIX
```

## Installation Steps (if needed)

### macOS (Homebrew)
```bash
brew install llvm@21
export LLVM_SYS_210_PREFIX=/usr/local/opt/llvm@21
```

### Ubuntu/Debian
```bash
wget https://apt.llvm.org/llvm.sh
chmod +x llvm.sh
sudo ./llvm.sh 21
export LLVM_SYS_210_PREFIX=/usr/lib/llvm-21
```

### Termux (Android)
```bash
pkg install llvm
# Note: Termux may have limited LLVM version options
```
```

**Step 2: Run LLVM discovery commands**

```bash
bash -c "which llvm-config && llvm-config --version" 2>&1 | tee -a docs/build-restoration/03-llvm-environment.md
```

**Step 3: Update issue tracker**

Add LLVM installation/version issues to tracker if current LLVM is not version 21.

**Step 4: Commit findings**

```bash
git add docs/build-restoration/03-llvm-environment.md docs/build-restoration/00-issue-tracker.md
git commit -m "docs: document LLVM environment requirements and status"
```

---

## Phase 2: Update Dependencies

### Task 5: Update Inkwell to 0.8.0 with LLVM 21

**Files:**
- Modify: `crates/jet/Cargo.toml`
- Modify: `crates/jet_runtime/Cargo.toml`
- Modify: `Makefile`
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Update jet/Cargo.toml dependencies**

Change lines 21-22 in `crates/jet/Cargo.toml`:

From:
```toml
inkwell = { rev = "6c0fb56b3554e939f9ca61b465043d6a84fb7b95", features = ["llvm18-0", "llvm-sys-180"], git = "https://github.com/TheDan64/inkwell.git" }
llvm-sys = { package = "llvm-sys", version = "180.0.0" }
```

To:
```toml
inkwell = { version = "0.8.0", features = ["llvm21-1"] }
llvm-sys = { package = "llvm-sys", version = "210" }
```

**Step 2: Update jet_runtime/Cargo.toml dependencies**

Change line 14 in `crates/jet_runtime/Cargo.toml`:

From:
```toml
inkwell = { rev = "6c0fb56b3554e939f9ca61b465043d6a84fb7b95", features = ["llvm18-0", "llvm-sys-180"], git = "https://github.com/TheDan64/inkwell.git" }
```

To:
```toml
inkwell = { version = "0.8.0", features = ["llvm21-1"] }
```

**Step 3: Update Makefile LLVM prefix**

Change line 1 in `Makefile`:

From:
```makefile
export LLVM_SYS_180_PREFIX=/usr/local/opt/llvm
```

To:
```makefile
export LLVM_SYS_210_PREFIX=/usr/local/opt/llvm@21
```

**Step 4: Set environment variable**

```bash
export LLVM_SYS_210_PREFIX=/usr/local/opt/llvm@21
```

Note: Adjust path based on actual LLVM 21 installation location from Task 4.

**Step 5: Run cargo update**

Run: `cargo update 2>&1 | tee -a docs/build-restoration/01-initial-build-output.txt`
Expected: Dependencies resolved, possibly with some version conflicts

**Step 6: Commit dependency updates**

```bash
git add crates/jet/Cargo.toml crates/jet_runtime/Cargo.toml Makefile
git commit -m "deps: upgrade inkwell to 0.8.0 with LLVM 21 support"
```

### Task 6: Update All Other Dependencies

**Files:**
- Modify: `crates/jet/Cargo.toml`
- Modify: `crates/jet_runtime/Cargo.toml`
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Check for outdated dependencies**

Run: `cargo outdated 2>&1 | tee docs/build-restoration/04-outdated-deps.txt`

Note: If cargo-outdated is not installed, skip this and manually check major dependencies.

**Step 2: Update dependencies with known breaking changes**

Review each dependency and update to latest compatible version:
- log: 0.4 → 0.4 (latest patch)
- simple_logger: 5.0.0 → latest 5.x
- clap: 4.5.4 → latest 4.x
- serde: 1.0.201 → latest 1.x
- thiserror: 1.0.61 → latest 1.x
- sha3: 0.10.8 → latest 0.10.x
- colored: 2.1.0 → latest 2.x
- cc: 1.0 → latest 1.x

**Step 3: Run cargo update to get latest compatible versions**

Run: `cargo update`
Expected: All dependencies updated within semver constraints

**Step 4: Commit dependency updates**

```bash
git add Cargo.toml crates/*/Cargo.toml Cargo.lock
git commit -m "deps: update all dependencies to latest compatible versions"
```

---

## Phase 3: Fix Compilation Errors

### Task 7: Fix Inkwell API Changes

**Files:**
- Modify: Files identified in Task 2 with Inkwell API errors
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Attempt build to see current errors**

Run: `cargo build 2>&1 | head -50 | tee -a docs/build-restoration/05-inkwell-api-errors.txt`
Expected: FAIL with type errors, missing methods, etc.

**Step 2: Research Inkwell 0.8.0 API changes**

Check Inkwell changelog and migration guide:
- Breaking changes between 0.x and 0.8.0
- Renamed types/methods
- Changed function signatures

Common changes to look for:
- Context creation/initialization
- Module building patterns
- Function signature changes
- Type conversions

**Step 3: Fix one error at a time**

For each compilation error:
1. Read the affected file
2. Understand the old API usage
3. Look up the new API in Inkwell docs
4. Make the minimal change to fix
5. Test with `cargo check`

**Step 4: Commit after each file is fixed**

```bash
git add path/to/fixed/file.rs
git commit -m "fix: update inkwell API usage in <component>"
```

**Step 5: Update issue tracker**

Mark Inkwell API issues as resolved.

### Task 8: Fix Missing Source Files

**Files:**
- Create/Modify: Files identified as missing in Task 3
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Review missing file list from Task 3**

Check which files are module directories vs single files.

**Step 2: For each missing file**

If `src/builder` is a directory:
1. Create `src/builder/mod.rs` to declare submodules
2. Check for builder/*.rs files that should be included

If binary entry point is missing:
1. Check if `src/bin/jetdbg.rs` or `src/bin/jetdbg/main.rs` exists
2. Create minimal main function if missing:

```rust
use jet::engine::Engine;

fn main() {
    println!("jetdbg - Jet EVM debugger");
    // Minimal implementation
}
```

**Step 3: Test compilation after each file**

Run: `cargo check`
Expected: New errors or fewer errors

**Step 4: Commit each file fix**

```bash
git add path/to/fixed/file.rs
git commit -m "fix: restore missing <component> file"
```

### Task 9: Handle Orphaned Files

**Files:**
- Delete or integrate: `crates/jet_runtime/src/exec_2.rs`, `crates/jet_runtime/src/lib_2.rs`
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Examine orphaned files**

Read each `*_2.rs` file to understand:
- Is it a duplicate of existing file?
- Is it a newer/better version?
- Is it incomplete reconstruction?

**Step 2: Compare with primary files**

```bash
diff crates/jet_runtime/src/lib.rs crates/jet_runtime/src/lib_2.rs
```

**Step 3: Decide action for each file**

Options:
- Delete if duplicate/older version
- Merge if contains missing functionality
- Rename and fix if it's the correct version

**Step 4: Execute and commit**

```bash
# If deleting:
git rm crates/jet_runtime/src/lib_2.rs
git commit -m "chore: remove duplicate lib_2.rs file"

# If merging:
# Manually merge content, then:
git add crates/jet_runtime/src/lib.rs
git rm crates/jet_runtime/src/lib_2.rs
git commit -m "fix: merge lib_2.rs content into lib.rs"
```

### Task 10: Fix Rust Edition Issues

**Files:**
- Modify: `crates/jet/Cargo.toml`
- Modify: `crates/jet_runtime/Cargo.toml`
- Modify: Source files with edition-specific errors
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Check edition consistency**

Current state:
- Workspace: Uses `edition2024` (unstable feature)
- jet crate: `edition = "2024"`
- jet_runtime crate: `edition = "2021"`

**Step 2: Decide on edition strategy**

Options:
1. Use stable edition 2021 everywhere (safe)
2. Use nightly with edition 2024 (experimental)

Recommendation: Use edition 2021 for stability.

**Step 3: Update Cargo.toml files**

In `Cargo.toml`, change line 1:
```toml
# Remove: cargo-features = ["edition2024"]
```

In `crates/jet/Cargo.toml`, change line 5:
```toml
edition = "2021"
```

Remove line 1:
```toml
# Remove: cargo-features = ["edition2024"]
```

**Step 4: Fix edition-specific code**

If any code uses edition 2024 features, update to edition 2021 compatible syntax.

**Step 5: Test build**

Run: `cargo check`
Expected: Fewer errors, no edition-related errors

**Step 6: Commit**

```bash
git add Cargo.toml crates/*/Cargo.toml
git commit -m "fix: use stable Rust edition 2021"
```

### Task 11: Fix Feature Flag Issues

**Files:**
- Modify: `crates/jet/src/lib.rs`
- Modify: Other files using nightly features
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Identify feature flag usage**

Current: `#![feature(allocator_api)]` in `crates/jet/src/lib.rs:1`

**Step 2: Check if feature is still needed**

Search for usage of allocator_api in the codebase:
```bash
grep -r "allocator_api" crates/
grep -r "Allocator" crates/
```

**Step 3: Options based on findings**

If used:
- Keep feature flag and require nightly Rust
- Refactor code to not use allocator_api

If not used:
- Remove the feature flag

**Step 4: Make changes**

If removing:
```rust
// Remove line 1 from crates/jet/src/lib.rs:
// #![feature(allocator_api)]
```

**Step 5: Test**

Run: `cargo check`
Expected: Build progresses further

**Step 6: Commit**

```bash
git add crates/jet/src/lib.rs
git commit -m "fix: remove unused allocator_api feature flag"
```

---

## Phase 4: Build and Test

### Task 12: Achieve Clean Build

**Files:**
- Modify: `docs/build-restoration/00-issue-tracker.md`
- Create: `docs/build-restoration/06-build-success.txt`

**Step 1: Run full build**

Run: `cargo build 2>&1 | tee docs/build-restoration/06-build-success.txt`
Expected: SUCCESS (possibly with warnings)

**Step 2: If build fails, iterate**

Return to Task 7-11 based on error type and fix remaining issues.

**Step 3: Address warnings**

Run: `cargo build 2>&1 | grep warning > docs/build-restoration/07-warnings.txt`

For each warning, decide:
- Fix now if critical (unused imports, deprecated APIs)
- Document for later if cosmetic

**Step 4: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings 2>&1 | tee docs/build-restoration/08-clippy.txt`
Expected: PASS or fixable warnings

**Step 5: Fix critical clippy issues**

Address issues that could cause runtime bugs:
- Potential panics
- Logic errors
- Undefined behavior

**Step 6: Commit successful build**

```bash
git add docs/build-restoration/06-build-success.txt
git commit -m "build: achieve clean compilation"
```

### Task 13: Discover and Document Tests

**Files:**
- Create: `docs/build-restoration/09-test-inventory.md`
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Find all test files**

```bash
find crates/ -name "*.rs" -exec grep -l "#\[test\]" {} \; > docs/build-restoration/test-files.txt
find crates/ -type d -name "tests" >> docs/build-restoration/test-files.txt
```

**Step 2: List all tests**

Run: `cargo test -- --list > docs/build-restoration/test-list.txt`
Expected: List of all test names

**Step 3: Create test inventory document**

```markdown
# Test Inventory

## Unit Tests
[List tests by module]

## Integration Tests
[List integration tests]

## Test Status
- Total tests: X
- Passing: ?
- Failing: ?
- Ignored: ?

## Test Categories
- [ ] Instruction tests
- [ ] Engine tests
- [ ] Builder tests
- [ ] Runtime tests
```

**Step 4: Commit inventory**

```bash
git add docs/build-restoration/09-test-inventory.md docs/build-restoration/test-*.txt
git commit -m "docs: create test inventory"
```

### Task 14: Run Tests and Capture Failures

**Files:**
- Create: `docs/build-restoration/10-test-failures.txt`
- Modify: `docs/build-restoration/09-test-inventory.md`
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Run all tests**

Run: `cargo test 2>&1 | tee docs/build-restoration/10-test-failures.txt`
Expected: SOME FAIL, SOME PASS

**Step 2: Categorize test failures**

For each failure, determine:
- LLVM version compatibility issue
- Missing runtime dependencies
- Incorrect test expectations
- Actual bugs in implementation

**Step 3: Update test inventory**

Add pass/fail status for each test or test module.

**Step 4: Update issue tracker**

Add each unique test failure as an issue:
```markdown
## High Priority (Blocks Tests)
- [ ] Issue ID: HIGH-001
  - Description: test_add_instruction fails with "context not initialized"
  - Location: crates/jet/tests/instructions.rs:45
  - Action Required: Initialize LLVM context in test setup
```

**Step 5: Commit test results**

```bash
git add docs/build-restoration/10-test-failures.txt docs/build-restoration/09-test-inventory.md docs/build-restoration/00-issue-tracker.md
git commit -m "test: capture initial test failures"
```

### Task 15: Fix High-Priority Test Failures

**Files:**
- Modify: Test files identified in Task 14
- Modify: Implementation files as needed
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Pick first high-priority failure**

Focus on blockers like:
- Test harness setup issues
- Global initialization problems
- Common utility failures

**Step 2: Write failing test (if missing)**

Ensure test actually tests the right behavior:
```rust
#[test]
fn test_add_instruction() {
    let context = Context::new();
    let engine = Engine::new(&context);
    let result = engine.execute_add(2, 3);
    assert_eq!(result, 5);
}
```

**Step 3: Run test to confirm failure**

Run: `cargo test test_add_instruction`
Expected: FAIL with specific error

**Step 4: Fix implementation**

Make minimal change to pass the test.

**Step 5: Run test to confirm pass**

Run: `cargo test test_add_instruction`
Expected: PASS

**Step 6: Commit**

```bash
git add tests/path/test.rs src/path/impl.rs
git commit -m "test: fix add_instruction test"
```

**Step 7: Repeat for each high-priority failure**

Work through test failures one at a time.

---

## Phase 5: Documentation and Validation

### Task 16: Update README

**Files:**
- Modify: `README.md`
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Update version requirements**

Change lines 17-22 in `README.md`:

From:
```markdown
- Rust (latest stable version)
- LLVM 18.0
```

To:
```markdown
- Rust 1.92+ (stable)
- LLVM 21.x
```

**Step 2: Update build instructions**

Update LLVM installation steps for version 21.

Change line 52:
```markdown
export LLVM_SYS_210_PREFIX=/usr/local/opt/llvm@21
```

**Step 3: Add troubleshooting section**

```markdown
## Troubleshooting

### LLVM Not Found
If you get errors about LLVM not being found:
1. Ensure LLVM 21 is installed
2. Set the environment variable: `export LLVM_SYS_210_PREFIX=/path/to/llvm-21`
3. Try: `llvm-config --version` to verify installation

### Build Errors
If the build fails:
1. Ensure you're using Rust 1.92 or later: `rustc --version`
2. Clean the build: `cargo clean`
3. Try: `cargo build -vv` for verbose output
```

**Step 4: Commit**

```bash
git add README.md
git commit -m "docs: update README for LLVM 21 and current setup"
```

### Task 17: Create Final Status Report

**Files:**
- Create: `docs/build-restoration/11-final-status.md`
- Modify: `docs/build-restoration/00-issue-tracker.md`

**Step 1: Write final status document**

```markdown
# Build Restoration Final Status

**Date:** 2026-01-27
**Status:** COMPLETE / IN PROGRESS / BLOCKED

## Summary

The Jet project has been successfully restored from archived pieces with the following outcomes:

### Build Status
- [x] Project compiles cleanly
- [x] All dependencies updated
- [x] LLVM 21 integration complete

### Test Status
- Passing: X/Y tests
- Failing: Z tests
- Coverage: XX%

### Remaining Issues
[Link to issue tracker for outstanding items]

## Changes Made

### Dependencies
- Upgraded Inkwell: git commit → 0.8.0
- Upgraded LLVM: 18.0 → 21.x
- Updated all dependencies to latest compatible versions

### Code Changes
[List major code changes made]

### Documentation
[List documentation updates]

## Next Steps

1. [Priority 1 remaining work]
2. [Priority 2 remaining work]
3. [Priority 3 remaining work]

## References
- Issue Tracker: `docs/build-restoration/00-issue-tracker.md`
- Test Inventory: `docs/build-restoration/09-test-inventory.md`
- Build Outputs: `docs/build-restoration/*.txt`
```

**Step 2: Run final validation**

```bash
cargo clean
cargo build --release
cargo test
cargo clippy --all-targets
```

**Step 3: Update status in final report**

Fill in actual numbers and status.

**Step 4: Commit**

```bash
git add docs/build-restoration/11-final-status.md docs/build-restoration/00-issue-tracker.md
git commit -m "docs: add final build restoration status report"
```

### Task 18: Create PR-Ready Branch

**Files:**
- None (git operations only)

**Step 1: Review all changes**

```bash
git log --oneline master..HEAD
git diff master...HEAD --stat
```

**Step 2: Ensure commit messages follow conventions**

All commits should follow pattern:
- `feat:` - new features
- `fix:` - bug fixes
- `docs:` - documentation
- `test:` - test changes
- `deps:` - dependency updates
- `chore:` - maintenance

**Step 3: Create summary of changes**

```bash
git log --oneline master..HEAD > docs/build-restoration/12-commit-summary.txt
```

**Step 4: Tag the restore point**

```bash
git tag -a v0.1.0-restored -m "Build system restored with LLVM 21"
```

**Step 5: Push branch**

```bash
git push origin HEAD
git push origin v0.1.0-restored
```

---

## Appendix: Key Files Reference

### Configuration Files
- `Cargo.toml` (workspace root)
- `crates/jet/Cargo.toml`
- `crates/jet_runtime/Cargo.toml`
- `Makefile`

### Source Structure
```
crates/
├── jet/
│   └── src/
│       ├── lib.rs
│       ├── builder/
│       ├── engine/
│       └── instructions.rs
└── jet_runtime/
    └── src/
        ├── lib.rs
        ├── binding/
        ├── builtins/
        ├── exec/
        └── symbols/
```

### Documentation
- `README.md`
- `docs/build-restoration/` (all restoration docs)

---

## Execution Notes

**Estimated Time:** 4-6 hours (depending on number of API changes)

**Prerequisites:**
- LLVM 21 installed on system
- Rust 1.92+ installed
- Git configured

**Skills Referenced:**
- @superpowers:test-driven-development (for Task 15)
- @superpowers:verification-before-completion (for Task 17)
- @git-commits (for all commits)

**Success Criteria:**
1. `cargo build` succeeds
2. At least 80% of tests passing
3. All critical issues documented
4. README updated with current instructions
