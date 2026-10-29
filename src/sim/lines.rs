//! What people say. Vermont is a small state with a lot of opinions about
//! maple syrup.

use rand::RngExt;
use rand_chacha::ChaCha8Rng;

pub fn pick(rng: &mut ChaCha8Rng, lines: &'static [&'static str]) -> &'static str {
    lines[rng.random_range(0..lines.len())]
}

/// What the raccoon says when the door opens. It is trying its best.
pub const TRICK_OR_TREAT: &[&str] = &[
    "Trick or treat!",
    "Trick or chitter!",
    "Trrrick or treeeat.",
    "Treat or... treat?",
    "*polite chittering*",
    "Trick or treat! (I am a child.)",
];

pub const GREET: &[&str] = &[
    "Well, hello there!",
    "Ooh, a ghost!",
    "Happy Halloween!",
    "Look at you!",
    "Oh! Hello.",
    "Hi there, little one.",
    "Evening!",
    "Wicked good costume.",
];

pub const GIVE: &[&str] = &[
    "Here you go!",
    "Take two, it's Halloween.",
    "Grade A Amber, the good stuff.",
    "Don't eat it all at once.",
    "Spooky! Here.",
    "We only buy local. Mostly.",
    "My sugarhouse made these. Well, my cousin's.",
    "There's more maple where that came from.",
    "Stay warm, it's going to frost tonight.",
    "Happy Halloween, kiddo.",
];

pub const GIVE_MAPLE: &[&str] = &[
    "Maple candy. Obviously.",
    "Real maple. None of that pancake syrup.",
    "Tapped my own trees. Well, one tree.",
    "If it isn't maple, it isn't candy.",
    "Forty gallons of sap for one gallon of syrup. Enjoy.",
];

pub const GIVE_CHEESE: &[&str] = &[
    "Sharp cheddar. Aged two years. You're welcome.",
    "Candy is overrated. Cheese is forever.",
    "A cube of cheddar. Very Vermont.",
];

pub const NEARSIGHTED: &[&str] = &[
    "What a lovely costume, dear!",
    "My, what a fuzzy little ghost.",
    "Is that a tail? How creative!",
    "Oh, you must be the Hendersons' boy.",
    "Such a striped little ghost!",
];

pub const SUSPICIOUS_GIVE: &[&str] = &[
    "...Fine. One.",
    "Hm. Here.",
    "I'm watching you, ghost.",
    "Your tail's showing. Here.",
];

pub const DENTIST: &[&str] = &[
    "A toothbrush! Brush twice a day.",
    "Floss! And here's a toothbrush.",
    "Sugar is the real monster. Toothbrush?",
];

pub const GRUMPY: &[&str] = &[
    "Raisins. Take 'em or leave 'em.",
    "Back in my day we got a penny. Here's three.",
    "Kids these days. Here.",
    "We don't do candy. Apple?",
];

pub const FULL_SIZE: &[&str] = &[
    "Full-size bars! Go on!",
    "Only the best on Halloween.",
    "We're the full-size house. Tell your friends.",
];

pub const REFUSE: &[&str] = &[
    "Nice try, raccoon.",
    "Is that... a raccoon?",
    "Honey? There's a raccoon on the porch.",
    "I can see your whiskers.",
    "Nope. Not falling for that.",
    "Shoo! Go on!",
    "Are you even a kid?",
    "That's not a costume, that's a raccoon in a sheet.",
];

pub const REFUSE_GREEDY: &[&str] = &[
    "That's quite a haul you've got there. I think you're set.",
    "Your bag's bigger than you are! No more.",
    "Leave some for the other kids.",
];

pub const REFUSE_SMELL: &[&str] = &[
    "What is that SMELL?",
    "Did you step in something?",
    "Whew! No, thank you.",
];

pub const SCREAM: &[&str] = &[
    "AAAAH! RACCOON!",
    "EEEK! Get the broom!",
    "IT'S GOT RABIES! Close the door!",
];

pub const PITY: &[&str] = &[
    "...Oh, all right. You look so sad. Here.",
    "Don't tell anyone I gave a raccoon candy.",
    "I know you're a raccoon. Take one anyway.",
];

pub const AGAIN: &[&str] = &[
    "Didn't you already come by?",
    "Same ghost, twice? I don't think so.",
    "I remember that tail.",
];

