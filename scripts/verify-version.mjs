import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const read = (file) => readFileSync(resolve(root, file), 'utf8')

function match(source, pattern, label) {
  const result = source.match(pattern)
  if (!result) throw new Error(`无法从 ${label} 读取版本号`)
  return result[1]
}

const versions = {
  'package.json': JSON.parse(read('package.json')).version,
  'tauri.conf.json5': match(
    read('src-tauri/tauri.conf.json5'),
    /"version":\s*"([^"]+)"/,
    'tauri.conf.json5'
  ),
  'Cargo.toml': match(
    read('Cargo.toml'),
    /\[workspace\.package\][\s\S]*?version = "([^"]+)"/,
    'Cargo.toml'
  ),
  'Cargo.lock (lightproxy)': match(
    read('Cargo.lock'),
    /name = "lightproxy"\nversion = "([^"]+)"/,
    'Cargo.lock'
  ),
  'Cargo.lock (lightproxy-core)': match(
    read('Cargo.lock'),
    /name = "lightproxy-core"\nversion = "([^"]+)"/,
    'Cargo.lock'
  ),
  'frontend/src/shared/version.ts': match(
    read('frontend/src/shared/version.ts'),
    /APP_VERSION = '([^']+)'/,
    'version.ts'
  )
}

const unique = new Set(Object.values(versions))
for (const [file, version] of Object.entries(versions)) {
  console.log(`${version === [...unique][0] ? 'ok  ' : 'FAIL'}  ${version}  ${file}`)
}
if (unique.size !== 1) {
  console.error(`版本号不一致: ${[...unique].join(', ')}`)
  process.exit(1)
}
console.log(`版本号一致: ${[...unique][0]}`)
