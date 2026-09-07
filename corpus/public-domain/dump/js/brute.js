// brute.js - APPEND VERSION - by Jigsy (https://github.com/Jigsy1) released under the Unlicense.
//
// Just a demo of looping through letter(s). A->B->C->...->Z->AA->BA->CA->...->...->XZZ->YZZ->ZZZ->...
//
// Ported from Python3.

function rotate(index) {
    let char = CHAR_MAP.indexOf(STRING[index]);
    STRING[index] = CHAR_MAP.charAt((char + 1) % CHAR_MAP.length);
    if (CHAR_MAP.charAt(char) === CHAR_MAP.charAt(CHAR_MAP.length - 1)) {
        STRING[index] = CHAR_MAP.charAt(0);
        if ((index + 1) == STRING.length) {
            STRING.push(CHAR_MAP.charAt(0));
            return;
        }
        rotate((index + 1));
    }
}

// const CHAR_MAP = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
// const END_STRING = Array(CHAR_MAP.length).fill(CHAR_MAP.chartAt(CHAR_MAP.length - 1));
const CHAR_MAP = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
var STRING = Array(1).fill(CHAR_MAP.charAt(0));
const END_STRING = Array(3).fill(CHAR_MAP.charAt(CHAR_MAP.length - 1));
// `-> Change (3) to a different number if you want it to terminate at a longer string length. E.g. (3) = A->ZZZ, (7) = A->ZZZZZZZ, (10) = A->ZZZZZZZZZZ, etc.

let index = 0;
let char = 0;
while (true) {
    console.log(STRING.join(""));
    if (STRING.join("") === END_STRING.join("")) {
        break;
    }
    if (STRING[index] === CHAR_MAP.charAt(CHAR_MAP.length - 1)) {
        if ((index + 1) == STRING.length) {
            STRING.push(CHAR_MAP.charAt(0));
        }
        else {
            rotate((index + 1));
        }
    }
    char += 1;
    STRING[index] = CHAR_MAP.charAt(char % CHAR_MAP.length);
}

console.log("End");

// EOF