pub const NEW_GHOST: &[&str] = &[
    "Oh, a different ghost! Here you go.",
    "Another ghost! There's a lot of you tonight.",
];

pub const EMPTY: &[&str] = &[
    "Sorry, we're all out!",
    "Cleaned out! Check back later.",
    "All gone. The bowl's empty.",
];

pub const NOBODY: &[&str] = &["(Nobody comes.)", "(Silence. A dog barks somewhere.)", "(The TV is on. Nobody comes.)"];

pub const OFFICE: &[&str] = &["(The building is dark. It's after five.)", "(A sign: CLOSED. HAPPY HALLOWEEN.)"];

/// Out walking, seeing the raccoon.
pub const FOLK_FRIENDLY: &[&str] = &[
    "Love the costume!",
    "Happy Halloween!",
    "Great night for it.",
    "Leaves are peaking this week, eh?",
    "Mud season's coming. Enjoy this.",
    "Did you try the cider donuts?",
    "Heard it's a big year for syrup.",
];

pub const FOLK_SUSPICIOUS: &[&str] = &[
    "Is that... a raccoon?",
    "Hey. HEY. That's a raccoon!",
    "Somebody call Fish and Wildlife.",
];

pub const FOLK_OBLIVIOUS: &[&str] = &[
    "...and that's why I drive a Subaru.",
    "Have you tried the new IPA at the taproom?",
    "I'm thinking about getting chickens.",
    "Town meeting's in March, you know.",
    "Do you know if the co-op has oat milk?",
];

pub const FOLK_GRUMPY: &[&str] = &["Kids.", "Watch it.", "Get off my lawn. It's not my lawn. Still."];

pub const FOLK_SCARED: &[&str] = &["AAH!", "Oh! Oh my.", "What is THAT?"];

pub const TOURIST: &[&str] = &[
    "A real Vermont raccoon! *click*",
    "Honey, get a picture! *flash*",
    "Is this the leaf peeping tour? *click*",
];

pub const PROFESSOR: &[&str] = &[
    "Is the sheet a commentary on the self?",
    "A ghost: absence made visible. Fascinating.",
    "Is this a performance piece?",
];

pub const LEGISLATOR: &[&str] = &[
    "Have you considered the Republic of Vermont?",
    "I'll introduce a bill about raccoons.",
    "Remember to vote at town meeting!",
];

pub const FARMER: &[&str] = &[
    "Sap ran early this year.",
    "Try some grade A dark. Changes your life.",
    "That's a lot of candy. Ever tried maple cream?",
];

/// Rival trick-or-treaters.
pub const RIVAL: &[&str] = &[
    "Race you to the next house!",
    "That house gives full-size bars!",
    "Mine's bigger.",
    "Nice costume. Is that a real tail?",
    "I'm a cow! Like the statues!",
    "Mom says only one more street.",
];

pub const RIVAL_RACCOON: &[&str] = &["RACCOON!!", "MOM! A RACCOON!", "It's a raccoon in a sheet!"];

pub const RIVAL_HAKARL: &[&str] = &["EWWWW!", "IT SMELLS LIKE FEET!", "RUN! It's the shark lady!", "I'm gonna barf!"];

pub const TWINS: &[&str] = &["Hey, we're twins!", "Nice sheet! Mine's from Target.", "Ghost buddies!"];

pub const TEEN: &[&str] = &["Nice haul, kid. Tax.", "Big bag. Sharing is caring.", "Halloween tax!"];

pub const MOM: &[&str] = &[
    "Hákarl! Fresh from the fridge! Well, the garage.",
    "Who wants hákarl?",
    "Loki! Skaði! Don't eat it all before bed!",
    "It's my grandmother's recipe!",
    "Five months in the ground, three on the line. Delicious.",
];

pub const LOKI: &[&str] = &[
    "RAWR! I'm a dinosaur!",
    "Look at my candy!",
    "Can I hold the chicken?",
    "I found an egg!",
    "Your tail is so real!",
    "Race you!",
    "I'm a T. rex. Rawr.",
];

pub const LOKI_OOPS: &[&str] = &["Oops!", "Sorry!", "Dinosaurs can't stop!"];

pub const LOKI_SUGAR: &[&str] = &["I'M A ROCKET!", "I can run to Canada!", "Is it bedtime? NO!", "Dinosaurs don't need to sit DOWN!"];

