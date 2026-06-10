import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

// Resolved at build time so the site always reflects the latest product
// release without manual edits. The Pages deploy workflow runs after each
// product release (and checks out with full history so tags are present).
const repoRoot = fileURLToPath(new URL("../../..", import.meta.url));

function fromGitTag(): string | null {
  try {
    const tag = execSync("git describe --tags --abbrev=0 --match 'v[0-9]*'", {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "ignore"],
    })
      .toString()
      .trim();
    return /^v\d+\.\d+\.\d+$/.test(tag) ? tag.slice(1) : null;
  } catch {
    return null;
  }
}

function fromChangelog(): string | null {
  try {
    const changelog = readFileSync(join(repoRoot, "CHANGELOG.md"), "utf8");
    const match = changelog.match(/^## \[(\d+\.\d+\.\d+)\]/m);
    return match ? match[1] : null;
  } catch {
    return null;
  }
}

export const SITE_VERSION = fromGitTag() ?? fromChangelog() ?? "0.0.0";

// "0.4" — used in the Cargo.toml dependency example, so partial releases
// (where the `rong` crate itself may not bump) stay accurate.
export const SITE_VERSION_MINOR = SITE_VERSION.split(".").slice(0, 2).join(".");
