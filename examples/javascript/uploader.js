// Tiny File Uploader
// Usage: rong run uploader.js <file-path> <server-url>

async function uploadFile(localPath, serverUrl) {
  try {
    console.log(`Uploading file: ${localPath}`);
    console.log(`To server: ${serverUrl}`);

    // 1. Read local file
    const fileContent = await Rong.readFile(localPath);
    const fileName = localPath.split("/").pop() || "file";

    // 2. Construct FormData
    const formData = new FormData();
    const fileBlob = new Blob([fileContent], {
      type: "application/octet-stream",
    });
    formData.append("file", fileBlob, fileName);
    formData.append("description", "Example file uploaded from Rong");

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
const args = Rong.args;

if (args.length < 3) {
  console.log("Usage: rong run uploader.js <file-path> <server-url>");
  Rong.exit(1);
}

const localFile = args[1];
const serverUrl = args[2];

// Execute the upload
uploadFile(localFile, serverUrl);
