# Path

Path manipulation utilities, compatible with Node.js `path` module.

## Methods

```javascript
path.join("a", "b", "c");        // "a/b/c"
path.resolve("./src", "index");  // "/abs/path/src/index"
path.normalize("a//b/../c");     // "a/c"

path.basename("/a/b/file.txt");           // "file.txt"
path.basename("/a/b/file.txt", ".txt");   // "file"
path.dirname("/a/b/file.txt");            // "/a/b"
path.extname("file.tar.gz");              // ".gz"

path.isAbsolute("/usr/bin");     // true
path.isAbsolute("./src");       // false
```

## parse & format

```javascript
path.parse("/home/user/file.txt");
// { root: "/", dir: "/home/user", base: "file.txt", ext: ".txt", name: "file" }

path.format({ dir: "/home/user", name: "file", ext: ".txt" });
// "/home/user/file.txt"
```

## Constants

```javascript
path.sep;        // "/" (Unix) or "\\" (Windows)
path.delimiter;  // ":" (Unix) or ";" (Windows)
```
