# brute.py (Rev.2) - APPEND VERSION - by Jigsy (https://github.com/Jigsy1) released under the Unlicense.
#
# Just a demo of looping through letter(s). A->B->C->...->AA->BA->CA->...->...->XZZZ->YZZZ->ZZZZ->...

import time

def rotate(index):
    """Rotate characters by one."""
    char = CHAR_MAP.rfind(STRING[index])
    STRING[index] = CHAR_MAP[(char + 1) % len(CHAR_MAP)]
    if CHAR_MAP[char] == CHAR_MAP[-1]:
        STRING[index] = CHAR_MAP[0]
        if (index + 1) == len(STRING):
            STRING.append(CHAR_MAP[0])
            return
        rotate((index + 1))

# CHAR_MAP = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
# END_STRING = list(CHAR_MAP[-1] * len(CHAR_MAP))
# >-> If you want the insanely long version, comment out CHAR_MAP and END_STRING below and uncomment the two above.
CHAR_MAP = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
STRING = list(CHAR_MAP[0] * 1)
END_STRING = list(CHAR_MAP[-1] * 3)

def main():
    index = 0
    thisChar = 0
    while 1:
        print("".join(STRING))
        if "".join(STRING) == "".join(END_STRING):
            break
        if STRING[index] == CHAR_MAP[-1]:
            if (index + 1) == len(STRING):
                STRING.append(CHAR_MAP[0])
            else:
                rotate((index + 1))
        thisChar += 1
        STRING[index] = CHAR_MAP[thisChar % len(CHAR_MAP)]
        # time.sleep(1)
    print("End")

if __name__ == "__main__":
    main()

# EOF
