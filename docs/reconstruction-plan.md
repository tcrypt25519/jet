# Jet Reconstruction Plan

**Status:** Draft
**Date:** 2026-01-27
**Objective:** Restore the Jet repository to a buildable and testable state, upgrading to LLVM 21 and latest Inkwell

---

## Executive Summary

The Jet repository has been reconstructed from archived pieces and currently does not build. This document outlines a comprehensive plan to identify, enumerate, and systematically fix all issues to restore the project to a working state. The primary focus is upgrading dependencies, particularly Inkwell and LLVM to version 21, and resolving all build and test failures.

---

## Current State Assessment

### Project Structure
- **Type:** Rust workspace with 2 crates
  - `crates/jet` - Main library and `jetdbg` binary
  - `crates/jet_runtime` - Runtime library with dylib/lib output
- **Build System:** Cargo with Makefile wrapper
- **Source Files:** 16 Rust files, 1 LLVM IR file (`runtime-ir/jet.ll`)
- **Tests:** Located in `crates/jet/tests/`

### Current Dependencies
```toml
# From crates/jet/Cargo.toml
inkwell = { rev = "6c0fb56b3554e939f9ca61b465043d6a84fb7b95",
            features = ["llvm18-0", "llvm-sys-180"],
            git = "https://github.com/TheDan64/inkwell.git" }
llvm-sys = { version = "180.0.0" }
```

**Current LLVM Version:** 18.0 (outdated git revision)
**Target LLVM Version:** 21
**Target Inkwell Version:** 0.8.0

### System Environment
- **LLVM Installed:** 18.1.3 at `/usr/lib/llvm-18`
- **Missing:** LLVM development headers (`llvm-18-dev` package not installed)
- **Available:** `llvm-config` tool

---

## Issues Identified

### Critical Issues (Build Blockers)

#### 1. Missing LLVM Development Headers
**Error:**
```
fatal error: llvm-c/Target.h: No such file or directory
```

**Cause:** The `llvm-18-dev` package is not installed. The system has the LLVM runtime and tools but not the C headers required to build `llvm-sys`.

**Impact:** Blocks all builds immediately during `llvm-sys` compilation.

**Required Actions:**
- Install LLVM 21 development headers (not LLVM 18)
- Update system environment variables
- Verify header paths

#### 2. Outdated Dependencies
**Current Issues:**
- Using a specific git commit of Inkwell instead of a stable release
- LLVM 18.0 dependencies instead of 21.x
- `llvm-sys` version 180.0.0 needs upgrade to 211.0.0

**Impact:**
- Missing bug fixes and improvements
- API incompatibilities with LLVM 21
- Maintenance difficulties

**Required Actions:**
- Upgrade to Inkwell 0.8.0 from crates.io
- Update feature flags from `llvm18-0` to `llvm21-1`
- Update `llvm-sys` to `211.0.0`

#### 3. Deprecated Cargo Features
**Warning:**
```
the cargo feature `edition2024` has been stabilized in the 1.85 release
and is no longer necessary
```

**Impact:** Minor - generates warnings but doesn't block builds

**Required Actions:**
- Remove `cargo-features = ["edition2024"]` from all Cargo.toml files
- Verify edition field remains set to appropriate value

### Potential Issues (To Be Verified)

#### 4. LLVM API Breaking Changes
**Concern:** LLVM 18 to 21 may have API changes affecting:
- Builder API methods (e.g., `build_struct_gep`)
- Execution Engine interface
- Memory buffer handling
- Type system changes

**Files at Risk:**
- `crates/jet/src/builder/contract.rs` - Uses builder API extensively
- `crates/jet/src/builder/ops.rs` - LLVM operations
- `crates/jet/src/engine/mod.rs` - Execution engine and JIT
- `crates/jet_runtime/src/exec.rs` - Runtime execution

**Required Actions:**
- Attempt compilation after dependency upgrade
- Document all API errors
- Consult Inkwell 0.8.0 changelog and migration guide
- Fix each API incompatibility

