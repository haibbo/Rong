# Rust-Driven JavaScript APIs and Classes

This guide explains how to expose Rust functions and classes to JavaScript in the Rong engine.

## Table of Contents

- [Quick Overview](#quick-overview)
- [Part 1: JavaScript Functions](#part-1-javascript-functions)
  - [Async Functions](#async-functions)
  - [Sync Functions](#sync-functions)
  - [Function Registration](#function-registration)
  - [Optional Parameters](#optional-parameters)
- [Part 2: JavaScript Classes](#part-2-javascript-classes)
  - [Class Definition](#class-definition)
  - [Constructor](#constructor)
  - [Instance Methods](#instance-methods)
  - [Getters and Setters](#getters-and-setters)
  - [Static Methods](#static-methods)
  - [Mutable Methods](#mutable-methods)
- [Advanced Topics](#advanced-topics)
  - [Complex Input Types with FromJSObj](#complex-input-types-with-fromjsobj)
  - [Attribute Reference](#attribute-reference)
- [Complete Examples](#complete-examples)

---

## Quick Overview

Rong provides two main approaches for exposing Rust to JavaScript:

| Approach        | Use Case                              | Example                    |
| :---            | :---                                  | :---                       |
| **Functions**   | Standalone utilities, module APIs     | `Rong.readFile()`          |
| **Classes**     | Stateful objects with methods         | `new Point2D(10, 20)`      |

**Key macros:**

- `#[js_export]` — Mark a struct to be exposed to JavaScript
- `#[js_class]` — Mark an `impl` block to define JS class methods
- `#[js_method]` — Mark individual methods to expose to JavaScript

---

## Part 1: JavaScript Functions

### Async Functions

Most Rust functions that perform I/O should be async. Rong automatically converts them to JavaScript Promises.

**Example:** File system operations

```rust
use rong::*;

/// Rename a file or directory
async fn rename(from: String, to: String) -> JSResult<()> {
    tokio::fs::rename(&from, &to)
        .await
        .map_err(|e| HostError::new("FS_IO", format!("Failed to rename: {}", e)).into())
}

/// Read a file's contents
async fn read_file(path: String) -> JSResult<String> {
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| HostError::new("FS_IO", format!("Failed to read: {}", e)).into())
}
```

**JavaScript usage:**

```javascript
// These return Promises automatically
await Rong.rename("old.txt", "new.txt");
const content = await Rong.readFile("data.txt");
```

### Sync Functions

Synchronous functions are also supported, but use them only for non-blocking operations.

```rust
/// Get the current working directory
fn cwd() -> JSResult<String> {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| HostError::new("FS_IO", format!("Failed to get cwd: {}", e)).into())
}

/// Check if a path is absolute
fn is_absolute(path: String) -> bool {
    std::path::Path::new(&path).is_absolute()
}
```

**JavaScript usage:**

```javascript
// Sync functions return values directly
const dir = Rong.cwd();
const absolute = Rong.isAbsolute("/usr/bin");
```

### Function Registration

Register functions using `JSFunc::new()` and attach them to global objects or modules.

```rust
pub fn init(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.rong();

    // Register async function
    let rename_fn = JSFunc::new(ctx, rename)?.name("rename")?;
    rong.set("rename", rename_fn)?;

    // Register sync function
    let cwd_fn = JSFunc::new(ctx, cwd)?.name("cwd")?;
    rong.set("cwd", cwd_fn)?;

    Ok(())
}
```

**Registration pattern breakdown:**

1. `JSFunc::new(ctx, function)` — Wrap the Rust function
2. `.name("functionName")` — Set the function name (for stack traces)
3. `rong.set("key", func)` — Attach to the global `Rong` object

### Optional Parameters

Use `Optional<T>` from `rong::function` for optional parameters.

```rust
use rong::function::Optional;

async fn read_file_with_encoding(
    path: String,
    encoding: Optional<String>
) -> JSResult<String> {
    let enc = encoding.0.unwrap_or_else(|| "utf-8".to_string());
    // Use encoding...
    todo!()
}
```

**JavaScript usage:**

```javascript
// Both calls work
await Rong.readFile("file.txt");
await Rong.readFile("file.txt", "utf-16");
```

---

## Part 2: JavaScript Classes

### Class Definition

Use `#[js_export]` on the struct and `#[js_class]` on the impl block.

**Basic example:**

```rust
use rong::*;
use rong::{js_export, js_class, js_method};

#[js_export]
#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[js_class(rename = "Point2D")]
impl Point {
    #[js_method(constructor)]
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
```

**JavaScript usage:**

```javascript
const p = new Point2D(10, 20);
```

**Notes:**
- `#[js_export]` makes the struct available to the macro system
- `rename = "Point2D"` sets the JavaScript class name (optional, defaults to struct name)
- `#[derive(Debug)]` is optional but helpful for debugging

### Constructor

Mark the constructor with `#[js_method(constructor)]`.

```rust
#[js_class]
impl Point {
    #[js_method(constructor)]
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
```

**Rules:**
- Must return `Self`
- Can have any number of parameters
- Can return `JSResult<Self>` for fallible construction

**Fallible constructor:**

```rust
#[js_method(constructor)]
fn new(x: i32, y: i32) -> JSResult<Self> {
    if x < 0 || y < 0 {
        return Err(HostError::new(
            "INVALID_ARG",
            "Coordinates must be non-negative"
        ).into());
    }
    Ok(Self { x, y })
}
```

### Instance Methods

Regular instance methods take `&self` or `&mut self`.

```rust
#[js_class]
impl Point {
    /// Calculate distance from origin
    #[js_method]
    fn distance(&self) -> f64 {
        ((self.x.pow(2) + self.y.pow(2)) as f64).sqrt()
    }

    /// Add another point
    #[js_method(rename = "add")]
    fn add(&self, other: Point) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}
```

**JavaScript usage:**

```javascript
const p1 = new Point2D(3, 4);
console.log(p1.distance()); // 5

const p2 = new Point2D(10, 20);
const p3 = p1.add(p2); // Point2D(13, 24)
```

### Getters and Setters

Use `#[js_method(getter)]` and `#[js_method(setter)]` for property access.

```rust
#[js_class]
impl Point {
    // Getter
    #[js_method(getter, enumerable)]
    fn x(&self) -> i32 {
        self.x
    }

    // Setter
    #[js_method(setter, rename = "x")]
    fn set_x(&mut self, x: i32) {
        self.x = x;
    }

    #[js_method(getter, enumerable)]
    fn y(&self) -> i32 {
        self.y
    }

    #[js_method(setter, rename = "y")]
    fn set_y(&mut self, y: i32) {
        self.y = y;
    }
}
```

**JavaScript usage:**

```javascript
const p = new Point2D(10, 20);

// Use like properties
console.log(p.x); // 10
p.x = 15;
p.y = 25;

// enumerable makes them show up in Object.keys()
console.log(Object.keys(p)); // ['x', 'y']
```

**Notes:**
- `enumerable` makes the property show up in `Object.keys()` and `for...in` loops
- Setter must use `rename` to match the getter's name
- Setter requires `&mut self`

### Static Methods

Methods without `self` become static methods.

```rust
#[js_class]
impl Point {
    /// Create a point at the origin
    #[js_method]
    fn origin() -> Self {
        Self { x: 0, y: 0 }
    }

    /// Create a point from polar coordinates
    #[js_method(rename = "fromPolar")]
    fn from_polar(r: f64, theta: f64) -> Self {
        Self {
            x: (r * theta.cos()) as i32,
            y: (r * theta.sin()) as i32,
        }
    }
}
```

**JavaScript usage:**

```javascript
const origin = Point2D.origin();
const p = Point2D.fromPolar(10, Math.PI / 4);
```

### Mutable Methods

Methods that modify the instance require `&mut self`.

```rust
#[js_class]
impl Point {
    #[js_method(rename = "moveBy")]
    fn move_by(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }

    #[js_method]
    fn scale(&mut self, factor: i32) {
        self.x *= factor;
        self.y *= factor;
    }
}
```

**JavaScript usage:**

```javascript
const p = new Point2D(10, 20);
p.moveBy(5, 5);   // p is now (15, 25)
p.scale(2);       // p is now (30, 50)
```

---

## Advanced Topics

### Complex Input Types with FromJSObj

For methods that accept JavaScript objects, use `#[derive(FromJSObj)]`.

**Example:** Storage options (input)

```rust
use rong::FromJSObj;

#[derive(FromJSObj, Default)]
pub struct StorageOptions {
    #[rename = "maxKeySize"]
    max_key_size: Option<u32>,

    #[rename = "maxValueSize"]
    max_value_size: Option<u32>,

    #[rename = "maxDataSize"]
    max_data_size: Option<u32>,
}

#[js_export]
pub struct Storage {
    // ...
}

#[js_class]
impl Storage {
    #[js_method(constructor)]
    fn new(path: String, options: Optional<StorageOptions>) -> JSResult<Self> {
        let opts = options.0.unwrap_or_default();
        // Use opts.max_key_size, etc.
        todo!()
    }
}
```

**JavaScript usage:**

```javascript
// Pass an object with camelCase properties
const storage = new Storage("./data.db", {
    maxKeySize: 1024,
    maxValueSize: 65536
});
```

**FromJSObj features:**
- Automatically converts JavaScript objects to Rust structs
- `#[rename = "jsName"]` maps JS property names to Rust field names
- Works with `Option<T>` for optional fields
- Supports nested objects and arrays

### Returning Complex Objects with IntoJSObj

For methods that return JavaScript objects, use `#[derive(IntoJSObj)]`.

**Example:** Storage info (output)

```rust
use rong::IntoJSObj;

#[derive(IntoJSObj)]
pub struct StorageInfo {
    #[rename = "currentSize"]
    current_size: u32,

    #[rename = "limitSize"]
    limit_size: u32,

    #[rename = "keyCount"]
    key_count: u32,
}

#[js_class]
impl Storage {
    #[js_method]
    fn info(&self) -> JSResult<StorageInfo> {
        Ok(StorageInfo {
            current_size: 1024,
            limit_size: 10240,
            key_count: 42,
        })
    }
}
```

**JavaScript usage:**

```javascript
const info = storage.info();
console.log(info.currentSize);  // 1024
console.log(info.keyCount);     // 42

// The object is a plain JavaScript object
console.log(Object.keys(info)); // ['currentSize', 'limitSize', 'keyCount']
```

**IntoJSObj features:**
- Automatically converts Rust structs to JavaScript objects
- `#[rename = "jsName"]` maps Rust field names to JS property names
- `Option<T>` fields are omitted if `None`
- Supports nested structs and common types (`String`, `i32`, `f64`, `bool`, etc.)

### Attribute Reference

#### `#[js_export]`

Must be applied to structs you want to expose to JavaScript.

```rust
#[js_export]
struct MyClass { /* ... */ }
```

#### `#[js_class]`

Applied to `impl` blocks to expose methods to JavaScript.

**Attributes:**
- `rename = "JsName"` — Set the JavaScript class name

```rust
#[js_class(rename = "MyJSClass")]
impl MyClass { /* ... */ }
```

#### `#[js_method]`

Applied to individual methods within a `#[js_class]` impl block.

**Attributes:**

| Attribute      | Description                           | Example                             |
| :---           | :---                                  | :---                                |
| `constructor`  | Mark as class constructor             | `#[js_method(constructor)]`         |
| `rename = "x"` | Set JavaScript method/property name   | `#[js_method(rename = "moveBy")]`   |
| `getter`       | Expose as property getter             | `#[js_method(getter)]`              |
| `setter`       | Expose as property setter             | `#[js_method(setter, rename = "x")]`|
| `enumerable`   | Make property enumerable              | `#[js_method(getter, enumerable)]`  |

**Combining attributes:**

```rust
#[js_method(getter, enumerable, rename = "myProp")]
fn get_my_prop(&self) -> i32 { /* ... */ }
```

---

## Complete Examples

### Example 1: File System Module

```rust
use rong::*;

/// Read file contents
async fn read_file(path: String) -> JSResult<String> {
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| HostError::new("FS_IO", format!("Read failed: {}", e)).into())
}

/// Write file contents
async fn write_file(path: String, content: String) -> JSResult<()> {
    tokio::fs::write(&path, content)
        .await
        .map_err(|e| HostError::new("FS_IO", format!("Write failed: {}", e)).into())
}

/// Check if path exists
async fn exists(path: String) -> bool {
    tokio::fs::metadata(&path).await.is_ok()
}

pub fn init_fs(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.rong();

    let read_fn = JSFunc::new(ctx, read_file)?.name("readFile")?;
    rong.set("readFile", read_fn)?;

    let write_fn = JSFunc::new(ctx, write_file)?.name("writeFile")?;
    rong.set("writeFile", write_fn)?;

    let exists_fn = JSFunc::new(ctx, exists)?.name("exists")?;
    rong.set("exists", exists_fn)?;

    Ok(())
}
```

**JavaScript usage:**

```javascript
// All async operations return Promises
const exists = await Rong.exists("data.txt");
if (exists) {
    const content = await Rong.readFile("data.txt");
    console.log(content);
}

await Rong.writeFile("output.txt", "Hello, World!");
```

### Example 2: Point Class (Complete)

```rust
use rong::*;
use rong::{js_export, js_class, js_method};

#[js_export]
#[derive(Debug, Clone)]
struct Point {
    x: i32,
    y: i32,
}

#[js_class(rename = "Point2D")]
impl Point {
    // Constructor
    #[js_method(constructor)]
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    // Getters and setters
    #[js_method(getter, enumerable)]
    fn x(&self) -> i32 {
        self.x
    }

    #[js_method(setter, rename = "x")]
    fn set_x(&mut self, x: i32) {
        self.x = x;
    }

    #[js_method(getter, enumerable)]
    fn y(&self) -> i32 {
        self.y
    }

    #[js_method(setter, rename = "y")]
    fn set_y(&mut self, y: i32) {
        self.y = y;
    }

    // Instance methods
    #[js_method]
    fn distance(&self) -> f64 {
        ((self.x.pow(2) + self.y.pow(2)) as f64).sqrt()
    }

    #[js_method(rename = "add")]
    fn add(&self, other: Point) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    // Mutable method
    #[js_method(rename = "moveBy")]
    fn move_by(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }

    // Static methods
    #[js_method]
    fn origin() -> Self {
        Self { x: 0, y: 0 }
    }
}

fn main() {
    let rt = RongJS::runtime();
    let ctx = rt.context();

    // Register the class
    ctx.register_class::<Point>().unwrap();

    // Use from JavaScript
    let result = ctx.eval::<String>(Source::from_bytes(r#"
        const p1 = new Point2D(10, 20);
        const p2 = new Point2D(30, 40);

        p1.x = 15;  // Use setter
        p1.moveBy(5, 5);  // Now at (20, 25)

        const p3 = p1.add(p2);  // (50, 65)
        const origin = Point2D.origin();

        `p1: (${p1.x}, ${p1.y}), p3: (${p3.x}, ${p3.y}), origin: (${origin.x}, ${origin.y})`
    "#)).unwrap();

    println!("{}", result);
}
```

### Example 3: Storage Class with Options

```rust
use rong::*;
use rong::{js_export, js_class, js_method, FromJSObj};
use rong::function::Optional;

#[derive(FromJSObj, Default)]
pub struct StorageOptions {
    #[rename = "maxSize"]
    max_size: Option<u32>,

    compression: Option<bool>,
}

#[js_export]
pub struct Storage {
    path: String,
    max_size: u32,
    compression: bool,
}

#[js_class]
impl Storage {
    #[js_method(constructor)]
    fn new(path: String, options: Optional<StorageOptions>) -> JSResult<Self> {
        let opts = options.0.unwrap_or_default();

        Ok(Self {
            path,
            max_size: opts.max_size.unwrap_or(1024 * 1024),
            compression: opts.compression.unwrap_or(false),
        })
    }

    #[js_method]
    async fn set(&self, key: String, value: String) -> JSResult<()> {
        // Store key-value pair
        todo!()
    }

    #[js_method]
    async fn get(&self, key: String) -> JSResult<Option<String>> {
        // Retrieve value
        todo!()
    }

    #[js_method(getter)]
    fn path(&self) -> String {
        self.path.clone()
    }
}
```

**JavaScript usage:**

```javascript
const storage = new Storage("./data.db", {
    maxSize: 10485760,  // 10MB
    compression: true
});

await storage.set("user:1", JSON.stringify({ name: "Alice" }));
const data = await storage.get("user:1");
console.log(storage.path); // "./data.db"
```

---

## See Also

- [Value System and Type Conversion](./value_system.md) - Understanding type conversions between Rust and JavaScript
- [Error Handling](./error_handling.md) - Creating and throwing JavaScript errors from Rust
