// order.js by Jigsy (https://github.com/Jigsy1) released under the Unlicense.
//
// Creates a circular tree based on an array of names. E.g. https://postimg.cc/n9BJGGN6
//
// This is actually an earlier revision of my code since the modified version is used on my website.
//
//
// Command line usage:
// --------------------
// ...:~$ node order[.js] item 1,item 2,item 3,...


const CHAR_PIPE = "¦"; // I've decided to add this incase you wish to change it from ¦ to | instead.

const args = process.argv.slice(2);
spaced = args.join(" ");
const joined = spaced.split(",");
let names = [];
if (args.length > 0) {
    for (let v = 0; v < joined.length; v++) {
        names.push(joined[v]);
    }
}
else {
    // Examples:
    //
    // let names = ["paimon"];
    // let names = ["klee", "nahida", "paimon", "columbina", "kamisato ayaka", "yoimiya", "sandrone", "ningguang", "citlali", "lisa", "lumine", "sangonomiya kokomi"];
    names = ["klee", "nahida", "paimon", "columbina", "kamisato ayaka", "yumemizuki mizuki", "sandrone", "ningguang", "citlali", "yoimiya", "lumine", "sangonomiya kokomi", "nefer"];
    // let names = ["alice", "klee"];
    // let names = ["yoimiya", "sangonomiya kokomi"];
}

if (names.length == 1) {
    // There's only one item, so print it and end the program there.
    console.log(names[0]);
    return;
}
let extra = names.length > 2 ? 1 : 0; // We need to add an extra item or print an extra item if there is more than two names in the array, otherwise the output formatting will look weird.
let name = "";
let string = "";
let sum = 0;
let temp = 0;
for (let a = 0; a < names.length; a++) {
    name = names[a];
    switch(true) {
        case (a == 0):
            if (names[a].length > names[(a + 1) % names.length].length) {
                string = "," + (extra == 1 ? "-" : "") + "-> " + name + " --,";
                console.log(string);
                temp = (names[a].length - names[(a + 1) % names.length].length);
                break;
            }
            sum = (names[(a + 1) % names.length].length - names[a].length);
            string = "," + (extra == 1 ? "-" : "") + "-> " + name + " " + "-".repeat(sum) + "--,";
            console.log(string);
            break;
        case (a == (names.length - 1)):
            if ((a % 2) == 0) {
                string = (extra == 1 ? CHAR_PIPE : "") + "`-> " + name + " --,";
                console.log(string);
                if (extra == 1) {
                    console.log("%s", "`" + "-".repeat((string.length - 2)) + "'");
                }
                break;
            }
            string = "`--" + (extra == 1 ? "-" : "") + " " + name + " <" + "-".repeat(temp) + "-'";
            console.log(string);
            break;
        case ((a % 2) == 0):
            if (names[a].length > names[(a + 1) % names.length].length) {
                string = (extra == 1 ? CHAR_PIPE : "") + "`-> " + name + " --,";
                console.log(string);
                temp = (names[a].length - names[(a + 1) % names.length].length);
                break;
            }
            sum = (names[(a + 1) % names.length].length - names[a].length);
            string = (extra == 1 ? CHAR_PIPE : "") + "`-> " + name + " " + "-".repeat(sum) + "--,";
            console.log(string);
            break;
        case ((a % 2) == 1):
            if (temp > 0) {
                string = (extra == 1 ? CHAR_PIPE : "") + ",-- " + name + " <" + "-".repeat(temp) + "-'";
                console.log(string);
                temp = 0;
                break;
            }
            string = (extra == 1 ? CHAR_PIPE : "") + ",-- " + name + " <-'";
            console.log(string);
            break;
        default:
            // noop
            break;
    }
}

// EOF