#### 5. LLVM IR Compatibility
**File:** `runtime-ir/jet.ll`

**Concerns:**
- Target triple is macOS-specific: `"x86_64-apple-macosx14.0.0"`
- Data layout may not match Linux/LLVM 21
- IR syntax changes between LLVM 18 and 21

**Required Actions:**
- Verify IR loads correctly in LLVM 21
- Update target triple to be platform-agnostic or Linux-specific
- Regenerate or update IR if needed

#### 6. Runtime C/C++ Integration
**Observed:**
- `jet_runtime` has build dependencies on `cc = "1.0"`
- No C/C++ source files found in `crates/jet_runtime`
- LLVM IR declares external Rust functions

**Potential Issue:** Build script may attempt to compile C code or link against LLVM libraries.

**Required Actions:**
- Investigate if build.rs exists and what it does
- Verify it works with LLVM 21

#### 7. Test Failures
**Location:** `crates/jet/tests/test_roms.rs` and `crates/jet/tests/roms/`

**Concerns:**
- Tests likely depend on JIT execution working correctly
- EVM contract test ROMs may expose runtime issues
- Integration tests may fail due to API changes

**Required Actions:**
- Run tests after build succeeds
- Document all test failures
- Fix runtime issues
- Verify EVM execution correctness

---

## Dependencies to Upgrade

### Primary Dependencies

| Package | Current Version | Target Version | Feature Changes |
|---------|----------------|----------------|-----------------|
| inkwell | git rev 6c0fb56 | 0.8.0 | `llvm18-0` → `llvm21-1` |
| llvm-sys | 180.0.0 | 211.0.0 | Remove explicit dependency |
| LLVM (system) | 18.1.3 | 21.x | Install llvm-21-dev |

### Secondary Dependencies (May Need Updates)

| Package | Current Version | Notes |
|---------|----------------|-------|
| log | 0.4 | Likely OK |
| simple_logger | 5.0.0 | Check latest (5.1.0 available) |
| clap | 4.5.4 | Check latest (4.5.x) |
| serde | 1.0.201 | Check latest (1.0.x) |
| syntect | 5.2.0 | Check latest (5.3.0 available) |
| libc | 0.2.154 | Check latest (0.2.x) |
| thiserror | 1.0.61 | Check latest (2.0.x available - breaking) |
| sha3 | 0.10.8 | Check latest |
| hex | 0.4.3 | Likely OK |

**Note:** Thiserror has a major version update (2.0.x) that may have breaking changes. Recommend staying on 1.0.x unless needed.

---

## Implementation Plan

### Phase 1: Environment Setup
**Goal:** Install LLVM 21 and prepare the build environment

**Tasks:**
1. **Install LLVM 21**
   ```bash
   # Remove or keep LLVM 18 (for comparison)
   # Install LLVM 21
   wget https://apt.llvm.org/llvm.sh
   chmod +x llvm.sh
   sudo ./llvm.sh 21

   # Install development headers
   sudo apt-get install llvm-21-dev
   ```

2. **Update Environment Variables**
   - Update `Makefile`:
     ```makefile
     export LLVM_SYS_210_PREFIX=/usr/lib/llvm-21
     ```
   - Or set in shell:
     ```bash
     export LLVM_SYS_210_PREFIX=/usr/lib/llvm-21
     ```

3. **Verify Installation**
   ```bash
   llvm-config-21 --version
   ls /usr/lib/llvm-21/include/llvm-c/
   ```

**Success Criteria:**
- LLVM 21 installed with development headers
- `llvm-config-21` accessible
- Headers visible at expected path

---

### Phase 2: Dependency Updates
**Goal:** Update all Cargo.toml files to use LLVM 21

**Tasks:**

1. **Update Root Cargo.toml**
   ```toml
   # Remove this line:
   cargo-features = ["edition2024"]

   [workspace]
   members = [
       "crates/jet",
       "crates/jet_runtime"
   ]
   resolver = "3"
   ```

