// Streaming file downloader (progress + low memory)
// Usage: rong downloader.js <url> <output-filename>

async function downloadFile(url, outputPath) {
  let file = null;
  let writer = null;
  try {
    console.log(`Downloading from: ${url}`);
    console.log(`Saving to: ${outputPath}`);

    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(
        `Failed to download file: ${response.status} ${response.statusText}`,
      );
    }

    const contentLength = Number(response.headers.get("content-length")) || 0;

    // Prepare output file + writable stream
    file = await Rong.open(outputPath, {
      write: true,
      create: true,
      truncate: true,
    });
    writer = file.writable.getWriter();

    // Stream body to file using async iterator
    const start = Date.now();
    let received = 0;
    const body = response.body;

    if (body) {
      for await (const chunk of body) {
        await writer.write(chunk);
        received += chunk?.byteLength || 0;
        if (contentLength) {
          const percent = ((received / contentLength) * 100).toFixed(1);
          console.log(
            `Progress: ${percent}% (${received}/${contentLength} bytes)`,
          );
        }
      }
    } else {
      throw new Error("Response has no body stream");
    }

    await writer.close();
    await file.close();
    writer = null;
    file = null;

    const elapsed = ((Date.now() - start) / 1000).toFixed(2);
    console.log(
      `\n✅ Downloaded ${received} bytes in ${elapsed}s -> ${outputPath}`,
    );
  } catch (error) {
    console.error(`❌ Error: ${error.message}`);
    // Cleanup partial file on failure
    try {
      if (writer) await writer.abort();
      if (file) await file.close();
      await Rong.remove(outputPath);
    } catch {}
  }
}

// Get command line arguments
const args = Rong.args;

if (args.length < 3) {
  console.log("Usage: rong downloader.js <url> <output-filename>");
  Rong.exit(1);
}

const url = args[1];
const outputPath = args[2];

// Execute the download
downloadFile(url, outputPath);
