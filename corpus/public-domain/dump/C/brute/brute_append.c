/*
 * brute.c (Rev.2) - APPEND VERSION - by Jigsy (https://github.com/Jigsy1) released under the Unlicense.
 *
 * Just a demo of looping through letter(s). A->B->C->...->AA->BA->...->...->XZZZ->YZZZ->ZZZZ->...
 *
 * Compiled on Windows 8.1 using TCC available from: https://bellard.org/tcc/
 *
 * How to:
 * ----------
 * ...> tcc -c brute.c
 * ...> tcc -run brute.o
 *
 */

#include <stdio.h>
#include <string.h>

/* START OF PROTOTYPES */

void rotate(int index);

/* END OF PROTOTYPES */

const char CHAR_MAP[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
#define MAPSIZE (sizeof(CHAR_MAP)/sizeof(CHAR_MAP[0]))
char STRING[MAPSIZE+1] = "A";		// +1 should account for \0.
const char END_STRING[] = "ZZZZ";	// Make sure this is the last character of CHAR_MAP * strlen(STRING)!

void rotate(int index) {
	char *tempChar;
	int tempIndex;
	tempChar = strchr(CHAR_MAP, STRING[index]);
	tempIndex = (int) (tempChar - CHAR_MAP);
	// `-> Find the position of the current character in CHAR_MAP.
	STRING[index] = CHAR_MAP[(tempIndex + 1) % strlen(CHAR_MAP)];
	if (CHAR_MAP[tempIndex] == CHAR_MAP[(strlen(CHAR_MAP) - 1)]) {
		STRING[index] = CHAR_MAP[0];
		if ((index + 1) == strlen(STRING)) {
			STRING[(index + 1)] = CHAR_MAP[0];
			return;
		}
		rotate((index + 1));
	}
}
int main(void) {
	int index = 0;
	unsigned long int thisChar = 0;
	for (;;) {
		printf("%s\n", STRING);
		if (strncmp(STRING, END_STRING, strlen(END_STRING)) == 0) {
			// `-> STRING matches END_STRING so end here.
			break;
		}
		if (STRING[index] == CHAR_MAP[(strlen(CHAR_MAP) - 1)]) {
			if ((index + 1) == strlen(STRING)) {
				STRING[(index + 1)] = CHAR_MAP[0];
			}
			else {
				rotate((index + 1));
			}
		}
		thisChar += 1;
		STRING[index] = CHAR_MAP[thisChar % strlen(CHAR_MAP)];
	}
	printf("End.\n");
	return 0;
}

// EOF