<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br><b>clock-lib</b><br>
    <sub><sup>API REFERENCE</sup></sub>
</h1>
<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <a href="./GUIDELINES.md" title="Developer Guidelines"><b>GUIDELINES</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <span>API</span>
    </sup>
</div>
<br>

> **Status:** This API reference tracks the public surface of `clock-lib`. It is updated at the end of every milestone, before any version tag or release. All code examples are verified against the current codebase.

## Table of Contents

- **[Installation](#installation)**
- **[Quick Start](#quick-start)**
- **[Public API](#public-api)**
  - [Tier-1 Functions](#tier-1-functions)
    - [`now()`](#now)
    - [`elapsed()`](#elapsed)
    - [`wall()`](#wall)
    - [`unix()`](#unix)
    - [`unix_ms()`](#unix_ms)
    - [`unix_ns()`](#unix_ns)
  - [The `Clock` Trait](#clock-trait)
    - [`SystemClock`](#systemclock)
    - [`ManualClock`](#manualclock)
  - [Types](#types)

<br>

## Installation

```toml
[dependencies]
clock-lib = "0.1"
```

<br>

## Quick Start

```rust
use clock_lib as clock;

let start = clock::now();
// ... work ...
let took = clock::elapsed(start);
```

<br>

## Public API

> **NOTE:** The implementation is in active development. This section is populated and kept current as each public item lands. Every struct, trait, function, and type that is part of the public API will be documented here with description, parameters, and multiple code examples.

### Tier-1 Functions

#### `now()`
_Documented when implemented._

#### `elapsed()`
_Documented when implemented._

#### `wall()`
_Documented when implemented._

#### `unix()`
_Documented when implemented._

#### `unix_ms()`
_Documented when implemented._

#### `unix_ns()`
_Documented when implemented._

### `Clock` Trait

_Documented when implemented._

#### `SystemClock`
_Documented when implemented._

#### `ManualClock`
_Documented when implemented._

### Types

_Documented when implemented._