2. **Update crates/jet/Cargo.toml**
   ```toml
   # Remove this line:
   cargo-features = ["edition2024"]

   [package]
   name = "jet"
   edition = "2024"  # Keep this
   # ... rest stays same

   [dependencies]
   log = "0.4"
   simple_logger = "5.1.0"  # Update minor version
   inkwell = { version = "0.8.0", features = ["llvm21-1"] }  # CHANGED
   # Remove llvm-sys explicit dependency - inkwell will handle it
   clap = { version = "4.5.4", features = ["derive"] }
   serde = { version = "1.0.201", features = ["derive"] }
   syntect = "5.3.0"  # Update
   libc = "0.2.180"  # Update
   paste = "1.0.15"
   thiserror = "1.0.69"  # Update but stay on 1.0.x
   jet_runtime = { path = "../jet_runtime" }
   hex = "0.4.3"
   ```

3. **Update crates/jet_runtime/Cargo.toml**
   ```toml
   # Remove this line:
   cargo-features = ["edition2024"]

   [package]
   name = "jet_runtime"
   version = "0.1.0"
   edition = "2021"  # Keep this or update to 2024

   [lib]
   crate-type = ["dylib", "lib"]

   [dependencies]
   log = "0.4"
   simple_logger = "5.1.0"
   inkwell = { version = "0.8.0", features = ["llvm21-1"] }  # CHANGED
   sha3 = "0.10.8"
   hex = "0.4.3"
   colored = "2.2.0"  # Or update to 3.1.1 (breaking changes)

   [build-dependencies]
   cc = "1.2"  # Update
   ```

4. **Clean Build Artifacts**
   ```bash
   cargo clean
   rm -rf target/
   rm Cargo.lock  # Force regeneration with new versions
   ```

**Success Criteria:**
- All Cargo.toml files updated
- Cargo.lock regenerated
- Dependencies fetch successfully

---

### Phase 3: Initial Build Attempt
**Goal:** Attempt to build and enumerate all compiler errors

**Tasks:**

1. **Run Build**
   ```bash
   cargo build 2>&1 | tee build-errors-phase3.txt
   ```

2. **Categorize Errors**
   - API method signature changes
   - Removed/renamed methods
   - Type changes
   - Module reorganization
   - Other compilation errors

3. **Document Each Error**
   Create `docs/api-migration-log.md` tracking:
   - File and line number
   - Error message
   - Old API usage
   - New API required
   - Fix applied

**Success Criteria:**
- Complete enumeration of all errors
- Errors categorized by type
- Documentation ready for fixes

---

### Phase 4: API Migration Fixes
**Goal:** Fix all API incompatibilities between Inkwell 0.4/LLVM 18 and Inkwell 0.8/LLVM 21

**Strategy:**
1. Start with lowest-level modules first (types, utilities)
2. Move to mid-level (builder operations)
3. Finish with high-level (engine, integration)

**Common API Changes to Expect:**

Based on typical LLVM version upgrades:

- **Builder methods may require explicit types:**
  ```rust
  // Old (LLVM 18)
  builder.build_struct_gep(struct_ty, ptr, index, name)

  // New (LLVM 21) - may need Result handling
  builder.build_struct_gep(struct_ty, ptr, index, name)?
  ```

- **Method return types may change:**
  - Direct returns → `Result<T, E>`
  - Option wrapping changes
  - Error handling improvements

- **Type construction changes:**
  - Context-specific type methods may have new signatures
  - Integer types, pointer types may have new APIs

**Files to Fix (Priority Order):**

1. `crates/jet/src/builder/env.rs` - Environment setup, type definitions
2. `crates/jet/src/builder/ops.rs` - Low-level LLVM operations
3. `crates/jet/src/builder/contract.rs` - Contract building logic
4. `crates/jet/src/builder/manager.rs` - Build management
5. `crates/jet/src/engine/mod.rs` - Execution engine
6. `crates/jet/src/bin/jetdbg.rs` - CLI tool
7. `crates/jet_runtime/src/builtins.rs` - Runtime functions
8. `crates/jet_runtime/src/exec.rs` - Execution context

