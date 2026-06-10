import { defineCollection } from "astro:content";
import { glob } from "astro/loaders";

// Module API pages are generated straight from the repo's markdown at build
// time — the same files that ship in @rongjs/rong-skill. No copies to maintain.

// docs/api/<name>.md — public JS API reference per module (id: "timer", ...)
const apiDocs = defineCollection({
  loader: glob({ pattern: "*.md", base: "../docs/api" }),
});

// modules/<crate>/README.md — fallback for modules without a docs/api entry
// (currently only rong_cron). id: "rong_cron/readme", ...
const moduleReadmes = defineCollection({
  loader: glob({ pattern: "*/README.md", base: "../modules" }),
});

export const collections = { apiDocs, moduleReadmes };
