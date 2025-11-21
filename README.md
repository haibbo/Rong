# Rong (融) - Multi-Engine JavaScript Runtime for Rust

[![Rust](https://img.shields.io/badge/rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/your-org/rong)

**Rong** (融, meaning "fusion" in Chinese) is a specialized JavaScript runtime for Rust designed for **LingXia App** and **microservices** in constrained environments. It provides a lightweight, secure JavaScript execution environment optimized for specific use cases rather than general-purpose Node.js replacement.

## 🌟 Why "Rong" (融)?

The name "Rong" (融) embodies the core philosophy of this project:

- **🔗 Fusion**: Seamlessly merges JavaScript engines with Rust native code
- **🌐 Harmony**: Creates harmonious integration between different runtime environments
- **⚡ Unity**: Unifies multiple JavaScript engines under a single, elegant API
- **🚀 Flow**: Enables smooth data flow between JavaScript and Rust worlds

In Chinese culture, "融" represents natural harmony and coexistence - perfectly capturing how Rong brings together diverse technologies into a unified whole.

## ✨ Features

### 🎯 Multi-Engine Support
- **QuickJS** - Lightweight and fast
- **JavaScriptCore** - WebKit's proven engine
- **ArkJS** - HarmonyOS JavaScript engine 🚧 **In Development**

### 🛠️ Developer Experience
- **Zero-cost abstractions** - Minimal runtime overhead
- **Type-safe bindings** - Rust's type system ensures memory safety
- **Async/await support** - Full Promise and async iterator integration
- **Rich module ecosystem** - Built-in modules for common tasks

### 🏗️ Architecture
- **Unified API** - Same code works across all engines
- **Memory efficient** - Careful resource management
- **Thread-safe** - Safe concurrent access patterns
- **Extensible** - Easy to add custom modules and bindings

```
┌─────────────────────────────────────────────────────────────┐
│                        Rong Core                            │
├─────────────────────────────────────────────────────────────┤
│ Unified API │ Type System │ Memory Management │ Async/Await │
├─────────────────────────────────────────────────────────────┤
│    QuickJS   │      JavaScriptCore     │      ArkJS         │
├─────────────────────────────────────────────────────────────┤
│              Built-in Modules & Extensions                  │
│   Timer │ HTTP │ FS │ Console │  Path │ URL │ ...           │
└─────────────────────────────────────────────────────────────┘
```


## 🚀 Quick Start

### Basic Usage

```rust
use rong::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a JavaScript runtime and context
    let rt = RongJS::runtime();
    let ctx = rt.context();

    // Execute JavaScript code
    let result: i32 = ctx.eval(Source::from_bytes(b"2 + 3"))?;
    println!("Result: {}", result); // Output: Result: 5

    Ok(())
}
```

### Binding Rust Structs

```rust
use rong::*;

#[js_export]
struct Point {
    x: f64,
    y: f64,
}

#[js_class]
impl Point {
    #[js_method(constructor)]
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[js_method]
    fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a JavaScript runtime and context
    let rt = RongJS::runtime();
    let ctx = rt.context();

    // Register the Point class
    ctx.register_class::<Point>()?;

    // Use it in JavaScript
    let distance: f64 = ctx.eval(Source::from_bytes(r#"
        let p1 = new Point(0, 0);
        let p2 = new Point(3, 4);
        p1.distance(p2);
    "#.as_bytes()))?;

    println!("Distance: {}", distance); // Output: Distance: 5

    Ok(())
}
```

## 📦 Built-in Modules

Rong comes with a rich set of built-in modules:

- **🕐 rong_timer** - setTimeout, setInterval, async timers
- **🌐 rong_http** - HTTP client/server, fetch API
- **📁 rong_fs** - File system operations
- **📝 rong_console** - Console logging and debugging
- **🔗 rong_url** - URL parsing and manipulation
- **📊 rong_buffer** - Binary data handling
- **⚡ rong_event** - Event emitter and handling
- **🛡️ rong_abort** - AbortController and signals

## 🔄 Type Conversion

Rong provides seamless type conversion between Rust and JavaScript types:

### Basic Types

| Rust Type | JavaScript Type | Example |
|-----------|-----------------|---------|
| `bool` | `boolean` | `true` ↔ `true` |
| `i32`, `i64` | `number` | `42` ↔ `42` |
| `f32`, `f64` | `number` | `3.14` ↔ `3.14` |
| `String` | `string` | `"hello"` ↔ `"hello"` |
| `()` | `undefined` | `()` ↔ `undefined` |

### Collections

```rust
use rong::*;

// Create runtime and context
let rt = RongJS::runtime();
let ctx = rt.context();

// Arrays
let rust_vec = vec![1, 2, 3];
let global = ctx.global();
global.set("numbers", rust_vec)?;
let js_result: Vec<i32> = ctx.eval(Source::from_bytes(b"numbers.map(x => x * 2)"))?;
// js_result = [2, 4, 6]

// Objects/Maps
use std::collections::HashMap;
let mut map = HashMap::new();
map.insert("name".to_string(), "Rong".to_string());
map.insert("version".to_string(), "0.1.0".to_string());
global.set("config", map)?;

let name: String = ctx.eval(Source::from_bytes(b"config.name"))?;
// name = "Rong"
```

### Custom Structs

```rust
use rong::*;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    active: bool,
}

let rt = RongJS::runtime();
let ctx = rt.context();

// Rust to JavaScript
let user = User { id: 1, name: "Alice".to_string(), active: true };
let global = ctx.global();
global.set("user", user)?;

// JavaScript to Rust
let updated_user: User = ctx.eval(Source::from_bytes(r#"
    ({
        id: user.id,
        name: user.name.toUpperCase(),
        active: !user.active
    })
"#.as_bytes()))?;
```

### Error Handling

```rust
use rong::*;

let rt = RongJS::runtime();
let ctx = rt.context();

// Handle JavaScript exceptions
match ctx.eval::<i32>(Source::from_bytes(b"invalid.syntax")) {
    Ok(result) => println!("Result: {}", result),
    Err(e) => println!("JavaScript Error: {}", e),
}

// Type conversion errors
match ctx.eval::<String>(Source::from_bytes(b"42")) {
    Ok(result) => println!("String: {}", result), // "42"
    Err(e) => println!("Conversion Error: {}", e),
}
```



## 🧪 Testing

⚠️ **Important**: Engine features are **required** for testing. Rong supports multiple JavaScript engines, and you must specify which engine to use.

### Available Engine Features

- `--features quickjs` - QuickJS engine
- `--features jscore` - JavaScriptCore engine
- `--features arkjs` - ArkJS engine

### Running Tests

```bash
# ✅ Test with QuickJS engine
cargo test --features quickjs

# ✅ Test with JavaScriptCore engine
cargo test --features jscore

# ✅ Test specific module with engine
cargo test -p rong_http --features quickjs
cargo test -p rong_timer --features jscore

# ✅ Test specific test case
cargo test --test iterator --features quickjs
cargo test --test promise --features jscore

# ✅ Run all core tests on QuickJS
cargo test --features quickjs --lib

# ✅ Test with verbose output
cargo test --features quickjs -- --nocapture
```



## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- QuickJS team for the lightweight JavaScript engine
- WebKit team for JavaScriptCore
- HarmonyOS team for ArkJS
- Rust community for excellent async ecosystem

---

**Rong (融)** - *Fusing JavaScript engines with Rust, creating harmony in diversity.*