**Iterative Process:**
1. Fix errors in one file
2. Run `cargo build` again
3. Document the fix
4. Repeat until build succeeds

**Success Criteria:**
- `cargo build` completes without errors
- All API migrations documented
- Code compiles for both release and debug

---

### Phase 5: LLVM IR Verification
**Goal:** Ensure `runtime-ir/jet.ll` is compatible with LLVM 21

**Tasks:**

1. **Verify IR Syntax**
   ```bash
   llvm-as-21 runtime-ir/jet.ll -o /tmp/jet.bc
   llvm-dis-21 /tmp/jet.bc -o /tmp/jet-verified.ll
   ```

2. **Check for Deprecation Warnings**
   Look for warnings about:
   - Deprecated instruction syntax
   - Type system changes
   - Attribute changes

3. **Update Target Triple** (if needed)
   Current: `target triple = "x86_64-apple-macosx14.0.0"`

   Options:
   - Generic: `"x86_64-unknown-unknown"`
   - Linux: `"x86_64-unknown-linux-gnu"`
   - Keep as-is if cross-platform compatibility not needed

4. **Test IR Loading**
   - Verify module loads in the engine without errors
   - Check that all declared functions are recognized

**Success Criteria:**
- IR loads successfully in LLVM 21
- No syntax errors or warnings
- Module integrates with Rust code

---

### Phase 6: Runtime Verification
**Goal:** Verify the runtime library builds and links correctly

**Tasks:**

1. **Check for Build Script**
   ```bash
   find crates/jet_runtime -name "build.rs"
   ```

   If it exists:
   - Review what it does
   - Update for LLVM 21 if needed
   - Test standalone runtime build

2. **Test Dynamic Library Output**
   ```bash
   cargo build -p jet_runtime
   ls -la target/debug/*.so  # or *.dylib on macOS
   ```

3. **Verify Symbol Exports**
   ```bash
   nm -D target/debug/libjet_runtime.so | grep jet
   ```

   Expected symbols:
   - `jet.stack.push.ptr`
   - `jet.stack.pop`
   - `jet.mem.store`
   - etc.

**Success Criteria:**
- Runtime builds successfully
- Dynamic library generated
- Required symbols exported
- No link errors

---

### Phase 7: Test Suite Execution
**Goal:** Run all tests and document failures

**Tasks:**

1. **Run All Tests**
   ```bash
   cargo test 2>&1 | tee test-results-phase7.txt
   ```

2. **Categorize Test Failures**
   - Build failures (should be none at this point)
   - Runtime errors (crashes, panics)
   - Assertion failures (incorrect results)
   - Timeout failures

3. **Document Each Failure**
   Create `docs/test-failure-log.md`:
   - Test name
   - Failure type
   - Error message/backtrace
   - Suspected cause
   - Fix approach

4. **Prioritize Fixes**
   - Critical: Tests that crash/panic
   - High: Tests with incorrect results
   - Medium: Tests with minor discrepancies
   - Low: Flaky tests

**Success Criteria:**
- All tests executed
- Failures documented
- Fix priorities established

---

### Phase 8: Test Fixes
**Goal:** Fix all test failures

**Strategy:**

For each failing test:

1. **Understand the Test**
   - Read test code
   - Understand expected behavior
   - Check test data (ROMs)

2. **Reproduce Locally**
   ```bash
   cargo test --test test_roms -- --nocapture [test_name]
   ```

3. **Debug the Issue**
   - Add logging/tracing
   - Use debugger if needed
   - Check JIT code generation
   - Verify runtime function calls

4. **Fix and Verify**
   - Implement fix
   - Run specific test
   - Run all tests to prevent regressions

**Common Issues to Expect:**
- JIT execution engine setup differences
- Memory model changes
- ABI changes affecting function calls
- Stack/memory layout differences
- EVM opcode implementation issues

**Success Criteria:**
- All tests pass
- No regressions introduced
- Test execution stable (not flaky)

