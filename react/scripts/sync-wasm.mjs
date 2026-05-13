import { access, copyFile, mkdir } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const projectRoot = path.resolve(__dirname, '..')
const sourceDir = path.resolve(projectRoot, '..', 'rust_version', 'pkg')
const targetDir = path.resolve(projectRoot, 'src', 'wasm')

const files = ['wordle_stuff.js', 'wordle_stuff_bg.wasm', 'wordle_stuff.d.ts', 'wordle_stuff_bg.wasm.d.ts']

await mkdir(targetDir, { recursive: true })

for (const file of files) {
  const source = path.join(sourceDir, file)
  const destination = path.join(targetDir, file)

  try {
    await access(source)
    await copyFile(source, destination)
  } catch {
    await access(destination)
  }
}
