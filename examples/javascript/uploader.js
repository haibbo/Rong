// Tiny File Uploader
// Usage: danity uploader.js <file-path> <server-url>

async function uploadFile(localPath, serverUrl) {
  try {
    console.log(`Uploading file: ${localPath}`);
    console.log(`To server: ${serverUrl}`);

    // 1. Read local file
    const fileContent = await Danity.readFile(localPath);
    const fileName = localPath.split("/").pop() || "file";

    // 2. Construct FormData
    const formData = new FormData();
    const fileBlob = new Blob([fileContent], {
      type: "application/octet-stream",
    });
    formData.append("file", fileBlob, fileName);
    formData.append("description", "Example file uploaded from Danity");

    // 3. Send POST request
    console.log("Starting upload...");
    const response = await fetch(serverUrl, {
      method: "POST",
      body: formData,
    });
    console.log("Upload complete");

    // 4. Handle response
    if (!response.ok) throw new Error(`Upload failed: ${response.statusText}`);
    const result = await response.json();
    console.log("✅ Upload successful:", result);
  } catch (error) {
    console.error("❌ Upload error:", error.message);
  }
}

// Get command line arguments
const args = Danity.args;

if (args.length < 2) {
  console.log("Usage: danity uploader.js <file-path> <server-url>");
  Danity.exit(1);
}

const localFile = args[0];
const serverUrl = args[1];

// Execute the upload
uploadFile(localFile, serverUrl);
