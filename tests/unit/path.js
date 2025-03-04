describe("Path", () => {
  it("should correctly handle basename", () => {
    expect(path.basename("/foo/bar/baz.html")).toBe("baz.html");
    expect(path.basename("/foo/bar/baz.html", ".html")).toBe("baz");
    expect(path.basename("/foo/bar/baz")).toBe("baz");
    expect(path.basename("/")).toBe("");
    expect(path.basename("")).toBe("");
  });

  it("should correctly handle dirname", () => {
    expect(path.dirname("/foo/bar/baz")).toBe("/foo/bar");
    expect(path.dirname("/foo/bar/baz/")).toBe("/foo/bar");
    expect(path.dirname("/foo")).toBe("/");
    expect(path.dirname("foo")).toBe(".");
    expect(path.dirname("")).toBe(".");
  });

  it("should correctly handle extname", () => {
    expect(path.extname("index.html")).toBe(".html");
    expect(path.extname("index.coffee.md")).toBe(".md");
    expect(path.extname("index.")).toBe(".");
    expect(path.extname("index")).toBe("");
    expect(path.extname(".index")).toBe("");
  });

  it("should correctly handle isAbsolute", () => {
    expect(path.isAbsolute("/foo/bar")).toBe(true);
    expect(path.isAbsolute("foo/bar")).toBe(false);
    expect(path.isAbsolute("./foo/bar")).toBe(false);
  });

  it("should correctly handle join", () => {
    expect(path.join("/foo", "bar", "baz")).toBe("/foo/bar/baz");
    expect(path.join("/foo", "bar", "../baz")).toBe("/foo/baz");
    expect(path.join("foo", "bar", "baz")).toBe("foo/bar/baz");
    expect(path.join("")).toBe(".");
  });

  it("should correctly handle normalize", () => {
    expect(path.normalize("/foo/bar//baz/asdf/quux/..")).toBe(
      "/foo/bar/baz/asdf",
    );
    expect(path.normalize("foo/bar//baz/asdf/quux/..")).toBe(
      "foo/bar/baz/asdf",
    );
    expect(path.normalize("/foo/../bar")).toBe("/bar");
    expect(path.normalize("foo/..")).toBe(".");
  });

  it("should correctly handle parse", () => {
    const parsed = path.parse("/home/user/dir/file.txt");
    expect(parsed.root).toBe("/");
    expect(parsed.dir).toBe("/home/user/dir");
    expect(parsed.base).toBe("file.txt");
    expect(parsed.ext).toBe(".txt");
    expect(parsed.name).toBe("file");
  });

  it("should correctly handle format", () => {
    expect(
      path.format({
        root: "/",
        dir: "/home/user/dir",
        base: "file.txt",
      }),
    ).toBe("/home/user/dir/file.txt");

    expect(
      path.format({
        dir: "/home/user/dir",
        name: "file",
        ext: ".txt",
      }),
    ).toBe("/home/user/dir/file.txt");
  });

  it("should handle platform specific values", () => {
    expect(path.sep).toBe("/");
    expect(path.delimiter).toBe(":");
  });

  it("should handle edge cases", () => {
    expect(path.normalize("")).toBe(".");
    expect(path.join("", "")).toBe(".");
  });

  it("should support Unicode", () => {
    expect(path.basename("/foo/bar/文件.txt")).toBe("文件.txt");
    expect(path.basename("/foo/bar/文件.txt", ".txt")).toBe("文件");
    expect(path.extname("文件.txt")).toBe(".txt");
  });
});

