// map.js by Jigsy (https://github.com/Jigsy1) released under the Unlicense.
//
// If you've ever done /map on a large IRC server, you'll understand what this does. If you don't, see: https://i.imgur.com/2FNaI4A.png
//
// As far as I can tell, this works correctly. (Keywords: As far as I can tell)

const CHAR_PIPE = "¦";
const CHAR_STUB = "-";
const CHAR_TAIL = "`";
const END_OF_MAP = "End of /MAP";

const servers = {
        // ,-> "child": "parent"[,] - The , isn't required for the final entry hence why it's in [].
        "angelicarogner.localhost": "towaherschel.localhost",
        "titarussell.localhost": "angelicarogner.localhost",
        "estellebright.localhost": "titarussell.localhost",
        "rennebright.localhost": "estellebright.localhost",
        "scherazardharvey.localhost": "estellebright.localhost",
        "klaudiavonauslese.localhost": "scherazardharvey.localhost",
        "saravalestein.localhost": "towaherschel.localhost",
        "milliumorion.localhost": "saravalestein.localhost",
        "altinaorion.localhost": "milliumorion.localhost",
        "fieclaussell.localhost": "saravalestein.localhost",
        "laurasarseid.localhost": "saravalestein.localhost",
        "rosine.localhost": "towaherschel.localhost",
        "asaherschel.localhost": "towaherschel.localhost",
        "priscillareisearnor.localhost": "towaherschel.localhost",
        "clairerieveldt.localhost": "priscillareisearnor.localhost",
        "alfinreisearnor.localhost": "priscillareisearnor.localhost",
        "eliseschwarzer.localhost": "alfinreisearnor.localhost",
        "luciaschwarzer.localhost": "eliseschwarzer.localhost",
        "roseliamillstein.localhost": "towaherschel.localhost",
        "vitaclotilde.localhost": "roseliamillstein.localhost",
        "emmamillstein.localhost": "roseliamillstein.localhost",
        "celinemillstein.localhost": "emmamillstein.localhost",
        "jessicaschleiden.localhost": "towaherschel.localhost",
        "noelseeker.localhost": "towaherschel.localhost",
        "sonyabaelz.localhost": "noelseeker.localhost",
        "mireille.localhost": "sonyabaelz.localhost",
        "franseeker.localhost": "noelseeker.localhost",
        "fionacraig.localhost": "towaherschel.localhost",
        "irinareinford.localhost": "towaherschel.localhost",
        "alisareinford.localhost": "irinareinford.localhost",
        "josettecapua.localhost": "alisareinford.localhost",
        "ferrisflorald.localhost": "alisareinford.localhost",
        "margaritadresden.localhost": "ferrisflorald.localhost",
        "sharonkreuger.localhost": "irinareinford.localhost",
        "arianrhod.localhost": "sharonkreuger.localhost",
        "ennea.localhost": "arianrhod.localhost",
        "duvalie.localhost": "arianrhod.localhost",
        "ines.localhost": "arianrhod.localhost",
        "shirleyorlando.localhost": "sharonkreuger.localhost",
        "aurelialeguin.localhost": "towaherschel.localhost",
        "maryaltheim.localhost": "ines.localhost",
        "junacrawford.localhost": "aurelialeguin.localhost",
        "musseegret.localhost": "towaherschel.localhost",
        "tioplato.localhost": "towaherschel.localhost",
        "ilyaplatiere.localhost": "tioplato.localhost",
        "rixiamao.localhost": "ilyaplatiere.localhost",
        "sullyatraid.localhost": "rixiamao.localhost",
        "cecilneues.localhost": "tioplato.localhost",
        "lucyseiland.localhost": "cecilneues.localhost",
        "keabannings.localhost": "tioplato.localhost",
        "shizukumaclaine.localhost": "keabannings.localhost",
        "gracelynn.localhost": "tioplato.localhost",
        "eliemacdowell.localhost": "tioplato.localhost",
        "mariabellcrois.localhost": "eliemacdowell.localhost",
        // ,-> A few new hundred (randomly placed?) entries.
        // ¦-> Let's just fill the entire list with more lovely ladies from the Trails of... Universe.
        "suzanneegret.localhost": "musseegret.localhost",
        "ferridaalfayed.localhost": "fieclaussell.localhost",
        "risettetwinings.localhost": "ferridaalfayed.localhost",
        "agnesclaudel.localhost": "ferridaalfayed.localhost",
        "judithranster.localhost": "mariabellcrois.localhost",
        "lucreziaisselee.localhost": "risettetwinings.localhost",
        "marielleayme.localhost": "risettetwinings.localhost",
        "yumeayme.localhost": "marielleayme.localhost",
        "linacrawford.localhost": "junacrawford.localhost",
        "nanacrawford.localhost": "junacrawford.localhost",
        "ariellenheim.localhost": "priscillareisearnor.localhost",
        "auriervander.localhost": "towaherschel.localhost",
        "marthaherschel.localhost": "towaherschel.localhost",
        "fatimaworzel.localhost": "towaherschel.localhost",
        "sheedaworzel.localhost": "fatimaworzel.localhost",
        "lilyworzel.localhost": "fatimaworzel.localhost",
        "clariceseeker.localhost": "noelseeker.localhost",
        "kilikarouran.localhost": "scherazardharvey.localhost",
        "iseriaforst.localhost": "saravalestein.localhost",
        "lavianwinslet.localhost": "iseriaforst.localhost",
        "jaynastorm.localhost": "lavianwinslet.localhost",
        "aliciavonauslese.localhost": "klaudiavonauslese.localhost",
        "shanshan.localhost": "tioplato.localhost",
        "theresia.localhost": "alisareinford.localhost",
        "emily.localhost": "theresia.localhost",
        "jingo.localhost": "ashleigh.localhost",
        "ashleigh.localhost": "tioplato.localhost",
        "lapisrosenberg.localhost": "angelicarogner.localhost",
        "claravoce.localhost": "towaherschel.localhost",
        "linde.localhost": "claravoce.localhost",
        "vivi.localhost": "linde.localhost",
        "beatrix.localhost": "saravalestein.localhost",
        "sariffa.localhost": "ferrisflorald.localhost",
        "kaelamacmillan.localhost": "judithranster.localhost",
        "theresa.localhost": "klaudiavonauslese.localhost",
        "polly.localhost": "theresa.localhost",
        "mary.localhost": "theresa.localhost",
        "sashapetrovna.localhost": "lucyseiland.localhost",
        "tatiana.localhost": "towaherschel.localhost",
        "monica.localhost": "laurasarseid.localhost",
        "louise.localhost": "junacrawford.localhost",
        "sandy.localhost": "alfinreisearnor.localhost",
        "maybelle.localhost": "scherazardharvey.localhost",
        "valerie.localhost": "saravalestein.localhost",
        "patiry.localhost": "leonora.localhost",
        "lotte.localhost": "patiry.localhost",
        "annabelle.localhost": "lotte.localhost",
        "scarlet.localhost": "annabelle.localhost",
        "carna.localhost": "scarlet.localhost",
        "dorothyhyatt.localhost": "estellebright.localhost",
        "lila.localhost": "carna.localhost",
        "luciola.localhost": "lila.localhost",
        "paulette.localhost": "lila.localhost",
        "nadiarayne.localhost": "lapisrosenberg.localhost",
        "wendy.localhost": "paulette.localhost",
        "kate.localhost": "eliemacdowell.localhost",
        "hilda.localhost": "wendy.localhost",
        "elsacochrane.localhost": "risettetwinings.localhost",
        "najeberca.localhost": "kaelamacmillan.localhost",
        "ninafenly.localhost": "najeberca.localhost",
        "sophiahayworth.localhost": "rixiamao.localhost",
        "shizunaremmisurugi.localhost": "hilda.localhost",
        "lynn.localhost": "shizunaremmisurugi.localhost",
        "aeolia.localhost": "lynn.localhost",
        "maya.localhost": "tatiana.localhost",
        "colette.localhost": "aeolia.localhost",
        "mint.localhost": "titarussell.localhost",
        "erikarussell.localhost": "titarussell.localhost",
        "elaineauclair.localhost": "agnesclaudel.localhost",
        "katarinaford.localhost": "shirleyorlando.localhost",
        "anelaceelfead.localhost": "scherazardharvey.localhost",
        "leonora.localhost": "towaherschel.localhost",
        "adagrant.localhost": "shirleyorlando.localhost",
        "paula.localhost": "colette.localhost",
        "becky.localhost": "paula.localhost",
        "beryl.localhost": "becky.localhost",
        "dorothee.localhost": "emmamillstein.localhost",
        "edel.localhost": "fieclaussell.localhost",
        "friedel.localhost": "beryl.localhost",
        "bridget.localhost": "mint.localhost",
        "mirabelaalton.localhost": "friedel.localhost",
        "jillridonor.localhost": "klaudiavonauslese.localhost",
        "ainaholden.localhost": "scherazardharvey.localhost",
        "chloebarnett.localhost": "hilda.localhost",
        "juliaschwarz.localhost": "maya.localhost",
        "celestedauslese.localhost": "juliaschwarz.localhost",
        "esmerayarchette.localhost": "juliaschwarz.localhost",
        "kanoneamalthea.localhost": "lucreziaisselee.localhost"
}

