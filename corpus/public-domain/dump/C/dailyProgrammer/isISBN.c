/*
 *
 * isISBN.c by Jigsy (https://github.com/Jigsy1) released under The Unlicense.
 *
 * For: https://old.reddit.com/r/dailyprogrammer/comments/2s7ezp/20150112_challenge_197_easy_isbn_validator/
 *
 * Compiled on Windows 8.1 using TCC available from: https://bellard.org/tcc/
 *
 * How to:
 * ----------
 * ...> tcc -c isISBN.c
 * ...> tcc -run isISBN.o
 *
 */

// I will point out that I'm not happy with this. It works (At least I hope so!), but I'm just not satisfied with it. -Jigsy

#include <ctype.h>
#include <stdio.h>
#include <string.h>

char * isISBN(char *thisISBN) {
	int thisLoop;
	int thisCount = 10;
	int thisSum = 0;
	for (thisLoop = 0; thisLoop <= strlen(thisISBN); thisLoop++) {
		if (isdigit(thisISBN[thisLoop])) {
			thisSum += (thisCount * (thisISBN[thisLoop] % 48));
			// Trying to work out how to convert this to a number was giving me a headache,
			// so just mod the ASCII number against zero's ASCII number.
			thisCount--;
			if (thisCount == 0) {
				break;
			}
			continue;
		}
		else {
			if (thisISBN[thisLoop] == 45) {
				// Aka. Hypen
				continue;
			}
			else if (thisISBN[thisLoop] == 88) {
				// Aka. X - It should be the last digit in an ISBN apparently if the value 10?
				thisSum += (thisCount * 10);
				thisCount--;
				break;
			}
			else {
				return "false";
			}
		}
	}
	if (thisCount > 0) {
		// Just incase.
		return "false";
	}
	if (thisSum % 11 > 0) {
		return "false";
	}
	return "true";
}

int main() {
	char string1[] = "0-7475-3269-9";
	printf("%s - %s\n", string1, isISBN(string1));
	char string2[] = "15-68811-11-X";
	printf("%s - %s\n", string2, isISBN(string2));
	// Own entries below with differing lengths, random hypen placements, etc.
	char string3[] = "5-1159-3441-7";
	printf("%s - %s\n", string3, isISBN(string3));
	char string4[] = "";
	printf("%s - %s\n", string4, isISBN(string4));
	char string5[] = "X";
	printf("%s - %s\n", string5, isISBN(string5));
	char string6[] = "0-747X-3269-X";
	printf("%s - %s\n", string6, isISBN(string6));
	char string7[] = "1234-5678-91011-12-13-14";
	printf("%s - %s\n", string7, isISBN(string7));
	char string8[] = "11";
	printf("%s - %s\n", string8, isISBN(string8));
	char string9[] = "4-789-004546";
	printf("%s - %s\n", string9, isISBN(string9));
	char string10[] = "0131103628"; // Hopefully someone will get this one. :>
	printf("%s - %s\n", string10, isISBN(string10));
}

// EOF