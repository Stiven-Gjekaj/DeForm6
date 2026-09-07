# brute.py (Rev.2) - APPEND VERSION - by Jigsy (https://github.com/Jigsy1) released under the Unlicense.
#
# Just a demo of looping through letter(s). A->B->C->...->AA->BA->CA->...->...->XZZZ->YZZZ->ZZZZ->...
# ...and cracking a hash.

import hashlib
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
# THIS_HASH = "c5f8c454ccd378437a4599aed72e1518"
# >-> If you want the insanely long version, comment out CHAR_MAP, END_STRING and THIS_HASH below and uncomment the three above.
CHAR_MAP = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
END_STRING = list(CHAR_MAP[-1] * 3)

THIS_HASH = "91164770fc34b877cc56ccaf61d6eee8"

STRING = list(CHAR_MAP[0] * 1)
# `-> Starting from the first character in the array: A

def main():
    print("Checking: {}".format(THIS_HASH))
    print("{} character set ^ {} character(s) long = ~{} loop(s).\n".format(len(CHAR_MAP), len(END_STRING), pow(len(CHAR_MAP), len(END_STRING))))
    index = 0
    thisChar = 0
    while 1:
        # print("".join(STRING))
        # `-> You can speed up the entire program by commenting this line out.
        encodedString = "".join(STRING).encode("UTF-8")
        # `-> String needs to be encoded to use hashlib.
        hashOutput = hashlib.md5(encodedString).hexdigest()
        # `-> If you change the hash to sha1, sha256, etc, just remember to change this from md5.
        if hashOutput == THIS_HASH:
            print("Match after {} loop(s): {} -> {}".format(thisChar, "".join(STRING), hashOutput))
            break
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
        # `-> Uncomment this if you want a one second gap between checks.
    print("End")

if __name__ == "__main__":
    main()

# EOF
