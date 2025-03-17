// Tiny File downloader
// Usage: danity downloader.js <url> <output-filename>

async function downloadFile(url, outputPath) {
  try {
    console.log(`Downloading from: ${url}`);
    console.log(`Saving to: ${outputPath}`);

    // Fetch the file
    console.log("startint fetch");
    const response = await fetch(url);
    console.log("fetch done");

    if (!response.ok) {
      throw new Error(
        `Failed to download file: ${response.status} ${response.statusText}`,
      );
    }

    // Get the response as an ArrayBuffer
    const fileData = await response.arrayBuffer();

    // Convert to Uint8Array which Danity.writeFile expects
    const fileBytes = new Uint8Array(fileData);

    // Write the file to disk
    await Danity.writeFile(outputPath, fileBytes);

    console.log(`Download complete! File saved to ${outputPath}`);

    // Get and display file size
    const fileInfo = await Danity.stat(outputPath);
    console.log(`File size: ${(fileInfo.size / 1024).toFixed(2)} KB`);
  } catch (error) {
    console.error(`Error: ${error.message}`);
  }
}

// Get command line arguments
const args = Danity.args;

console.log("args:", args);
if (args.length < 2) {
  console.log("Usage: danity downloader.js <url> <output-filename>");
  Danity.exit(1);
}

const url = args[0];
const outputPath = args[1];

// Execute the download
downloadFile(url, outputPath);
