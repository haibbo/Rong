// Resumable Streaming File Uploader (chunked + Content-Range / Upload-Offset)
// Usage: rong uploader.js <file-path> <server-url>

const CHUNK_SIZE = 4 * 1024 * 1024; // 4 MiB chunks

async function headProbeOffset(serverUrl) {
  try {
    const res = await fetch(serverUrl, { method: "HEAD" });
    if (!res.ok) return 0;
    const off =
      res.headers.get("Upload-Offset") || res.headers.get("X-Upload-Offset");
    return off ? Number(off) || 0 : 0;
  } catch {
    return 0;
  }
}

async function loadLocalResume(resumePath) {
  try {
    const txt = await Rong.file(resumePath).text();
    const obj = JSON.parse(txt);
    return Number(obj.offset) || 0;
  } catch {
    return 0;
  }
}

async function saveLocalResume(resumePath, offset) {
  try {
    await Rong.write(resumePath, JSON.stringify({ offset }));
  } catch {}
}

async function deleteLocalResume(resumePath) {
  try {
    await Rong.remove(resumePath);
  } catch {}
}

async function readChunk(file, offset, length) {
  // Position the file to desired offset, then read up to length bytes to a new Uint8Array
  await file.seek(offset, Rong.SeekMode.Start);
  const buf = new Uint8Array(length);
  let filled = 0;
  while (filled < length) {
    const remain = length - filled;
    const ab = new ArrayBuffer(remain);
    const n = await file.read(ab);
    if (n == null) break; // EOF
    const view = new Uint8Array(ab, 0, n);
    buf.set(view, filled);
    filled += n;
    if (n === 0) break;
  }
  return buf.subarray(0, filled);
}

async function uploadFile(localPath, serverUrl) {
  let file = null;
  try {
    console.log(`Uploading file (resumable): ${localPath}`);
    console.log(`To server: ${serverUrl}`);

    const stat = await Rong.file(localPath).stat();
    const total = Number(stat.size) || 0;
    if (!total) throw new Error("File size is zero or unknown");

    const resumePath = `${localPath}.upload.resume`;
    // Probe server offset (optional, depends on server support)
    const serverOffset = await headProbeOffset(serverUrl);
    // Load local resume checkpoint
    const localOffset = await loadLocalResume(resumePath);
    let offset = Math.max(serverOffset, localOffset);
    if (offset > total) offset = 0;

    file = await Rong.file(localPath).open({ read: true });
    console.log(
      `Starting upload at offset ${offset}/${total} (chunk=${CHUNK_SIZE} bytes)`,
    );

    const started = Date.now();
    while (offset < total) {
      const end = Math.min(offset + CHUNK_SIZE, total);
      const size = end - offset;
      const chunk = await readChunk(file, offset, size);

      const headers = {
        "Content-Length": String(chunk.byteLength),
        "Content-Type": "application/octet-stream",
        // Provide both hints; server may honor either
        "Content-Range": `bytes ${offset}-${end - 1}/${total}`,
        "Upload-Offset": String(offset),
      };

      const res = await fetch(serverUrl, {
        method: "PUT",
        headers,
        body: chunk,
      });
      if (!res.ok) {
        const text = await res.text().catch(() => "");
        throw new Error(
          `Chunk upload failed at ${offset}: ${res.status} ${res.statusText} ${text}`,
        );
      }

      offset = end;
      await saveLocalResume(resumePath, offset);

      const percent = ((offset / total) * 100).toFixed(1);
      console.log(`Progress: ${percent}% (${offset}/${total} bytes)`);
    }

    console.log("\nUpload complete");
    const elapsed = ((Date.now() - started) / 1000).toFixed(2);
    console.log(`✅ Uploaded ${total} bytes in ${elapsed}s`);
    await deleteLocalResume(resumePath);
  } catch (error) {
    console.error("❌ Upload error:", error.message);
  } finally {
    try {
      if (file) await file.close();
    } catch {}
  }
}

// Get command line arguments
const args = Rong.args;

if (args.length < 3) {
  console.log("Usage: rong uploader.js <file-path> <server-url>");
  Rong.exit(1);
}

const localFile = args[1];
const serverUrl = args[2];

// Execute the upload
uploadFile(localFile, serverUrl);
