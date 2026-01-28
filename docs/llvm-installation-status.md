# LLVM Installation Status and Findings

**Date:** 2026-01-28
**Agent:** Claude (Session 01Sd7TY8FzjwTv3DmSuayBtB)
**Status:** Blocked by network connectivity issues

---

## Current Blocker

**Issue:** Temporary DNS resolution failures preventing package downloads from Ubuntu repositories

**Error:**
```
Temporary failure resolving 'archive.ubuntu.com'
Temporary failure resolving 'security.ubuntu.com'
```

This is preventing installation of LLVM packages via `apt-get`.

---

## Key Findings

### 1. LLVM 21 Not Available in Ubuntu Repositories

**Discovery:** LLVM 21 is not yet released in Ubuntu 24.04 (Noble) repositories.

- The llvm.sh script from apt.llvm.org lists LLVM 20 as the current stable version
- The script explicitly does not support LLVM 21 yet
- Ubuntu repositories contain LLVM versions 14-20, but not 21

**Available LLVM Versions in Ubuntu 24.04:**
```
llvm-14, llvm-14-dev
llvm-15, llvm-15-dev
llvm-16, llvm-16-dev
llvm-17, llvm-17-dev
llvm-18, llvm-18-dev
llvm-19, llvm-19-dev
llvm-20, llvm-20-dev  ← Highest available
```

### 2. Recommended Approach: Use LLVM 20

**Rationale:**
- LLVM 20 is available in Ubuntu repositories
- Inkwell 0.8.0 supports LLVM 11-21 (including 20)
- LLVM 20 feature flag for Inkwell: `llvm20-0`
- llvm-sys version for LLVM 20: `200.0.0`

**Required Changes from Original Plan:**
```toml
# Original (LLVM 21):
inkwell = { version = "0.8.0", features = ["llvm21-1"] }
export LLVM_SYS_211_PREFIX=/usr/lib/llvm-21

# Revised (LLVM 20):
inkwell = { version = "0.8.0", features = ["llvm20-0"] }
export LLVM_SYS_200_PREFIX=/usr/lib/llvm-20
```

---

## Installation Steps for Next Agent

Once network connectivity is restored, follow these steps:

### 1. Install LLVM 20

```bash
sudo apt-get update
sudo apt-get install -y llvm-20 llvm-20-dev
```

### 2. Set Environment Variable

```bash
export LLVM_SYS_200_PREFIX=/usr/lib/llvm-20
```

Add to shell profile for persistence:
```bash
echo 'export LLVM_SYS_200_PREFIX=/usr/lib/llvm-20' >> ~/.bashrc
```

### 3. Verify Installation

```bash
# Check LLVM version
llvm-config-20 --version  # Should output 20.x.x

# Verify headers exist
ls /usr/lib/llvm-20/include/llvm-c/  # Should list header files

# Verify environment variable
echo $LLVM_SYS_200_PREFIX  # Should output /usr/lib/llvm-20
```

### 4. Update Cargo.toml Files

Update both crate configurations:

**File: `/home/user/jet/Cargo.toml`**
```toml
# Remove this line:
cargo-features = ["edition2024"]

# Rest of file stays the same
```

**File: `/home/user/jet/crates/jet/Cargo.toml`**
```toml
# Remove this line:
cargo-features = ["edition2024"]

[dependencies]
# Change inkwell dependency:
inkwell = { version = "0.8.0", features = ["llvm20-0"] }

# Remove explicit llvm-sys dependency if present
# (inkwell will handle it automatically)
```

**File: `/home/user/jet/crates/jet_runtime/Cargo.toml`**
```toml
# Remove this line:
cargo-features = ["edition2024"]

[dependencies]
# Change inkwell dependency:
inkwell = { version = "0.8.0", features = ["llvm20-0"] }
```

### 5. Clean and Build

```bash
cd /home/user/jet
cargo clean
rm -f Cargo.lock
cargo build 2>&1 | tee build-output-llvm20.txt
```

---

## Alternative: Build LLVM 21 from Source

If LLVM 21 is absolutely required (rather than LLVM 20), it can be built from source:

```bash
# Download LLVM 21 source
wget https://github.com/llvm/llvm-project/releases/download/llvmorg-21.1.0/llvm-21.1.0.src.tar.xz
tar xf llvm-21.1.0.src.tar.xz
cd llvm-21.1.0.src

# Build and install
mkdir build && cd build
cmake -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_INSTALL_PREFIX=/usr/local/llvm-21 \
      -DLLVM_ENABLE_PROJECTS="clang;lld" \
      ..
make -j$(nproc)
sudo make install

# Set environment
export LLVM_SYS_211_PREFIX=/usr/local/llvm-21
```

**Warning:** Building from source takes significant time (1-3 hours) and disk space (~30GB during build).

---

## What Was Attempted

1. ✅ Downloaded llvm.sh script from apt.llvm.org
2. ❌ Attempted to run `./llvm.sh 21` - failed because LLVM 21 not supported yet
3. ✅ Identified that LLVM 20 is the highest available version
4. ❌ Attempted to install LLVM 20 via apt-get - failed due to network issues

---

## Network Diagnostics

Current network errors suggest DNS resolution problems. Recommended diagnostics:

```bash
# Check DNS resolution
ping -c 3 archive.ubuntu.com
ping -c 3 8.8.8.8

# Check network connectivity
curl -I https://google.com

# Try alternative DNS
echo "nameserver 8.8.8.8" | sudo tee /etc/resolv.conf.new

# Wait and retry
sleep 60
sudo apt-get update
```

---

## Recommendation

**Primary Path:** Use LLVM 20 instead of LLVM 21
- It's available in repos (once network is working)
- Fully supported by Inkwell 0.8.0
- Same API as LLVM 21 for most purposes
- Easier and faster to install

**Only use LLVM 21 if:**
- Specific LLVM 21 features are absolutely required
- Willing to build from source (significant time investment)

---

## Files Modified

None yet - network issues prevented any actual changes.

---

## Files to Modify Next

1. `/home/user/jet/Cargo.toml` - Remove cargo-features
2. `/home/user/jet/crates/jet/Cargo.toml` - Update inkwell to llvm20-0, remove cargo-features
3. `/home/user/jet/crates/jet_runtime/Cargo.toml` - Update inkwell to llvm20-0, remove cargo-features
4. `/home/user/jet/docs/reconstruction-plan.md` - Update with LLVM 20 information
5. Environment configuration files (Makefile, README) - Use LLVM_SYS_200_PREFIX

---

## References

- [Inkwell GitHub Repository](https://github.com/TheDan64/inkwell) - Supports LLVM 11-21
- [llvm-sys on crates.io](https://crates.io/crates/llvm-sys) - Latest is 211.0.0, LLVM 20 is 200.0.0
- LLVM 20 documentation: https://releases.llvm.org/20.0.0/docs/ReleaseNotes.html

---

## Next Agent Checklist

- [ ] Verify network connectivity is restored
- [ ] Install LLVM 20 and llvm-20-dev packages
- [ ] Set LLVM_SYS_200_PREFIX environment variable
- [ ] Verify LLVM installation with llvm-config-20
- [ ] Update all Cargo.toml files (remove cargo-features, update inkwell)
- [ ] Clean build artifacts (cargo clean, rm Cargo.lock)
- [ ] Attempt build with LLVM 20
- [ ] Document any compiler errors encountered
- [ ] Proceed with API migration fixes as needed
- [ ] Update reconstruction plan with final LLVM version used
