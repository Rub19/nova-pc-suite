import sharp from "sharp";
import { mkdir, writeFile } from "node:fs/promises";
await mkdir("src-tauri/icons", { recursive: true });
const source = "assets/nova.svg";
for (const size of [32, 128, 256, 512])
  await sharp(source)
    .resize(size, size)
    .png()
    .toFile(`src-tauri/icons/${size}x${size}.png`);
const png = await sharp(source).resize(256, 256).png().toBuffer();
const header = Buffer.alloc(22);
header.writeUInt16LE(1, 2);
header.writeUInt16LE(1, 4);
header.writeUInt16LE(1, 10);
header.writeUInt16LE(32, 12);
header.writeUInt32LE(png.length, 14);
header.writeUInt32LE(22, 18);
await writeFile("src-tauri/icons/icon.ico", Buffer.concat([header, png]));
