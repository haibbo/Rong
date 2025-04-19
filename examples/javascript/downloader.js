// Tiny File downloader
// Usage: rong run downloader.js <url> <output-filename>

async function downloadFile(url, outputPath) {
  try {
    console.log(`Downloading from: ${url}`);
    console.log(`Saving to: ${outputPath}`);

    // Fetch the file
    console.log("Starting fetch");
    const response = await fetch(url);
    console.log("Fetch done");

    if (!response.ok) {
      throw new Error(
        `Failed to download file: ${response.status} ${response.statusText}`,
      );
    }

    // Get the response as an ArrayBuffer
    const fileData = await response.arrayBuffer();

    // Convert to Uint8Array which Rong.writeFile expects
    const fileBytes = new Uint8Array(fileData);

    // Write the file to disk
    await Rong.writeFile(outputPath, fileBytes);

    console.log(`Download complete! File saved to ${outputPath}`);

    // Get and display file size
    const fileInfo = await Rong.stat(outputPath);
    console.log(`File size: ${(fileInfo.size / 1024).toFixed(2)} KB`);
  } catch (error) {
    console.error(`Error: ${error.message}`);
  }
}

// Get command line arguments
const args = Rong.args;

console.log("args:", args);
if (args.length < 3) {
  console.log("Usage: rong run downloader.js <url> <output-filename>");
  Rong.exit(1);
}

const url = args[1];
const outputPath = args[2];

// Execute the download
downloadFile(url, outputPath);
