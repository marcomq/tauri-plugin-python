import SwiftRs
import Tauri
import UIKit
import WebKit

class PingArgs: Decodable {
  let value: String?
}

class ExamplePlugin: Plugin {
}

@_cdecl("init_plugin_python")
func initPlugin() -> Plugin {
  return ExamplePlugin()
}
