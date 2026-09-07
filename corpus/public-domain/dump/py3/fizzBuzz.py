# fizzBuzz.py by Jigsy (https://github.com/Jigsy1) released under the Unlicense.

def main():
        for thisLoop in range(1, 101):
                # `-> Start at 1, end at 100.
                if thisLoop % 5 == 0 and thisLoop % 3 == 0:
                        print("Fizz Buzz", end=", ")
                        continue
                if thisLoop % 5 == 0:
                        print("Buzz", end=", ")
                        continue
                if thisLoop % 3 == 0:
                        print("Fizz", end=", ")
                        continue
                print(thisLoop, end=", ")

if __name__ == "__main__":
        main()

# EOF