---

### Phase 9: Integration Testing
**Goal:** Test the `jetdbg` binary end-to-end

**Tasks:**

1. **Build Binary**
   ```bash
   cargo build --release --bin jetdbg
   ```

2. **Test Basic Functionality**
   ```bash
   ./target/release/jetdbg --help
   ```

3. **Run Sample Contracts**
   - Use test ROMs from `crates/jet/tests/roms/`
   - Verify execution
   - Check output correctness

4. **Performance Validation**
   - Compare execution times (if baseline available)
   - Check for performance regressions
   - Profile if needed

**Success Criteria:**
- Binary builds and runs
- Can execute EVM contracts
- Produces correct results
- Performance acceptable

---

### Phase 10: Documentation Updates
**Goal:** Update all documentation to reflect LLVM 21 changes

**Tasks:**

1. **Update README.md**
   - Change LLVM version from 18 to 21
   - Update installation instructions
   - Update environment variables:
     ```shell
     export LLVM_SYS_210_PREFIX=/usr/local/opt/llvm
     ```
   - Update Ubuntu install commands:
     ```shell
     sudo ./llvm.sh 21
     sudo apt-get install llvm-21-dev
     ```

2. **Update Makefile**
   ```makefile
   export LLVM_SYS_210_PREFIX=/usr/lib/llvm-21
   export RUST_BACKTRACE=1
   ```

3. **Create Migration Guide**
   - Document all API changes encountered
   - Provide before/after examples
   - List breaking changes
   - Add troubleshooting section

4. **Update Project Documentation**
   - Create `docs/api-migration-log.md` (if not already done)
   - Create `docs/llvm21-upgrade-notes.md`
   - Update `docs/jet-description.md` if needed

**Success Criteria:**
- All docs reflect LLVM 21
- Installation instructions work
- Migration guide complete
- Future contributors can follow updates

---

### Phase 11: Final Validation
**Goal:** Comprehensive validation of the restored system

**Tasks:**

1. **Clean Build Test**
   ```bash
   cargo clean
   rm -rf target/
   make build
   ```

2. **Run Full Test Suite**
   ```bash
   make test
   ```

3. **Run Linter**
   ```bash
   make clippy
   ```

4. **Run Full Pre-Commit Check**
   ```bash
   make commit-check
   ```

5. **Test on Fresh Environment**
   - Spin up clean container/VM
   - Follow installation docs
   - Build and test
   - Verify no hidden dependencies

**Success Criteria:**
- Clean builds work
- All tests pass
- No clippy warnings
- Commit-check passes
- Fresh environment works

---

## Rollback Strategy

If the LLVM 21 upgrade proves too difficult:

### Fallback Option 1: Stick with LLVM 18
1. Install `llvm-18-dev` package
2. Use Inkwell 0.4.0 stable release with `llvm18-0` feature
3. Fix only the build issues, not the upgrade
4. Plan upgrade for later

### Fallback Option 2: Try LLVM 19 or 20
1. LLVM 19 or 20 may have fewer breaking changes
2. Adjust feature flags accordingly
3. Test incremental upgrade path

---

## Risk Assessment

### High Risk
- **LLVM API breaking changes:** May require extensive code changes
- **Test failures:** May indicate fundamental incompatibilities
- **Performance regressions:** LLVM 21 may have different optimization characteristics

### Medium Risk
- **Build time increase:** LLVM 21 may be slower to compile
- **Dependency conflicts:** Other crates may not support LLVM 21 yet
- **Platform-specific issues:** LLVM behavior may differ on different systems

### Low Risk
- **Documentation gaps:** Can be filled as we go
- **Minor dependency updates:** Usually backwards compatible
- **Cargo warnings:** Easy to fix

---

## Success Metrics

### Build Success
- [ ] `cargo build` completes without errors
- [ ] `cargo build --release` completes without errors
- [ ] No compiler warnings in project code
- [ ] Clean builds work (from scratch)

