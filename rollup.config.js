import { readFileSync } from 'fs'
import { join } from 'path'
import { cwd } from 'process'

import { nodeResolve } from '@rollup/plugin-node-resolve'
import typescript from '@rollup/plugin-typescript'

const pkg = JSON.parse(readFileSync(join(cwd(), 'package.json'), 'utf8'))


const pluginJsName = 'python' // window.__TAURI__.python
const iifeVarName = '__TAURI_PYTHON_PLUGIN_API__'

export default [{
  input: 'guest-js/index.ts',
  output: [
    {
      file: pkg.exports.import,
      format: 'esm'
    },
    {
      file: pkg.exports.require,
      format: 'cjs'
    }
  ],
  plugins: [
    typescript({
      declaration: true,
      declarationDir: `./${pkg.exports.import.split('/')[0]}`
    })
  ],
  external: [
    /^@tauri-apps\/api/,
    ...Object.keys(pkg.dependencies || {}),
    ...Object.keys(pkg.peerDependencies || {})
  ]
},
  {
    input: 'guest-js/index.ts',
    output: [
      {
        name: iifeVarName,
        file: pkg.exports.html,
        format: 'iife',
        banner: "if ('__TAURI__' in window) {",
        // the last `}` closes the if in the banner
        footer: `Object.defineProperty(window.__TAURI__, '${pluginJsName}', { value: ${iifeVarName} }) }`

      }
    ],
    plugins: [typescript(), nodeResolve()],
  }
]
