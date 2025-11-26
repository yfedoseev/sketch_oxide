const { existsSync, readFileSync } = require('fs')
const { join } = require('path')

const { platform, arch } = process

let nativeBinding = null
let localFileExisted = false
let loadError = null

function isMusl() {
  // For Alpine and other musl-based distributions
  if (!process.versions.openssl) {
    return true
  }
  return readFileSync('/etc/ld-musl-x86_64.so.1', 'utf8', (err) => err && null) !== null
}

switch (platform) {
  case 'android':
    switch (arch) {
      case 'arm64':
        localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.android-arm64.node'))
        try {
          nativeBinding = require('./sketch_oxide_node.android-arm64.node')
        } catch (e) {
          loadError = e
        }
        break
      case 'arm':
        localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.android-arm.node'))
        try {
          nativeBinding = require('./sketch_oxide_node.android-arm.node')
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on android: ${arch}`)
    }
    break
  case 'win32':
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.win32-x64.node'))
        try {
          nativeBinding = require('./sketch_oxide_node.win32-x64.node')
        } catch (e) {
          loadError = e
        }
        break
      case 'ia32':
        localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.win32-ia32.node'))
        try {
          nativeBinding = require('./sketch_oxide_node.win32-ia32.node')
        } catch (e) {
          loadError = e
        }
        break
      case 'arm64':
        localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.win32-arm64.node'))
        try {
          nativeBinding = require('./sketch_oxide_node.win32-arm64.node')
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on win32: ${arch}`)
    }
    break
  case 'darwin':
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.darwin-x64.node'))
        try {
          nativeBinding = require('./sketch_oxide_node.darwin-x64.node')
        } catch (e) {
          loadError = e
        }
        break
      case 'arm64':
        localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.darwin-arm64.node'))
        try {
          nativeBinding = require('./sketch_oxide_node.darwin-arm64.node')
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on darwin: ${arch}`)
    }
    break
  case 'linux':
    switch (arch) {
      case 'x64':
        if (isMusl()) {
          localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.linux-x64-musl.node'))
          try {
            nativeBinding = require('./sketch_oxide_node.linux-x64-musl.node')
          } catch (e) {
            loadError = e
          }
        } else {
          localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.linux-x64-gnu.node'))
          try {
            nativeBinding = require('./sketch_oxide_node.linux-x64-gnu.node')
          } catch (e) {
            loadError = e
          }
        }
        break
      case 'arm64':
        if (isMusl()) {
          localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.linux-arm64-musl.node'))
          try {
            nativeBinding = require('./sketch_oxide_node.linux-arm64-musl.node')
          } catch (e) {
            loadError = e
          }
        } else {
          localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.linux-arm64-gnu.node'))
          try {
            nativeBinding = require('./sketch_oxide_node.linux-arm64-gnu.node')
          } catch (e) {
            loadError = e
          }
        }
        break
      case 'arm':
        localFileExisted = existsSync(join(__dirname, 'sketch_oxide_node.linux-arm-gnueabihf.node'))
        try {
          nativeBinding = require('./sketch_oxide_node.linux-arm-gnueabihf.node')
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on linux: ${arch}`)
    }
    break
  default:
    throw new Error(`Unsupported platform: ${platform}`)
}

if (!nativeBinding) {
  if (loadError) {
    throw loadError
  }
  throw new Error(`Failed to load native binding. Platform: ${platform}, arch: ${arch}, localFileExisted: ${localFileExisted}`)
}

module.exports = nativeBinding
