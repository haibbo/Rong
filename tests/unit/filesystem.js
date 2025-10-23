describe("Filesystem", () => {
  // Helper function to get temporary file path
  function getTempPath(filename) {
    return `${WORKSPACE_ROOT}/target/test-tmp/${filename}`;
  }

  // Helper function to ensure temp directory exists
  async function ensureTempDir() {
    try {
      await Rong.mkdir(`${WORKSPACE_ROOT}/target/test-tmp`, {
        recursive: true,
      });
    } catch (e) {
      // Directory might already exist
    }
  }

  // Helper function to clean up temp directory
  async function cleanupTempDir() {
    try {
      await Rong.remove(`${WORKSPACE_ROOT}/target/test-tmp`, {
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

    await Rong.writeTextFile(testFile, testContent);
    const content = await Rong.readTextFile(testFile);
    assert.equal(content, testContent);

    // Test with options
    await Rong.writeTextFile(testFile, "Append", { append: true });
    const appendedContent = await Rong.readTextFile(testFile);
    assert.equal(appendedContent, testContent + "Append");

    // Test abort
    const controller = new AbortController();
    const promise = Rong.writeTextFile(testFile, "Should not write", {
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

    await Rong.writeFile(testFile, testData);
    const data = await Rong.readFile(testFile);
    const readData = new Uint8Array(data);

    // Compare length and content
    assert.equal(readData.length, testData.length, "Data length should match");
    for (let i = 0; i < testData.length; i++) {
      assert.equal(readData[i], testData[i], `Data at index ${i} should match`);
    }

    // Test with options
    const appendData = new Uint8Array([6, 7, 8]);
    await Rong.writeFile(testFile, appendData, { append: true });
    const appendedData = new Uint8Array(await Rong.readFile(testFile));

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
    await Rong.writeTextFile(testFile, "1234567890");

    await Rong.truncate(testFile, 5);
    const content = await Rong.readTextFile(testFile);
    assert.equal(content, "12345");

    await Rong.truncate(testFile); // Should truncate to 0
    const emptyContent = await Rong.readTextFile(testFile);
    assert.equal(emptyContent, "");

    await cleanupTempDir();
  });

  it("copyFile", async () => {
    await ensureTempDir();
    const sourceFile = getTempPath("source.txt");
    const destFile = getTempPath("dest.txt");
    const content = "Copy test content";

    await Rong.writeTextFile(sourceFile, content);
    await Rong.copyFile(sourceFile, destFile);

    const copiedContent = await Rong.readTextFile(destFile);
    assert.equal(copiedContent, content);

    await cleanupTempDir();
  });

  it("rename", async () => {
    await ensureTempDir();
    const oldPath = getTempPath("old.txt");
    const newPath = getTempPath("new.txt");
    const content = "Rename test content";

    await Rong.writeTextFile(oldPath, content);
    await Rong.rename(oldPath, newPath);

    const exists = await Rong.readTextFile(newPath);
    assert.equal(exists, content);

    let rejected = false;
    try {
      await Rong.readTextFile(oldPath);
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
    await Rong.writeTextFile(testFile, "To be removed");
    await Rong.remove(testFile);

    let rejected = false;
    try {
      await Rong.readTextFile(testFile);
    } catch (e) {
      rejected = true;
    }
    assert(rejected, "Reading removed file should fail");

    // Test directory removal
    const testDir = getTempPath("test_dir");
    const nestedDir = getTempPath("test_dir/nested");
    const nestedFile = getTempPath("test_dir/nested/test.txt");

    await Rong.mkdir(testDir);
    await Rong.mkdir(nestedDir, { recursive: true });
    await Rong.writeTextFile(nestedFile, "Test content");

    // Try to remove non-empty directory without recursive option
    rejected = false;
    try {
      await Rong.remove(testDir);
    } catch (e) {
      rejected = true;
    }
    assert(
      rejected,
      "Removing non-empty directory without recursive should fail",
    );

    // Remove directory recursively
    await Rong.remove(testDir, { recursive: true });

    // Verify directory is removed
    rejected = false;
    try {
      await Rong.readDir(testDir);
    } catch (e) {
      rejected = true;
    }
    assert(rejected, "Reading removed directory should fail");

    await cleanupTempDir();
  });

  it("realPath", async () => {
    await ensureTempDir();
    const testFile = getTempPath("real.txt");
    await Rong.writeTextFile(testFile, "");

    const realPath = await Rong.realPath(testFile);
    assert(realPath.endsWith(testFile));

    await cleanupTempDir();
  });

  it("mkdir and readDir", async () => {
    await ensureTempDir();
    const testDir = getTempPath("test_dir");
    const nestedDir = getTempPath("test_dir/nested");
    const testFile = getTempPath("test_dir/test.txt");

    // Test mkdir
    await Rong.mkdir(testDir);
    await Rong.mkdir(nestedDir, { recursive: true });
    await Rong.writeTextFile(testFile, "");

    // Test readDir
    const entries = [];
    const dirEntries = await Rong.readDir(testDir);
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
    await Rong.remove(testFile);
    await Rong.remove(nestedDir);
    await Rong.remove(testDir);

    await cleanupTempDir();
  });

  it("stat", async () => {
    await ensureTempDir();
    const testFile = getTempPath("stat.txt");
    const testContent = "Test content";
    await Rong.writeTextFile(testFile, testContent);

    const info = await Rong.stat(testFile);
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
    await Rong.writeTextFile(testFile, testContent);

    const info = await Rong.lstat(testFile);
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
    const readPromise = Rong.readTextFile(testFile, { signal });
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
    const writePromise = Rong.writeTextFile(testFile, "Should not write", {
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
      await Rong.remove(testFile);
    } catch {}

    await cleanupTempDir();
  });

  it("symlink and readlink", async () => {
    await ensureTempDir();
    const targetFile = getTempPath("target.txt");
    const linkFile = getTempPath("link.txt");
    const content = "Symlink test content";

    // Create target file first
    await Rong.writeTextFile(targetFile, content);

    // Get absolute paths
    const targetAbsPath = await Rong.realPath(targetFile);
    const linkAbsPath = `${WORKSPACE_ROOT}/target/test-tmp/link.txt`;

    // Create symlink and verify it
    await Rong.symlink(targetAbsPath, linkAbsPath);
    const linkTarget = await Rong.readlink(linkAbsPath);
    assert(
      linkTarget === targetAbsPath,
      "Link target should match absolute path",
    );

    // Read through symlink
    const linkContent = await Rong.readTextFile(linkAbsPath);
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
    await Rong.writeTextFile(testFile, "Test content");

    // Test chmod (Unix-like systems only)
    if (typeof Rong.chmod === "function") {
      await Rong.chmod(testFile, 0o600);
      const info = await Rong.stat(testFile);
      assert.equal(info.mode & 0o777, 0o600);
    }

    await cleanupTempDir();
  });

  it("chown", async () => {
    await ensureTempDir();
    const testFile = getTempPath("chown.txt");
    await Rong.writeTextFile(testFile, "Test content");

    // Test chown (Unix-like systems only)
    if (typeof Rong.chown === "function") {
      try {
        // Try to chown to current user (might fail if not root)
        await Rong.chown(testFile, process.getuid(), process.getgid());
      } catch (e) {
        // Ignore permission errors
      }
    }

    await cleanupTempDir();
  });

  it("chdir", async () => {
    await ensureTempDir();
    const testDir = getTempPath("chdir_test");
    await Rong.mkdir(testDir);

    const oldPath = await Rong.realPath(".");
    await Rong.chdir(testDir);

    const newPath = await Rong.realPath(".");
    assert(newPath.endsWith("chdir_test"));

    await Rong.chdir(oldPath);
    await Rong.remove(testDir);

    await cleanupTempDir();
  });

  it("utime", async () => {
    await ensureTempDir();
    const testFile = getTempPath("utime.txt");
    await Rong.writeTextFile(testFile, "Test content");

    const now = Date.now();
    await Rong.utime(testFile, {
      accessed: now,
      modified: now,
    });

    const info = await Rong.stat(testFile);
    // Allow 1 second difference due to timestamp precision
    assert(Math.abs(info.accessed - now) < 1000);
    assert(Math.abs(info.modified - now) < 1000);

    await cleanupTempDir();
  });

  it("FsFile - basic operations", async () => {
    await ensureTempDir();
    const testFile = getTempPath("basic.txt");

    // Create test file
    await Rong.writeTextFile(testFile, "Hello from FsFile!");

    // Test open, stat, truncate, sync, close
    const file = await Rong.open(testFile, { read: true, write: true });

    const stats = await file.stat();
    assert(stats.isFile && !stats.isDirectory && !stats.isSymlink);
    assert(stats.size > 0 && typeof stats.modified === "number");

    await file.truncate(5);
    const statAfterTruncate = await file.stat();
    assert.equal(
      statAfterTruncate.size,
      5,
      "File should be truncated to 5 bytes",
    );

    await file.sync();
    await file.close();

    await cleanupTempDir();
  });

  it("FsFile - read/write operations", async () => {
    await ensureTempDir();

    const testFile = getTempPath("readwrite.dat");
    const testData = new Uint8Array([1, 2, 3, 4, 5, 255, 0, 128]);

    // Write binary data
    const file1 = await Rong.open(testFile, { write: true, create: true });
    const arrayBuffer = testData.buffer.slice(
      testData.byteOffset,
      testData.byteOffset + testData.byteLength,
    );
    const bytesWritten = await file1.write(arrayBuffer);
    assert.equal(bytesWritten, 8, "Should write 8 bytes");
    await file1.close();

    // Read and verify data
    const file2 = await Rong.open(testFile, { read: true });
    const readBuffer = new ArrayBuffer(8);
    const bytesRead = await file2.read(readBuffer);
    assert.equal(bytesRead, 8, "Should read 8 bytes");

    const readArray = new Uint8Array(readBuffer);
    for (let i = 0; i < testData.length; i++) {
      assert.equal(readArray[i], testData[i], `Byte ${i} should match`);
    }

    // Test EOF
    const eofBuffer = new ArrayBuffer(4);
    const eofResult = await file2.read(eofBuffer);
    assert.equal(eofResult, null, "Should return null at EOF");
    await file2.close();

    // Test append mode
    const file3 = await Rong.open(testFile, { write: true, append: true });
    const appendData = new Uint8Array([9, 10]);
    await file3.write(
      appendData.buffer.slice(
        appendData.byteOffset,
        appendData.byteOffset + appendData.byteLength,
      ),
    );
    await file3.close();

    // Verify append
    const file4 = await Rong.open(testFile, { read: true });
    const fullBuffer = new ArrayBuffer(10);
    const totalRead = await file4.read(fullBuffer);
    assert.equal(totalRead, 10, "Should read 10 bytes after append");
    await file4.close();

    await cleanupTempDir();
  });

  it("FsFile.readable returns ReadableStream (reader)", async () => {
    await ensureTempDir();
    const srcPath = getTempPath("stream_src.txt");

    // Prepare large content to ensure multiple chunks
    const chunk = "chunk_" + "A".repeat(1024);
    let text = "";
    for (let i = 0; i < 200; i++) text += chunk + "\n";
    await Rong.writeTextFile(srcPath, text);

    // Open and read via ReadableStream reader
    const file = await Rong.open(srcPath, { read: true });
    const rs = file.readable;
    expect(rs instanceof ReadableStream).toBe(true);
    const reader = rs.getReader();
    let total = 0;
    const bufs = [];
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      bufs.push(new Uint8Array(value));
      total += value.byteLength;
    }
    await file.close();

    const all = new Uint8Array(total);
    let off = 0;
    for (const b of bufs) {
      all.set(b, off);
      off += b.byteLength;
    }
    const decoded = new TextDecoder().decode(all);
    assert(decoded.includes("chunk_"));
    assert(decoded.length === text.length);

    await Rong.remove(srcPath);
  });

  it("FsFile.readable.pipeTo WritableStream (copy)", async () => {
    await ensureTempDir();
    const srcPath = getTempPath("stream_src_copy.txt");
    const dstPath = getTempPath("stream_dst_copy.txt");

    // Create source
    const payload = "COPY_".repeat(50_000); // ~250kB
    await Rong.writeTextFile(srcPath, payload);

    // Open src and dst
    const src = await Rong.open(srcPath, { read: true });
    const dst = await Rong.open(dstPath, {
      write: true,
      create: true,
      truncate: true,
    });

    await src.readable.pipeTo(dst.writable);
    await src.close();
    await dst.close();

    const copied = await Rong.readTextFile(dstPath);
    assert.equal(copied.length, payload.length);
    assert(copied.startsWith("COPY_"));
    assert(copied.endsWith("COPY_"));

    await Rong.remove(srcPath);
    await Rong.remove(dstPath);
  });

  it("FsFile.readable supports async iteration (for-await)", async () => {
    await ensureTempDir();
    const srcPath = getTempPath("stream_for_await.txt");

    // Create source content with recognizable markers
    let text = "";
    for (let i = 0; i < 100; i++) {
      const line = `iter_${String(i).padStart(4, "0")}\n`;
      text += line.repeat(512); // make it large enough for multiple chunks
    }
    await Rong.writeTextFile(srcPath, text);

    const file = await Rong.open(srcPath, { read: true });
    const rs = file.readable;
    expect(rs instanceof ReadableStream).toBe(true);

    // Verify async iterator exists on instance and returns itself
    const ai = rs[Symbol.asyncIterator]?.call(rs);
    expect(ai === rs).toBe(true);

    const decoder = new TextDecoder();
    let total = 0;
    let acc = "";
    let seenStart = false;
    let seenEnd = false;
    for await (const chunk of rs) {
      total += chunk.byteLength;
      acc += decoder.decode(chunk);
      if (!seenStart && acc.includes("iter_0000")) seenStart = true;
      if (!seenEnd && acc.includes("iter_0099")) seenEnd = true;
    }
    await file.close();

    assert(total > 0);
    assert(seenStart);
    assert(seenEnd);
    assert.equal(acc.length, text.length);

    await Rong.remove(srcPath);
  });

  it("FsFile - OpenOptions", async () => {
    await ensureTempDir();

    const testFile = getTempPath("options.dat");

    // Test createNew - should create new file
    const file1 = await Rong.open(testFile, { write: true, createNew: true });
    await file1.write(new TextEncoder().encode("new file").buffer);
    await file1.close();

    // Test createNew - should fail if file exists
    let failed = false;
    try {
      await Rong.open(testFile, { write: true, createNew: true });
    } catch (e) {
      failed = true;
    }
    assert(failed, "createNew should fail if file already exists");

    // Test truncate - should clear file and write new content
    const file2 = await Rong.open(testFile, { write: true, truncate: true });
    await file2.write(new TextEncoder().encode("truncated").buffer);
    await file2.close();

    const content = await Rong.readTextFile(testFile);
    assert.equal(
      content,
      "truncated",
      "File should be truncated and contain new content",
    );

    await cleanupTempDir();
  });

  it("FsFile - error handling", async () => {
    await ensureTempDir();

    // Test constructor should not be callable
    let constructorFailed = false;
    try {
      new FsFile();
    } catch (e) {
      constructorFailed = true;
    }
    assert(
      constructorFailed,
      "Should not be able to construct FsFile directly",
    );

    // Test opening non-existent file without create
    let openFailed = false;
    try {
      await Rong.open(getTempPath("nonexistent.txt"), { read: true });
    } catch (e) {
      openFailed = true;
    }
    assert(openFailed, "Opening non-existent file should fail");

    await cleanupTempDir();
  });

  it("FsFile - seek operations", async () => {
    await ensureTempDir();

    const testFile = getTempPath("seek.txt");
    const testData = "Hello, World! This is a test file for seeking.";

    // Create test file with known content
    {
      const file = await Rong.open(testFile, { write: true, create: true });
      const encoder = new TextEncoder();
      const data = encoder.encode(testData);
      await file.write(
        data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength),
      );
      await file.close();
    }

    // Test seek operations
    {
      const file = await Rong.open(testFile, { read: true, write: true });

      // Test seek from start (default) - using Rong.SeekMode.Start
      let position = await file.seek(7, Rong.SeekMode.Start); // Position at "World!"
      assert.equal(position, 7, "Should seek to position 7");

      // Read from current position to verify
      const buffer1 = new ArrayBuffer(6);
      const bytesRead1 = await file.read(buffer1);
      assert.equal(bytesRead1, 6, "Should read 6 bytes");
      const text1 = new TextDecoder().decode(new Uint8Array(buffer1));
      assert.equal(text1, "World!", "Should read 'World!' from position 7");

      // Test seek from current position
      position = await file.seek(-6, Rong.SeekMode.Current); // Go back to "World!"
      assert.equal(position, 7, "Should be back at position 7");

      // Test seek from end
      position = await file.seek(-5, Rong.SeekMode.End); // Position 5 bytes from end
      const buffer2 = new ArrayBuffer(5);
      const bytesRead2 = await file.read(buffer2);
      assert.equal(bytesRead2, 5, "Should read 5 bytes");
      const text2 = new TextDecoder().decode(new Uint8Array(buffer2));
      // Read the actual last 5 characters from the expected position
      const expectedText = testData.slice(-5); // Last 5 characters: "ing."
      assert.equal(
        text2,
        expectedText,
        `Should read '${expectedText}' from end-5 position`,
      );

      // Test seek to start explicitly
      position = await file.seek(0, Rong.SeekMode.Start);
      assert.equal(position, 0, "Should be at start of file");

      const buffer3 = new ArrayBuffer(5);
      const bytesRead3 = await file.read(buffer3);
      assert.equal(bytesRead3, 5, "Should read 5 bytes");
      const text3 = new TextDecoder().decode(new Uint8Array(buffer3));
      assert.equal(text3, "Hello", "Should read 'Hello' from start");

      // Test seek beyond file end (should work)
      position = await file.seek(1000, Rong.SeekMode.Start);
      assert.equal(position, 1000, "Should seek beyond file end");

      // Reading from beyond end should return null
      const bufferEOF = new ArrayBuffer(10);
      const eofResult = await file.read(bufferEOF);
      assert.equal(
        eofResult,
        null,
        "Should return null when reading beyond EOF",
      );

      await file.close();
    }

    // Test error handling
    {
      const file = await Rong.open(testFile, { read: true });

      // Test invalid whence value
      let errorThrown = false;
      try {
        await file.seek(0, 999); // Invalid seek mode
      } catch (e) {
        errorThrown = true;
        console.log("Invalid whence error (expected):", e.message);
      }
      assert(errorThrown, "Should throw error for invalid whence value");

      await file.close();
    }

    await cleanupTempDir();
  });

  // FsFile streams (readable + writable)
  it("FsFile.streams: readable", async () => {
    await ensureTempDir();
    const path = getTempPath("fs_readable_stream_test.bin");

    // Prepare source data (>64KiB to cross chunk boundaries)
    const total = 128 * 1024 + 123;
    const src = new Uint8Array(total);
    for (let i = 0; i < total; i++) src[i] = i % 251;
    await Rong.writeFile(path, src);

    const file = await Rong.open(path, { read: true });
    const rs = file.readable;
    assert(rs instanceof ReadableStream);

    const reader = rs.getReader();
    const chunks = [];
    let size = 0;
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      const part = new Uint8Array(value);
      chunks.push(part);
      size += part.byteLength;
    }
    await reader.releaseLock();
    await file.close();

    // Concatenate chunks
    const out = new Uint8Array(size);
    let offset = 0;
    for (const c of chunks) {
      out.set(c, offset);
      offset += c.byteLength;
    }

    assert.equal(size, total, "total bytes read should match");
    for (let i = 0; i < total; i++) {
      assert.equal(out[i], src[i], `byte ${i} should match`);
    }
  });

  it("FsFile.streams: writable", async () => {
    await ensureTempDir();
    const path = getTempPath("fs_writable_stream_test.txt");

    // Open file with write/create/truncate
    const file = await Rong.open(path, {
      write: true,
      create: true,
      truncate: true,
    });
    const ws = file.writable;
    assert(ws instanceof WritableStream);

    // Write via writer
    const writer = ws.getWriter();
    const enc = new TextEncoder();
    await writer.write(enc.encode("hello"));
    await writer.write(enc.encode("_stream_"));
    await writer.write(enc.encode("world"));
    await writer.close();

    // Verify content
    const data = new Uint8Array(await Rong.readFile(path));
    const text = new TextDecoder().decode(data);
    assert.equal(text, "hello_stream_world");
  });
});
