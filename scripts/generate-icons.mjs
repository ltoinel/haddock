import sharp from "sharp";
import { readFileSync, writeFileSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const iconsDir = join(__dirname, "..", "src-tauri", "icons");
const svgPath = join(iconsDir, "icon.svg");
const svgBuffer = readFileSync(svgPath);

const sizes = [
  { name: "32x32.png", size: 32 },
  { name: "128x128.png", size: 128 },
  { name: "128x128@2x.png", size: 256 },
  { name: "icon.png", size: 512 },
  // Windows Store icons
  { name: "Square30x30Logo.png", size: 30 },
  { name: "Square44x44Logo.png", size: 44 },
  { name: "Square71x71Logo.png", size: 71 },
  { name: "Square89x89Logo.png", size: 89 },
  { name: "Square107x107Logo.png", size: 107 },
  { name: "Square142x142Logo.png", size: 142 },
  { name: "Square150x150Logo.png", size: 150 },
  { name: "Square284x284Logo.png", size: 284 },
  { name: "Square310x310Logo.png", size: 310 },
  { name: "StoreLogo.png", size: 50 },
];

console.log("Generating PNG icons from SVG...");

for (const { name, size } of sizes) {
  const outPath = join(iconsDir, name);
  await sharp(svgBuffer, { density: 300 })
    .resize(size, size)
    .png()
    .toFile(outPath);
  console.log(`  ${name} (${size}x${size})`);
}

// Generate ICO (contains 16, 32, 48, 256)
const icoSizes = [16, 32, 48, 256];
const icoBuffers = [];
for (const size of icoSizes) {
  const buf = await sharp(svgBuffer, { density: 300 })
    .resize(size, size)
    .png()
    .toBuffer();
  icoBuffers.push({ size, buf });
}

// Build ICO file manually
function buildIco(images) {
  const numImages = images.length;
  const headerSize = 6;
  const dirEntrySize = 16;
  const dirSize = dirEntrySize * numImages;
  let offset = headerSize + dirSize;

  // ICO header
  const header = Buffer.alloc(headerSize);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // type: ICO
  header.writeUInt16LE(numImages, 4);

  const dirEntries = [];
  const dataBuffers = [];

  for (const { size, buf } of images) {
    const entry = Buffer.alloc(dirEntrySize);
    entry.writeUInt8(size >= 256 ? 0 : size, 0); // width (0 = 256)
    entry.writeUInt8(size >= 256 ? 0 : size, 1); // height
    entry.writeUInt8(0, 2); // color palette
    entry.writeUInt8(0, 3); // reserved
    entry.writeUInt16LE(1, 4); // color planes
    entry.writeUInt16LE(32, 6); // bits per pixel
    entry.writeUInt32LE(buf.length, 8); // data size
    entry.writeUInt32LE(offset, 12); // data offset
    dirEntries.push(entry);
    dataBuffers.push(buf);
    offset += buf.length;
  }

  return Buffer.concat([header, ...dirEntries, ...dataBuffers]);
}

const icoBuffer = buildIco(icoBuffers);
writeFileSync(join(iconsDir, "icon.ico"), icoBuffer);
console.log("  icon.ico (16, 32, 48, 256)");

// Generate ICNS placeholder (macOS) - just copy the 512 PNG
// Real ICNS would need a proper encoder, but Tauri handles PNG fallback
const png512 = await sharp(svgBuffer, { density: 300 })
  .resize(512, 512)
  .png()
  .toBuffer();
writeFileSync(join(iconsDir, "icon.icns"), png512);
console.log("  icon.icns (512x512 PNG placeholder)");

console.log("\nDone! All icons generated.");
