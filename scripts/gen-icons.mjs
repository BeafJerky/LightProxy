import { deflateSync } from 'node:zlib'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const iconsDir = resolve(root, 'src-tauri/icons')
mkdirSync(iconsDir, { recursive: true })

const crcTable = (() => {
  const table = new Uint32Array(256)
  for (let n = 0; n < 256; n += 1) {
    let c = n
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    table[n] = c >>> 0
  }
  return table
})()

function crc32(buffer) {
  let c = 0xffffffff
  for (const byte of buffer) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8)
  return (c ^ 0xffffffff) >>> 0
}

function chunk(type, data) {
  const length = Buffer.alloc(4)
  length.writeUInt32BE(data.length)
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data])
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(body))
  return Buffer.concat([length, body, crc])
}

function encodePng(width, height, rgba) {
  const header = Buffer.alloc(13)
  header.writeUInt32BE(width, 0)
  header.writeUInt32BE(height, 4)
  header[8] = 8
  header[9] = 6
  const raw = Buffer.alloc((width * 4 + 1) * height)
  for (let y = 0; y < height; y += 1) {
    raw[y * (width * 4 + 1)] = 0
    rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4)
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0))
  ])
}

function roundedRectSdf(px, py, cx, cy, hx, hy, radius) {
  const qx = Math.abs(px - cx) - (hx - radius)
  const qy = Math.abs(py - cy) - (hy - radius)
  const outside = Math.hypot(Math.max(qx, 0), Math.max(qy, 0))
  const inside = Math.min(Math.max(qx, qy), 0)
  return outside + inside - radius
}

function render(size) {
  const pixels = Buffer.alloc(size * size * 4)
  const bg = [0x25, 0x63, 0xeb]
  const fg = [0xff, 0xff, 0xff]
  const ringStroke = Math.max(size * 0.075, 1.2)
  const halfX = size * 0.19
  const halfY = size * 0.145
  const radius = size * 0.13
  const centers = [size * 0.355, size * 0.645]
  const cy = size * 0.5

  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const px = x + 0.5
      const py = y + 0.5
      const bgSdf = roundedRectSdf(px, py, size / 2, size / 2, size / 2, size / 2, size * 0.226)
      let color = null
      if (bgSdf <= 0) {
        color = bg
        const inGap = Math.abs(px - size / 2) < Math.max(size * 0.018, 0.8) &&
          Math.abs(py - cy) < halfY + ringStroke
        if (!inGap) {
          for (const cx of centers) {
            const sdf = roundedRectSdf(px, py, cx, cy, halfX, halfY, radius)
            if (Math.abs(sdf) <= ringStroke / 2) {
              color = fg
              break
            }
          }
        }
      }
      const offset = (y * size + x) * 4
      if (color) {
        pixels[offset] = color[0]
        pixels[offset + 1] = color[1]
        pixels[offset + 2] = color[2]
        pixels[offset + 3] = 255
      }
    }
  }
  return encodePng(size, size, pixels)
}

const pngBySize = new Map()
for (const size of [16, 20, 24, 32, 48, 64, 128, 256, 512]) {
  pngBySize.set(size, render(size))
}

for (const [size, name] of [
  [16, '16x16.png'],
  [32, '32x32.png'],
  [128, '128x128.png'],
  [256, '128x128@2x.png'],
  [512, 'icon.png']
]) {
  writeFileSync(resolve(iconsDir, name), pngBySize.get(size))
}

const icoSizes = [16, 20, 24, 32, 48, 64, 128, 256]
const header = Buffer.alloc(6)
header.writeUInt16LE(0, 0)
header.writeUInt16LE(1, 2)
header.writeUInt16LE(icoSizes.length, 4)
const entries = []
let offset = 6 + 16 * icoSizes.length
for (const size of icoSizes) {
  const data = pngBySize.get(size)
  const entry = Buffer.alloc(16)
  entry[0] = size === 256 ? 0 : size
  entry[1] = size === 256 ? 0 : size
  entry.writeUInt16LE(1, 4)
  entry.writeUInt16LE(32, 6)
  entry.writeUInt32LE(data.length, 8)
  entry.writeUInt32LE(offset, 12)
  entries.push(entry)
  offset += data.length
}
writeFileSync(
  resolve(iconsDir, 'icon.ico'),
  Buffer.concat([header, ...entries, ...icoSizes.map((size) => pngBySize.get(size))])
)

console.log(`icons written to ${iconsDir}`)
