/*
 * lettersum.c by Jigsy (https://github.com/Jigsy1) released under The Unlicense.
 *
 * For: https://old.reddit.com/r/dailyprogrammer/comments/onfehl/20210719_challenge_399_easy_letter_value_sum/
 *
 * Compiled on Windows 8.1 using TCC available from: https://bellard.org/tcc/
 *
 * How to:
 * ----------
 * ...> tcc -c lettersum.c
 * ...> tcc -run lettersum.o
 *
 */

#include <ctype.h>
#include <stdio.h>
#include <string.h>

/* START OF PROTOTYPES */

int getAlphaNumber(char thisLetter);
int lettersum(char *thisString);

/* END OF PROTOTYPES */

int getAlphaNumber(char thisLetter) {
	if (!isalpha(thisLetter)) {
		return 0;
	}
	if (isupper(thisLetter)) {
		thisLetter = tolower(thisLetter);
	}
	const char alphabet[] = "abcdefghijklmnopqrstuvwxyz";
	int thisLoop, thisPos = 0;
	for (thisLoop = 0; thisLoop <= strlen(alphabet); thisLoop++) {
		if (thisLetter == alphabet[thisLoop]) {
			return (thisLoop + 1);
		}
	}
	return thisPos;
}
int lettersum(char *thisString) {
	int thisLoop, thisSum = 0;
	for (thisLoop = 0; thisLoop <= strlen(thisString); thisLoop++) {
		thisSum += getAlphaNumber(thisString[thisLoop]);
	}
	return thisSum;
}

int main(void) {
	char string1[] = "";
	printf("%s = %d\n", string1, lettersum(string1));	// -> Should return: 0
	char string2[] = "a";
	printf("%s = %d\n", string2, lettersum(string2));	// -> Should return: 1
	char string3[] = "z";
	printf("%s = %d\n", string3, lettersum(string3));	// -> Should return: 26
	char string4[] = "cab";
	printf("%s = %d\n", string4, lettersum(string4));	// -> Should return: 6
	char string5[] = "excellent";
	printf("%s = %d\n", string5, lettersum(string5));	// -> Should return: 100
	char string6[] = "microspectrophotometries";
	printf("%s = %d\n", string6, lettersum(string6));	// -> Should return: 317

	// ,-> My own entries below:
	char string7[] = "antidisestablishmentarianism";	// -> Should return: ?
	printf("%s = %d\n", string7, lettersum(string7));
	char string8[] = "pneumonoultramicroscopicsilicovolcanoconiosis";	// -> Should return: ?
	printf("%s = %d\n", string8, lettersum(string8));
	char string9[] = "floccinaucinihilipilification";	// -> Should return: ?
	printf("%s = %d\n", string9, lettersum(string9));
	char string10[] = "xylophone";
	printf("%s = %d\n", string10, lettersum(string10));	// -> Should return: ?

	return 0;
}

// EOF