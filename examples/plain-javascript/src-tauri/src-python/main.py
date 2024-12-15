# auto loaded on startup

counter = 0

def greet_python(a):
  global counter
  counter = counter + 1
  print("received: " + str(a))
  s = "" if counter < 2 else "s"
  return f'Hello {a}! You\'ve been greeted {counter} time{s} from Python.'