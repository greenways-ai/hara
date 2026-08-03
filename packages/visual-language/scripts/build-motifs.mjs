import { readdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const masters = join(root, "assets/motifs/master");
const destination = join(root, "assets/motifs/web");
const widths = [640, 1280, 1920, 2560, 4096];
const files = (await readdir(masters)).filter((file) => file.endsWith(".png"));

await Promise.all(files.flatMap((file) => widths.flatMap((width) => [
  sharp(join(masters, file)).resize(width, width, { fit: "cover", kernel: sharp.kernel.lanczos3 })
    .avif({ quality: 62, effort: 6 }).toFile(join(destination, file.replace(".png", `-${width}.avif`))),
  sharp(join(masters, file)).resize(width, width, { fit: "cover", kernel: sharp.kernel.lanczos3 })
    .webp({ quality: 82, effort: 6 }).toFile(join(destination, file.replace(".png", `-${width}.webp`)))
] )));

console.log(`Built ${files.length * widths.length * 2} responsive motif assets.`);
