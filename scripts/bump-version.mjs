import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const targets = {
  package: 'package.json',
  tauri: 'src-tauri/tauri.conf.json5',
  cargo: 'Cargo.toml',
  lock: 'Cargo.lock',
  frontend: 'frontend/src/shared/version.ts'
}

const current = JSON.parse(readFileSync(resolve(root, targets.package), 'utf8')).version
const input = process.argv[2]
if (!input) {
  console.error(`用法: node scripts/bump-version.mjs <patch|minor|major|版本号>\n当前版本: ${current}`)
  process.exit(1)
}

function nextVersion(from, kind) {
  const [major, minor, patch] = from.split('.').map(Number)
  if (kind === 'major') return `${major + 1}.0.0`
  if (kind === 'minor') return `${major}.${minor + 1}.0`
  if (kind === 'patch') return `${major}.${minor}.${patch + 1}`
  return kind
}

const version = nextVersion(current, input)
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`版本号格式无效: ${version}`)
  process.exit(1)
}

const read = (file) => readFileSync(resolve(root, file), 'utf8')
const write = (file, text) => writeFileSync(resolve(root, file), text)

const pkg = JSON.parse(read(targets.package))
pkg.version = version
write(targets.package, `${JSON.stringify(pkg, null, 2)}\n`)

write(
  targets.tauri,
  read(targets.tauri).replace(/("version":\s*")[^"]+(")/, `$1${version}$2`)
)

write(
  targets.cargo,
  read(targets.cargo).replace(
    /(\[workspace\.package\][\s\S]*?version = ")[^"]+(")/,
    `$1${version}$2`
  )
)

write(
  targets.lock,
  read(targets.lock).replace(
    /(name = "lightproxy(?:-core|-update)?"\nversion = ")[^"]+(")/g,
    `$1${version}$2`
  )
)

write(
  targets.frontend,
  read(targets.frontend).replace(
    /export const APP_VERSION = '[^']+'/,
    `export const APP_VERSION = '${version}'`
  )
)

console.log(`${current} -> ${version}`)
