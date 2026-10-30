// Generates `public/wallpapers/<id>_thumb.webp` (320×180) for every wallpaper
// image in public/wallpapers. Usage: `npm run thumbs` (add `--force` to redo all).
import { readdir, stat } from "node:fs/promises";
import path from "node:path";
import sharp from "sharp";

const dir = path.resolve(import.meta.dirname, "../public/wallpapers");
const force = process.argv.includes("--force");
const files = (await readdir(dir)).filter((f) => /\.(webp|jpe?g|png)$/i.test(f) && !/_thumb\./i.test(f));

for (const file of files) {
  const id = file.replace(/\.[^.]+$/, "");
  const out = path.join(dir, `${id}_thumb.webp`);
  if (!force && (await stat(out).catch(() => null))) continue;
  await sharp(path.join(dir, file)).resize(320, 180, { fit: "cover" }).webp({ quality: 82 }).toFile(out);
  console.log(`thumb: ${id}_thumb.webp`);
}
