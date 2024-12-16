# auto loaded on startup

counter = 0

def greet_python(input):
  global counter
  counter = counter + 1
  print("received: " + str(input))
  s = "" if counter < 2 else "s"
  return f'Hello {input}! You\'ve been greeted {counter} time{s} from Python.'