### Test Success
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Test execution is stable (not flaky)
- [ ] Coverage maintained or improved

### Code Quality
- [ ] No clippy warnings
- [ ] Code follows Rust best practices
- [ ] No unsafe code added (or justified)
- [ ] Error handling improved where needed

### Documentation
- [ ] README updated
- [ ] All installation steps verified
- [ ] Migration guide created
- [ ] Code comments updated where APIs changed

### Functionality
- [ ] `jetdbg` binary works
- [ ] Can compile and run EVM contracts
- [ ] Results match expected behavior
- [ ] No performance regressions

---

## Timeline Estimate

This is a rough estimate - actual time may vary significantly based on the number of API changes encountered:

- **Phase 1 (Environment Setup):** 30-60 minutes
- **Phase 2 (Dependency Updates):** 30 minutes
- **Phase 3 (Initial Build):** 15 minutes
- **Phase 4 (API Migration):** 2-8 hours (highly variable)
- **Phase 5 (LLVM IR Verification):** 1-2 hours
- **Phase 6 (Runtime Verification):** 1-2 hours
- **Phase 7 (Test Execution):** 30 minutes
- **Phase 8 (Test Fixes):** 2-6 hours (highly variable)
- **Phase 9 (Integration Testing):** 1-2 hours
- **Phase 10 (Documentation):** 1-2 hours
- **Phase 11 (Final Validation):** 1 hour

**Total Estimated Time:** 10-25 hours of work

---

## Next Steps

1. Review this plan with stakeholders
2. Set up a tracking mechanism (GitHub Issues, project board, etc.)
3. Begin Phase 1: Environment Setup
4. Proceed sequentially through phases
5. Document issues and solutions as they arise
6. Update this plan as needed based on discoveries

---

## Resources

### Documentation
- [Inkwell Documentation](https://thedan64.github.io/inkwell/)
- [Inkwell GitHub Repository](https://github.com/TheDan64/inkwell)
- [LLVM 21 Release Notes](https://releases.llvm.org/21.0.0/docs/ReleaseNotes.html)
- [llvm-sys Crate](https://crates.io/crates/llvm-sys)

### Community
- Inkwell GitHub Issues
- Rust LLVM Discord/forums
- LLVM mailing lists

### Tools
- `llvm-config-21` - Query LLVM configuration
- `llvm-as-21` - LLVM assembler
- `llvm-dis-21` - LLVM disassembler
- `cargo tree` - Inspect dependency tree
- `cargo clippy` - Rust linter

---

## Appendix A: Build Error Reference

### Error: Missing LLVM Headers
```
fatal error: llvm-c/Target.h: No such file or directory
```
**Solution:** Install `llvm-21-dev` package

### Error: Wrong LLVM Version
```
LLVM version mismatch: expected 21.x, found 18.x
```
**Solution:** Set `LLVM_SYS_210_PREFIX` environment variable

### Error: Inkwell API Not Found
```
error[E0599]: no method named `build_foo` found for struct `Builder`
```
**Solution:** Check Inkwell 0.8.0 API docs for renamed/moved methods

---

## Appendix B: Dependency Version Matrix

| Crate | LLVM 18 | LLVM 21 | Notes |
|-------|---------|---------|-------|
| inkwell | 0.4.0 (git) | 0.8.0 | Use `llvm21-1` feature |
| llvm-sys | 180.0.0 | 211.0.0 | Implicit via inkwell |

---

## Appendix C: File Change Checklist

- [ ] `/Cargo.toml` - Remove cargo-features
- [ ] `/crates/jet/Cargo.toml` - Update inkwell, remove cargo-features
- [ ] `/crates/jet_runtime/Cargo.toml` - Update inkwell, remove cargo-features
- [ ] `/Makefile` - Update LLVM_SYS env var
- [ ] `/README.md` - Update LLVM version references
- [ ] `/runtime-ir/jet.ll` - Verify/update if needed
- [ ] Source files as needed based on API changes

---

## Document History

- 2026-01-27: Initial draft created after repository assessment
