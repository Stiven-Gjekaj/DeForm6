/*
 * twirl.c by Jigsy (https://github.com/Jigsy1) released under the Unlicense.
 *
 * Compiled on Windows 8.1 using TCC available from: https://bellard.org/tcc/
 *
 * How to:
 * ----------
 * ...> tcc -c twirl.c
 * ...> tcc -run twirl.o
 *
 *
 * Note: You will need to do CTRL+C to terminate the program in the Command Prompt.
 *
 */

#include <stdio.h>
#include <winapi/windows.h>

#define SLEEP_TIME 500
// `-> 500 is equal to 0.5s on Windows.

int main(void) {
	for (;;) {
		// `-> Infinite loop.
		printf("\b|");
		Sleep(SLEEP_TIME);
		printf("\b/");
		Sleep(SLEEP_TIME);
		printf("\b-");
		Sleep(SLEEP_TIME);
		printf("\b\\");
		Sleep(SLEEP_TIME);
	}
	return 0;
}

// EOF
