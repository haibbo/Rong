# rong_path

Path manipulation utilities available under the global `path` object.

## JS APIs

- `path.basename(path, suffix?)` — return the last portion of a path
- `path.dirname(path)` — return the directory name of a path
- `path.extname(path)` — return the file extension (including the dot)
- `path.isAbsolute(path)` — check whether a path is absolute
- `path.join(...paths)` — join path segments together
- `path.normalize(path)` — normalize a path, resolving `..` and `.` segments
- `path.resolve(...paths)` — resolve a sequence of paths to an absolute path
- `path.parse(path)` — parse a path into `{ root, dir, base, ext, name }`
- `path.format(pathObject)` — format a parsed path object back into a string
- `path.sep` — platform-specific path separator (`/` or `\\`)
- `path.delimiter` — platform-specific PATH delimiter (`:` or `;`)
