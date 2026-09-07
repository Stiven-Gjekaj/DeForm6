# brute.py by Jigsy (https://github.com/Jigsy1) released under the Unlicense.
#
# Just a demo of looping through letter(s). aaaa->baaa->caaa->...->yzzz->zzzz


# import sys


# define

# CHAR_MAP = "abcdefghijklmnopqrstuvwxyz"
# STR_LEN = 3

CHAR_MAP = "abcd"
STR_LEN = len(CHAR_MAP)


# Core

# sys.setrecursionlimit(20000)
# ¦-> I would avoid increasing this beyond 20,000 if you use 26*26*26. (Which is 17,576 anyway.)
# `-> And 4^4 is about 256, so that should be enough for this demo.

breakFlag = []
# `-> DO NOT ADD ANYTHING TO THIS!
string = list(CHAR_MAP[0] * STR_LEN)

def rotateChar(stringIndex, mapIndex, again):
        if string[stringIndex] == CHAR_MAP[(len(CHAR_MAP) - 1)]:
                mapIndex = 0
                newIndex = (stringIndex + 1)
                string[stringIndex] = CHAR_MAP[mapIndex]
                if newIndex < len(string):
                        tempIndex = CHAR_MAP.rfind(string[newIndex])
                        rotateChar(newIndex, (tempIndex + 1), 0)
                else:
                        # ,-> As I can't find any other way to end the recursion loop as "return" doesn't seem to be
                        # ¦->  doing anything, and "break" doesn't work outside of a loop... and Python doesn't have "goto"
                        # ¦->  (Seriously!? goto is useful in some cases! (Like this one! >_<))
                        # ¦->  we'll just set a hacky flag and stop the loop that way.
                        breakFlag.append('1')
                        print("\nEnd.")
        string[stringIndex] = CHAR_MAP[mapIndex]
        if again > 0:
                if len(breakFlag) == 0:
                        print(string)
                        rotateChar(stringIndex, (mapIndex + 1), 1)

def main():
        rotateChar(0, 0, 1)
        # `-> Start at 0, from the 1st character ('a') which is 0 in arrayspeak, and rotate again.

if __name__ == "__main__":
	main()


# EOF
