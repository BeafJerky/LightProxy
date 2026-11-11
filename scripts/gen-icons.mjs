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

function coverage(distancePx) {
  return Math.min(Math.max(0.5 - distancePx, 0), 1)
}

function segmentDistance(px, py, ax, ay, bx, by) {
  const vx = bx - ax
  const vy = by - ay
  const lengthSq = vx * vx + vy * vy
  const t = lengthSq === 0 ? 0 : Math.min(Math.max(((px - ax) * vx + (py - ay) * vy) / lengthSq, 0), 1)
  return Math.hypot(px - (ax + t * vx), py - (ay + t * vy))
}

// Lever geometry in a 32-unit design space, matching frontend/public/favicon.svg.
function render(size) {
  const pixels = Buffer.alloc(size * size * 4)
  const bg = [0x25, 0x63, 0xeb]
  const fg = [0xff, 0xff, 0xff]
  const unit = size / 32
  const stroke = 4.25
  const baseY = 22.15
  const baseLeft = 7
  const baseRight = 25.25
  const armX = 16.125
  const knobX = 21.875
  const knobY = 12.1875

  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const px = (x + 0.5) / unit
      const py = (y + 0.5) / unit
      const tileCoverage = coverage(roundedRectSdf(px, py, 16, 16, 16, 16, 8) * unit)
      if (tileCoverage <= 0) continue
      const markDistance = Math.min(
        segmentDistance(px, py, baseLeft, baseY, baseRight, baseY) - stroke / 2,
        segmentDistance(px, py, armX, baseY, knobX, knobY) - stroke / 2,
        Math.hypot(px - knobX, py - knobY) - stroke
      ) * unit
      const markCoverage = coverage(markDistance)
      const offset = (y * size + x) * 4
      pixels[offset] = Math.round(fg[0] * markCoverage + bg[0] * (1 - markCoverage))
      pixels[offset + 1] = Math.round(fg[1] * markCoverage + bg[1] * (1 - markCoverage))
      pixels[offset + 2] = Math.round(fg[2] * markCoverage + bg[2] * (1 - markCoverage))
      pixels[offset + 3] = Math.round(tileCoverage * 255)
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