function count_progeny(server) {
    let progeny = 0;
    for (let parent in servers) {
        if (server == servers[parent]) {
            progeny += 1;
        }
    }
    return progeny;
}

function make_pretty(old, position) {
    let string = Array.from(old);
    let loop = 0;
    while (loop < position.length) {
        let temp = position[loop];
        if (temp != -1) {
            if (string[temp] != " ") {
                if (string[temp] != CHAR_TAIL) {
                    string[temp] = " ";
                }
            }
        }
        loop += 1;
    }
    return string.join(" ");
}

function recurse_map(recursion, progeny, previous, tail) {
    let loop = 0;
    let string;
    let offspring = progeny;
    Object.entries(servers).forEach(([child, parent]) => {
        if (previous === parent) {
            loop++;
            progeny = count_progeny(child);
            string = Array(recursion).fill(CHAR_PIPE);
            if (progeny > 0) {
                if (loop != offspring) {
                    string.push(CHAR_PIPE);
                }
                else {
                    string.push(CHAR_TAIL);
                }
            }
            else {
                if (loop < offspring) {
                    string.push(CHAR_PIPE);
                }
                else {
                    string.push(CHAR_TAIL);
                }
            }
            if (string.indexOf(CHAR_TAIL) != -1) {
                tail = tail + " " + string.indexOf(CHAR_TAIL);
            }
            let temp = tail.split(" ");
            let display = make_pretty(string, temp) + CHAR_STUB + child;
            console.log(display);
            if (progeny > 0) {
                recurse_map((recursion + 1), progeny, child, tail);
            }
        }
    });
}

let ourParent = "towaherschel.localhost";
console.log(ourParent);
let progeny = count_progeny(ourParent);
if (progeny > 0) {
    recurse_map(0, progeny, ourParent, "-1");
}
console.log(END_OF_MAP);

// EOF
