describe("Filesystem", () => {
  // Helper function to get temporary file path
  function getTempPath(filename) {
    return `${WORKSPACE_ROOT}/target/test-tmp/${filename}`;
  }

  // Helper function to ensure temp directory exists
  async function ensureTempDir() {
    try {
      await Danity.mkdir(`${WORKSPACE_ROOT}/target/test-tmp`, {
        recursive: true,
      });
    } catch (e) {
      // Directory might already exist
    }
  }

  // Helper function to clean up temp directory
  async function cleanupTempDir() {
    try {
      await Danity.remove(`${WORKSPACE_ROOT}/target/test-tmp`, {
        recursive: true,
      });
    } catch (e) {
      // Directory might not exist or be already removed
    }
  }

  it("readTextFile and writeTextFile", async () => {
    await ensureTempDir();
    const testFile = getTempPath("test.txt");
    const testContent = "Hello, World!";

    await Danity.writeTextFile(testFile, testContent);
    const content = await Danity.readTextFile(testFile);
    assert.equal(content, testContent);

    // Test with options
    await Danity.writeTextFile(testFile, "Append", { append: true });
    const appendedContent = await Danity.readTextFile(testFile);
    assert.equal(appendedContent, testContent + "Append");

    // Test abort
    const controller = new AbortController();
    const promise = Danity.writeTextFile(testFile, "Should not write", {
      signal: controller.signal,
    });
    controller.abort();

    let error;
    try {
      await promise;
    } catch (e) {
      error = e;
    }
    assert(error instanceof DOMException);
    assert.equal(error.name, "AbortError");

    await cleanupTempDir();
  });

  it("readFile and writeFile", async () => {
    await ensureTempDir();
    const testFile = getTempPath("test.bin");
    const testData = new Uint8Array([1, 2, 3, 4, 5]);

    await Danity.writeFile(testFile, testData);
    const data = await Danity.readFile(testFile);
    const readData = new Uint8Array(data);

    // Compare length and content
    assert.equal(readData.length, testData.length, "Data length should match");
    for (let i = 0; i < testData.length; i++) {
      assert.equal(readData[i], testData[i], `Data at index ${i} should match`);
    }

    // Test with options
    const appendData = new Uint8Array([6, 7, 8]);
    await Danity.writeFile(testFile, appendData, { append: true });
    const appendedData = new Uint8Array(await Danity.readFile(testFile));

    // Verify appended data
    assert.equal(
      appendedData.length,
      testData.length + appendData.length,
      "Appended data length should match",
    );
    const expectedData = new Uint8Array([...testData, ...appendData]);
    for (let i = 0; i < expectedData.length; i++) {
      assert.equal(
        appendedData[i],
        expectedData[i],
        `Appended data at index ${i} should match`,
      );
    }

    await cleanupTempDir();
  });

  it("truncate", async () => {
    await ensureTempDir();
    const testFile = getTempPath("truncate.txt");
    await Danity.writeTextFile(testFile, "1234567890");

    await Danity.truncate(testFile, 5);
    const content = await Danity.readTextFile(testFile);
    assert.equal(content, "12345");

    await Danity.truncate(testFile); // Should truncate to 0
    const emptyContent = await Danity.readTextFile(testFile);
    assert.equal(emptyContent, "");

    await cleanupTempDir();
  });

  it("copyFile", async () => {
    await ensureTempDir();
    const sourceFile = getTempPath("source.txt");
    const destFile = getTempPath("dest.txt");
    const content = "Copy test content";

    await Danity.writeTextFile(sourceFile, content);
    await Danity.copyFile(sourceFile, destFile);

    const copiedContent = await Danity.readTextFile(destFile);
    assert.equal(copiedContent, content);

    await cleanupTempDir();
  });

  it("rename", async () => {
    await ensureTempDir();
    const oldPath = getTempPath("old.txt");
    const newPath = getTempPath("new.txt");
    const content = "Rename test content";

    await Danity.writeTextFile(oldPath, content);
    await Danity.rename(oldPath, newPath);

    const exists = await Danity.readTextFile(newPath);
    assert.equal(exists, content);

    let rejected = false;
    try {
      await Danity.readTextFile(oldPath);
    } catch (e) {
      rejected = true;
    }
    assert(rejected, "Reading old file should fail");

    await cleanupTempDir();
  });

  it("remove", async () => {
    // Test file removal
    await ensureTempDir();
    const testFile = getTempPath("remove.txt");
    await Danity.writeTextFile(testFile, "To be removed");
    await Danity.remove(testFile);

    let rejected = false;
    try {
      await Danity.readTextFile(testFile);
    } catch (e) {
      rejected = true;
    }
    assert(rejected, "Reading removed file should fail");

    // Test directory removal
    const testDir = getTempPath("test_dir");
    const nestedDir = getTempPath("test_dir/nested");
    const nestedFile = getTempPath("test_dir/nested/test.txt");

    await Danity.mkdir(testDir);
    await Danity.mkdir(nestedDir, { recursive: true });
    await Danity.writeTextFile(nestedFile, "Test content");

    // Try to remove non-empty directory without recursive option
    rejected = false;
    try {
      await Danity.remove(testDir);
    } catch (e) {
      rejected = true;
    }
    assert(
      rejected,
      "Removing non-empty directory without recursive should fail",
    );

    // Remove directory recursively
    await Danity.remove(testDir, { recursive: true });

    // Verify directory is removed
    rejected = false;
    try {
      await Danity.readDir(testDir);
    } catch (e) {
      rejected = true;
    }
    assert(rejected, "Reading removed directory should fail");

    await cleanupTempDir();
  });

  it("realPath", async () => {
    await ensureTempDir();
    const testFile = getTempPath("real.txt");
    await Danity.writeTextFile(testFile, "");

    const realPath = await Danity.realPath(testFile);
    assert(realPath.endsWith(testFile));

    await cleanupTempDir();
  });

  it("mkdir and readDir", async () => {
    await ensureTempDir();
    const testDir = getTempPath("test_dir");
    const nestedDir = getTempPath("test_dir/nested");
    const testFile = getTempPath("test_dir/test.txt");

    // Test mkdir
    await Danity.mkdir(testDir);
    await Danity.mkdir(nestedDir, { recursive: true });
    await Danity.writeTextFile(testFile, "");

    // Test readDir
    const entries = [];
    const dirEntries = await Danity.readDir(testDir);
    for await (const entry of dirEntries) {
      console.log(
        "entry:%s, isFile:%s, isDirectory:%s",
        entry.name,
        entry.isFile,
        entry.isDirectory,
      );
      entries.push(entry);
    }

    assert.equal(entries.length, 2);
    const hasNestedDir = entries.some(
      (e) => e.name === "nested" && e.isDirectory,
    );
    const hasTestFile = entries.some((e) => e.name === "test.txt" && e.isFile);
    assert(hasNestedDir, "Should have nested directory");
    assert(hasTestFile, "Should have test file");

    // Cleanup
    await Danity.remove(testFile);
    await Danity.remove(nestedDir);
    await Danity.remove(testDir);

    await cleanupTempDir();
  });

  it("stat", async () => {
    await ensureTempDir();
    const testFile = getTempPath("stat.txt");
    const testContent = "Test content";
    await Danity.writeTextFile(testFile, testContent);

    const info = await Danity.stat(testFile);
    assert(info.isFile);
    assert(!info.isDirectory);
    assert(!info.isSymlink);
    assert.equal(info.size, testContent.length);
    assert(typeof info.modified === "number");
    assert(typeof info.accessed === "number");
    if (info.mode) {
      assert(typeof info.mode === "number");
    }

    await cleanupTempDir();
  });

  it("lstat", async () => {
    await ensureTempDir();
    const testFile = getTempPath("lstat.txt");
    const testContent = "Test content";
    await Danity.writeTextFile(testFile, testContent);

    const info = await Danity.lstat(testFile);
    assert(info.isFile);
    assert(!info.isDirectory);
    assert(!info.isSymlink);
    assert.equal(info.size, testContent.length);
    assert(typeof info.modified === "number");
    assert(typeof info.accessed === "number");
    if (info.mode) {
      assert(typeof info.mode === "number");
    }

    await cleanupTempDir();
  });

  it("abort signal", async () => {
    await ensureTempDir();
    const testFile = getTempPath("abort.txt");
    const controller = new AbortController();
    const { signal } = controller;

    // Test readTextFile abort
    const readPromise = Danity.readTextFile(testFile, { signal });
    controller.abort();

    let error;
    try {
      await readPromise;
    } catch (e) {
      error = e;
    }
    assert(error instanceof DOMException);
    assert.equal(error.name, "AbortError");

    // Test writeTextFile abort with new controller
    const writeController = new AbortController();
    const writePromise = Danity.writeTextFile(testFile, "Should not write", {
      signal: writeController.signal,
    });
    writeController.abort();

    error = undefined;
    try {
      await writePromise;
    } catch (e) {
      error = e;
    }
    assert(error instanceof DOMException);
    assert.equal(error.name, "AbortError");

    // Clean up if file was created
    try {
      await Danity.remove(testFile);
    } catch {}

    await cleanupTempDir();
  });

  it("symlink and readlink", async () => {
    await ensureTempDir();
    const targetFile = getTempPath("target.txt");
    const linkFile = getTempPath("link.txt");
    const content = "Symlink test content";

    // Create target file first
    await Danity.writeTextFile(targetFile, content);

    // Get absolute paths
    const targetAbsPath = await Danity.realPath(targetFile);
    const linkAbsPath = `${WORKSPACE_ROOT}/target/test-tmp/link.txt`;

    // Create symlink and verify it
    await Danity.symlink(targetAbsPath, linkAbsPath);
    const linkTarget = await Danity.readlink(linkAbsPath);
    assert(
      linkTarget === targetAbsPath,
      "Link target should match absolute path",
    );

    // Read through symlink
    const linkContent = await Danity.readTextFile(linkAbsPath);
    assert.equal(
      linkContent,
      content,
      "Content read through symlink should match",
    );

    await cleanupTempDir();
  });

  it("chmod", async () => {
    await ensureTempDir();
    const testFile = getTempPath("chmod.txt");
    await Danity.writeTextFile(testFile, "Test content");

    // Test chmod (Unix-like systems only)
    if (typeof Danity.chmod === "function") {
      await Danity.chmod(testFile, 0o600);
      const info = await Danity.stat(testFile);
      assert.equal(info.mode & 0o777, 0o600);
    }

    await cleanupTempDir();
  });

  it("chown", async () => {
    await ensureTempDir();
    const testFile = getTempPath("chown.txt");
    await Danity.writeTextFile(testFile, "Test content");

    // Test chown (Unix-like systems only)
    if (typeof Danity.chown === "function") {
      try {
        // Try to chown to current user (might fail if not root)
        await Danity.chown(testFile, process.getuid(), process.getgid());
      } catch (e) {
        // Ignore permission errors
      }
    }

    await cleanupTempDir();
  });

  it("chdir", async () => {
    await ensureTempDir();
    const testDir = getTempPath("chdir_test");
    await Danity.mkdir(testDir);

    const oldPath = await Danity.realPath(".");
    await Danity.chdir(testDir);

    const newPath = await Danity.realPath(".");
    assert(newPath.endsWith("chdir_test"));

    await Danity.chdir(oldPath);
    await Danity.remove(testDir);

    await cleanupTempDir();
  });

  it("utime", async () => {
    await ensureTempDir();
    const testFile = getTempPath("utime.txt");
    await Danity.writeTextFile(testFile, "Test content");

    const now = Date.now();
    await Danity.utime(testFile, {
      accessed: now,
      modified: now,
    });

    const info = await Danity.stat(testFile);
    // Allow 1 second difference due to timestamp precision
    assert(Math.abs(info.accessed - now) < 1000);
    assert(Math.abs(info.modified - now) < 1000);

    await cleanupTempDir();
  });
});
