/*
 * fizzBuzz.c by Jigsy (https://github.com/Jigsy1) released under the Unlicense.
 *
 * Compiled on Windows 8.1 using TCC available from: https://bellard.org/tcc/
 *
 * How to:
 * ----------
 * ...> tcc -c fizzBuzz.c
 * ...> tcc -run fizzBuzz.o
 *
 */

#include <stdio.h>

#define BUZZ "Buzz"
#define FIZZ "Fizz"
#define FIZZ_BUZZ "Fizz Buzz"

int main(void) {
	int thisLoop;
	for (thisLoop = 1; thisLoop <= 100; thisLoop++) {
		if (thisLoop % 5 == 0 && thisLoop % 3 == 0) {
			printf("%s, ", FIZZ_BUZZ);
			continue;
		}
		if (thisLoop % 5 == 0) {
			printf("%s, ", BUZZ);
			continue;
		}
		if (thisLoop % 3 == 0) {
			printf("%s, ", FIZZ);
			continue;
		}
		printf("%d, ", thisLoop);
		// `-> The output symmetry is quite interesting.
	}
	return 0;
}

// EOF