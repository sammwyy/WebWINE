import fs from "node:fs/promises";
import path from "node:path";
import sharp from "sharp";
import { isIco, decodeIco } from "icojs";

const THEMES_DIR = path.resolve("public", "themes");

async function walk(dir) {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await walk(fullPath)));
    } else {
      files.push(fullPath);
    }
  }
  return files;
}

async function convertImage(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  const baseName = path.basename(filePath, ext);
  const dirName = path.dirname(filePath);
  const outPath = path.join(dirName, `${baseName}.webp`);

  if (ext === ".png" || ext === ".jpg" || ext === ".jpeg") {
    await sharp(filePath).webp({ quality: 90 }).toFile(outPath);
    console.log(`Converted ${path.relative(THEMES_DIR, filePath)} -> .webp`);
  } else if (ext === ".ico") {
    const nodeBuf = await fs.readFile(filePath);
    // Node.js Buffers share underlying memory slabs. We MUST extract a clean ArrayBuffer for icojs.
    const arrayBuffer = nodeBuf.buffer.slice(nodeBuf.byteOffset, nodeBuf.byteOffset + nodeBuf.byteLength);
    
    if (isIco(arrayBuffer)) {
      const images = await decodeIco(arrayBuffer);
      if (images.length > 0) {
        for (let i = 0; i < images.length; i++) {
          const img = images[i];
          const outPathVersion = path.join(dirName, `${baseName}_v${i + 1}.webp`);
          await sharp(Buffer.from(img.buffer)).webp({ quality: 90 }).toFile(outPathVersion);
          console.log(`Converted ${path.relative(THEMES_DIR, filePath)} [${img.width}x${img.height}] -> _v${i + 1}.webp`);
        }
      }
    } else {
      // It might just be a PNG renamed to .ico
      try {
        await sharp(nodeBuf).webp({ quality: 90 }).toFile(outPath);
        console.log(`Converted pseudo-ICO ${path.relative(THEMES_DIR, filePath)} -> .webp`);
      } catch (e) {
        console.error(`File ${filePath} has .ico extension but is not a valid ICO or image.`);
      }
    }
  }
}

async function main() {
  console.log("Building icons...");
  let files;
  try {
    files = await walk(THEMES_DIR);
  } catch (err) {
    console.error("Could not read themes directory:", err);
    return;
  }

  for (const file of files) {
    const ext = path.extname(file).toLowerCase();
    if ([".png", ".jpg", ".jpeg", ".ico"].includes(ext)) {
      try {
        await convertImage(file);
      } catch (err) {
        console.error(`Failed to convert ${file}:`, err);
      }
    }
  }
  console.log("Done building icons.");
}

main().catch(console.error);
