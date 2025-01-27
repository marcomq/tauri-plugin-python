# File auto loaded on startup
_tauri_plugin_functions = ["greet_python"] # make these functions callable from UI

counter = 0

def greet_python(input):
  global counter
  counter = counter + 1
  print("received: " + str(input))
  s = "" if counter < 2 else "s"
  return f'Hello {input}! You\'ve been greeted {counter} time{s} from Python.'