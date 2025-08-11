# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

libperl-rs is a Rust library for embedding the Perl 5 runtime into Rust applications. It provides safe FFI bindings to libperl through a three-crate workspace structure:
- `libperl-rs`: High-level safe Rust API
- `libperl-sys`: Low-level FFI bindings (auto-generated via bindgen)
- `libperl-config`: Perl configuration extraction utilities

## Essential Commands

### Build Commands
```bash
# Standard build
cargo build

# Build with verbose output (useful for debugging build issues)
cargo build --verbose

# Build all workspace members
cargo build --all
```

### Testing Commands
```bash
# Run all tests including examples (primary test method)
cargo test --all --examples

# Run specific example
cargo run --example 000_perl_parse

# Run Docker-based tests against multiple Perl versions
./runtest-docker.zsh

# Test with specific Perl version in Docker
docker run --rm -v $(pwd):/work -w /work perl:5.40 bash -c "apt-get update && apt-get install -y llvm libclang-dev clang && cargo test --all --examples"
```

### Linting and Checks
```bash
# Format code
cargo fmt

# Run clippy
cargo clippy --all-targets

# Check compilation without building
cargo check --all
```

## Architecture and Code Organization

### Workspace Structure
The project uses a Cargo workspace with three interdependent crates:

1. **libperl-sys** (build dependency): 
   - Uses bindgen in `build.rs` to generate FFI bindings from `wrapper.h`
   - Provides raw unsafe bindings to Perl C API
   - Auto-detects Perl features (threading, API version) at build time

2. **libperl-config** (build dependency):
   - Parses Perl's Config.pm to extract build configuration
   - Provides utilities for discovering Perl installation details
   - Used by build scripts to configure compilation

3. **libperl-rs** (main crate):
   - Safe Rust wrapper around libperl-sys
   - Main entry point in `src/lib.rs` and `src/perl.rs`
   - Provides `Perl` struct for interpreter lifecycle management

### Key Architectural Patterns

**Thread Safety Handling**: The codebase uses conditional compilation to handle threaded vs non-threaded Perl:
```rust
// Thread-aware API macros in src/perl.rs
cfg_if! {
    if #[cfg(perl_useithreads)] {
        // Threaded Perl requires passing interpreter context
        macro_rules! perl_api { ... }
    } else {
        // Non-threaded Perl uses global interpreter
        macro_rules! perl_api { ... }
    }
}
```

**Version Compatibility**: Features are conditionally compiled based on Perl version:
- `cfg(perlapi_ver26)` through `cfg(perlapi_ver40)` for version-specific features
- Detected automatically during build via libperl-config

**Build Process Flow**:
1. `libperl-config` extracts Perl configuration from system
2. `libperl-sys/build.rs` uses this config to generate bindings via bindgen
3. Main crate compiles with appropriate feature flags

### Critical Files for Understanding the Codebase

- `src/perl.rs`: Core Perl interpreter wrapper and API implementation
- `libperl-sys/build.rs`: Bindgen configuration and feature detection logic
- `libperl-config/src/perl_config.rs`: Perl configuration parsing
- `examples/`: Comprehensive usage examples demonstrating API patterns

## Development Guidelines

### Adding New Perl API Bindings
1. Add the C function declaration to `libperl-sys/wrapper.h`
2. Implement safe wrapper in `src/perl.rs` using the `perl_api!` or `unsafe_perl_api!` macros
3. Add an example in `examples/` demonstrating usage
4. Ensure compatibility with both threaded and non-threaded Perl

### Testing Against Multiple Perl Versions
The project supports Perl 5.10+ with special handling for versions 5.26, 5.28, 5.30, 5.32, 5.34, 5.36, 5.38, and 5.40. CI tests all these versions in both threaded and non-threaded configurations.

### Memory Safety Considerations
- All Perl API calls must go through the safety macros in `src/perl.rs`
- The `Perl` struct manages interpreter lifecycle with RAII
- Use `typed-arena` for managing Perl-allocated memory when needed

## System Requirements

### Build Dependencies
- Perl 5 development headers (`perl-dev` or equivalent)
- LLVM and Clang (for bindgen)
- Standard Rust toolchain

### Platform Notes
- Primary development on Linux x86_64
- Docker recommended for testing multiple Perl versions
- SELinux users: Check volume mounting permissions in Docker

## Common Issues and Solutions

### Bindgen Failures
If bindgen fails to generate bindings:
1. Ensure LLVM/Clang are installed: `apt-get install llvm libclang-dev clang`
2. Check that Perl development headers are available
3. Verify `perl -MConfig -e 'print $Config{archlib}'` returns a valid path

### Thread Safety Errors
The codebase automatically detects whether Perl was built with threads. If you encounter thread-related issues:
1. Check your Perl's thread configuration: `perl -V:useithreads`
2. Ensure consistent thread configuration across all dependencies

### Feature Detection
The build script automatically detects Perl features. To debug:
1. Run `cargo build --verbose` to see detected features
2. Check generated `target/debug/build/*/output` for feature flags