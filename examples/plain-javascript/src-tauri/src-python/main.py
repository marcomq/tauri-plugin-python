# auto loaded on startup

a = 0
def testing(c, b):
  global a
  a = a + 1
  print(c)
  print(b)
  print("hello world: " + str(c))
  return str(a)+"testResult"+str(b)