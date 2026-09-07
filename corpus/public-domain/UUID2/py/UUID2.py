# A python 3 script to generate an alpha numeric "UUID."

import random
import string
import tkinter

from tkinter import *

uuid2 = Tk()
uuid2.title("UUID2")
uuid2.resizable(height=False, width=False)

def makeID():
  str = ""
  out = ""
  str = "".join(random.SystemRandom().choice(string.ascii_uppercase + string.digits) for _ in range(32))
  out = f"{str[0:8]}-{str[8:12]}-{str[12:16]}-{str[16:20]}-{str[20:]}"
  if braces.get() == 1:
    out = "{" + out + "}"
  print(out)
  uuid2.clipboard_clear()
  uuid2.clipboard_append(out)
  uuid2.update()

braces = IntVar(value=1)

braceBox = Checkbutton(uuid2, text="{Use Braces}", variable=braces, onvalue=1, offvalue=0)
genButton = Button(uuid2, text="Generate", command=makeID)

braceBox.pack(side = RIGHT)
genButton.pack(side = LEFT)

uuid2.mainloop()

# EOF
