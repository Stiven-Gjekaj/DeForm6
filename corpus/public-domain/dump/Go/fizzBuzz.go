// fizzBuzz.go by Jigsy (https://github.com/Jigsy1) released under the Unlicense.

package main

import "fmt"

func main() {
	for loop := 1; loop < 100; loop++ {
		if loop % 5 == 0 && loop % 3 == 0 {
			fmt.Print("Fizz Buzz, ")
			continue
		}
		if loop % 5 == 0 {
			fmt.Print("Buzz, ")
			continue
		}
		if loop % 3 == 0 {
			fmt.Print("Fizz, ")
			continue
		}
		fmt.Print(loop, ", ")
	}
}

// EOF