pub const LOKI_CRASH: &[&str] = &["WAAAAH! I don't know WHYYY!", "*sob* My legs are sad!", "I want... I want... WAAAH!", "Everything is TOO MUCH!"];

pub const SKADI_CRASH: &[&str] = &["Oh no. Sugar crash. Coming, Loki!", "Loki! It's okay, it's just the candy!", "Here we go. Hang on, Loki!"];

pub const SKADI_ENOUGH: &[&str] = &["Loki. No more candy.", "That's enough sugar for one dinosaur.", "How many have you EATEN?", "Mom is going to have words with me."];

pub const SKADI_BUMPED: &[&str] = &["LOKI!", "Watch where you're going!", "Ow! Loki!", "Hey, that was mine!"];

pub const FOLK_BUMPED: &[&str] = &["Whoa there, little dinosaur!", "Hey!", "Easy, buddy!", "Somebody's had a lot of candy."];

pub const LOKI_CRY: &[&str] = &["WAAAAH!", "My egg!", "*sniff* It broke!"];

pub const SKADI: &[&str] = &[
    "Meow. I'm a cat. It counts.",
    "Loki, stay on the sidewalk.",
    "Your costume's really good, by the way.",
    "That's the full-size house. Trust me.",
    "Gullinkombi, no!",
    "Mom's going to make us share.",
];

pub const SKADI_SORRY: &[&str] = &["Oh no, sorry sorry!", "Are you okay?", "I didn't see you!"];

pub const SKADI_COMFORT: &[&str] = &["It's okay, Loki. She'll lay another one.", "Hey, hey. It's okay.", "Come here, buddy."];

pub const SKADI_GIVEBACK: &[&str] = &["You dropped these.", "Here, these are yours.", "Don't lose them!"];

pub const SKADI_FIX: &[&str] = &["Hold still, your sheet's ripped.", "Here, let me fix that.", "There. Good as new. Mostly."];

pub const SKADI_FAIR: &[&str] = &["Take ONE. It says take one.", "That's not fair, put some back.", "Leave some for the little kids!"];

pub const JOY: &[&str] = &["HÁKARL!!!", "YAY!", "MOM'S HÁKARL!"];

pub const HEN: &[&str] = &["Bawk.", "Bawk?", "BAWK!", "Bok bok.", "Bwaaak."];

/// Robert Frost at the library door, quoting himself: lines from poems
/// published in 1913 to 1923 (A Boy's Will, North of Boston, Mountain
/// Interval and New Hampshire), in the public domain in the United States.
pub const FROST: &[&str] = &[
    // "Stopping by Woods on a Snowy Evening", 1923.
    "\"The woods are lovely, dark and deep, / But I have promises to keep, / And miles to go before I sleep.\" And candy to give before I sleep. Here.",
    // "Nothing Gold Can Stay", 1923.
    "\"Nature's first green is gold, / Her hardest hue to hold.\" Candy's the same. Eat it while it lasts.",
    // "The Road Not Taken", 1916.
    "\"Two roads diverged in a yellow wood, / And sorry I could not travel both...\" Take two pieces, then. It's Halloween.",
    // "Mending Wall", 1914.
    "\"Something there is that doesn't love a wall.\" Or a closed door. Here you are, ghost.",
    // "Mending Wall", 1914.
    "\"Good fences make good neighbors.\" Good candy makes better ones. Take one.",
    // "October", 1913.
    "\"O hushed October morning mild, / Thy leaves have ripened to the fall.\" Evening, too. And a bookmark.",
];

/// Calvin Coolidge (1872 to 1933), of Plymouth Notch, thirtieth President:
/// "Silent Cal". He hands it over and that is all.
pub const COOLIDGE: &[&str] = &["...", "*nods*", ".", "*hands it over*", "Mm."];

/// A grown-up offered hákarl.
pub const ADULT_HAKARL: &[&str] = &["Oh my GOD, what is that smell?", "No. No thank you. NO.", "Is that... fish? Why is it fish?", "I have to go. I have to go right now."];

/// A householder offered hákarl on their own doorstep.
pub const DOOR_HAKARL: &[&str] = &["AAAGH! What IS that?!", "Nope! Nope nope nope!", "Honey! Close the windows!", "I'm calling... somebody! Anybody!"];
