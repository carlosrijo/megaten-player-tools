# newmoon-reader

Read-only snapshots of a NewMoon character's stats and gear from the running ImagineClient,
plus the data files for the "Rubito Build Tracker" page.

It only ever opens the game process with read access (`PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`):
it never writes to the game or changes anything.

## Download (portable, nothing to install)

Get `NewMoonTracker.exe` from the [latest release](https://github.com/carlosrijo/megaten-player-tools/releases/latest),
put it in a folder of its own and run it. It needs 64-bit Windows 10 or 11 with the Microsoft Edge WebView2 Runtime
(already part of Windows 11). Your data goes in a `data` folder made beside the exe, so keep the exe somewhere you
can write to (not Program Files); to move it to another PC, copy the exe together with that folder. Your stats CSV
goes in `Documents\smt-stats` (change `statsFolder` in `data\config.json`).

The release has no pictures from the game (demon, enemy and DCM icons): those are the game's own art, made locally
from the client with the commands below, so those pages show names without icons.

### Making a release

Push a version tag; GitHub builds the exe and publishes the release (`.github\workflows\release.yml`):

```
git tag v1.0.0
git push origin v1.0.0
```

A tag with a dash (`v1.1.0-beta.1`) makes a pre-release. To try the build first, run the Release workflow by hand
(Actions > Release > Run workflow) and download the exe from the run, or build it here:
`pwsh -NoProfile -File tools\publish-portable.ps1` (gives `dist\NewMoonTracker.exe`, and `dist-symbols\NewMoonTracker.pdb`: the debug
symbols of that exe, which are never shipped; the workflow keeps them as a second artifact, for reading crash reports).

## Desktop app

The program is written in Rust (`crates\`, a Cargo workspace). It needs Rust (rustup, stable, `x86_64-pc-windows-msvc`)
and the Visual Studio C++ build tools.

```
cargo build --release
target\release\NewMoonTracker.exe
```

`cargo build --release` makes both programs in `target\release`: the app (`NewMoonTracker.exe`, with the page files
inside it) and the command line reader (`newmoon-reader.exe`). After a change, with the app possibly open:
`pwsh -NoProfile -File tools\rebuild-app.ps1` closes it, builds both and opens it again (Windows locks a running
program, so a build with the app open fails to replace it).

The C# version (`src\`, `app\*.cs`, `*.csproj`) is the one this was ported from, kept in the repository for comparison
until it is removed; changes go to the Rust code. `PORTING.md` says how the two correspond.

"NewMoon Build Tracker": one window with Overview, Bonuses, Gear (opens on "By piece", each piece with its icon from the game; hovering an icon shows a game-style tooltip), Expertise, Digitalize and History, a digitalize demon picker on the Overview ("No demon" is the normal view, a demon is the digitalized one, for every page),
and a **Take snapshot** button (F5) that runs the same read-only snapshot as the command line and then refreshes the page.
It shows `data\dash\*.json`, so it opens with the last saved data even when the game is closed.

- **Overview:** beside your picture, Max HP, Max MP and the 12 stats as compact bars (base, bonus and, in the
  Digitalized view, what the digitalize demon adds). Under them, three one-line bands:
  - **Expertise:** your expertise points against the cap, as the game's Expertise window shows them (e.g.
    52000.00pt / 60000.00pt, both read from the game), then each chain expertise you have unlocked with its
    class-rank as a short list (two columns when there is room). Beside it, the icons of every skill those ranks gave you, in one run (hover one for its name and chain; a switched-off toggle is dimmed);
    click a chain to open it on the Expertise page, or "View more" for the whole page.
  - **The digitalize demon:** its name is a dropdown to pick another one, or no demon.
  - **Gear:** every piece you wear as its icon from the game, then your complete sets. Hover an icon for the game's
    tooltip, click it to open that piece on the Gear page.

  Then **Key stats and affinities**, one row: on the left one line per key stat (a bar only where the stat has a cap,
  showing how close you are; it changes colour past the cap). Click a line to see what gear, expertise and the
  digitalize demon each add, and to jump to every source on Bonuses. On the right, one radar chart per element with
  your affinity boost, the cap it stops at (100%, or more for an element a passive raises: Ice Limit Mastery Lv1 makes
  Ice 150%) and your resistance; the buttons under it bring one of the three forward and show its numbers.
  At the bottom, across the page, **What changed**: the 5 biggest changes since an earlier snapshot ("Show all" lists
  the rest). The snapshot picker says how many changes each one has.
- **Expertise page:** opens with a **Summary**: points spent against the cap (with what is free), how many expertises are raised, at their highest class and chained, and how many passives are counted; then every raised expertise and chain expertise with its class-rank, a bar to its highest class ("Max" once reached; the highest class is the game client's own) and its points, with the icons of the skills its ranks gave you under it. Click one to show only its passives in the list below.
- **Assistant:** Black Frost stands in the window's bottom right corner, drawn in 3D from the game's own model with the
  game's own animations (the app reads them from your game folder the first time, so the game must have been running
  once with the app open). It idles, does something by itself now and then, plays a trick when you click it, casts
  while a snapshot is being read and cheers or flinches at the result. The app's messages are what it says: speech
  bubbles above its head. Hold the right mouse button on it and drag sideways to turn it around (it stays the way you
  leave it). Settings > Assistant hides it and brings it back.
- **Characters:** the name is read from the running game, nothing to type. With one character in game the app shows it;
  with two clients open it asks which one. Each character keeps its own folder and spreadsheet (`data\characters.json`);
  click the name in the header to switch.
- `app\MainForm.cs` the window (WinForms + WebView2) and the handful of requests the page can make.
- `app\ui\` the page: `index.html`, `app.css`, `app.js` (plain JavaScript, no libraries, works offline).
- `src\Build\Pipeline.cs` the snapshot / rebuild steps shared by the app and the command line.
- Preferences (character, CSV path, theme, window position) live in `%AppData%\NewMoonTracker\settings.json`.
- Theme: by default the app follows Windows' light/dark app setting, also when it changes while the app is open
  (Settings can pin Light or Dark). The window sets the page's colour scheme (`SetScheme` in `MainForm.cs`); the page
  only asks `prefers-color-scheme`. `--capture out.png --then-scheme light|dark` checks the live switch.

Working on the app's page:
- `NewMoonTracker.exe --dev` loads the page straight from `app\ui`, so page edits need no rebuild (F12 opens DevTools).
- `NewMoonTracker.exe --capture out.png [--page gear] [--theme dark|light] [--mode digi] [--size 1280x860]` saves a
  picture of the page and exits.
- In a plain browser: `tools\serve.ps1 -Root . -Port 18765`, then `http://localhost:18765/app/ui/index.html`.
  For the Simulator use `newmoon-reader.exe serve --port 18765 [--data <copy of the data folder>]` instead: the same
  page, with the real calculation behind the Simulator (the plain server has none, and the page says so).
  A stand-in for the app reads `data\dash`. Add `?empty=1` (first run), `?snap=fail` or `?snap=live` (snapshot results),
  `?game=1`, `?page=…`, `?theme=…`, `?mode=digi`.
- The assistant: `--capture` pictures leave it out unless the run adds `--query assistant=1`. In a plain browser it
  needs the model file first (`newmoon-reader.exe model`, below); `?assistant=0` leaves it out, `?assistant=none`
  stands in for "the game's folder isn't known yet".

### Tools: DCM Locator

Under **Tools** in the left bar. Lists every DCM (Demon Compendium Memory) and where the game data says it comes from:
shops with prices, enemies with drop rates and zones, chests, event or NPC rewards, random boxes, and presents from the
demon itself. Search by demon, enemy, zone or shop; the chips filter by kind of source. It works without a snapshot.
Each row shows the demon's own icon from your game client when the client has one.

The DCM list, zone names and shop stock are the NewMoon client's. Drops, chests, presents and events come from the
public server data, so NewMoon may have changed them; DCMs with "No known source" are most likely NewMoon additions.
To record what players confirmed on NewMoon, add notes to `app\ui\dcm-notes.json`
(`"notes": { "<item id>": ["Sold by X in Y for Z"] }`; the item id is shown at the bottom of an opened row).
They appear first, marked "NewMoon note".

### Tools: Quest Guide

Under **Tools** in the left bar. Lists every quest (680; 665 without the inactive ones) and, when opened, what to do
step by step: who to talk to and where (zone and map grid position), which dialogue choices to pick, what you need,
what you get and give, and the kill or collect counts. Where a step can be done in more than one way (for example a
Law or a Chaos answer) every way is shown with what it changes. Search by quest, person, place or item; the chips
filter by kind of quest.

The quest names, objective text and counts are the game's own. The steps come from the public server's event files
(468 quests have them; the rest show the game's objective text only), so NewMoon may have changed them. Valuables
(key items) are shown with the name in the game data, which is Japanese.
### Tools: Gear Database

Under **Tools** in the left bar. Every piece of gear in the game (10,270) with what it gives: stats, effects, appearance
bonus, the sets it belongs to, mod slots, and who can wear it. Where a piece comes from (dungeon, ticket, exchange)
is traced through the game data (drops, chests, shops, NPC exchanges, the ticket a piece comes in), with community notes from
SnowSheet's gear reference for NewMoon beside it. Search with several
words at once ("limit break celu"), filter by slot, and untick "Only gear with a known source" to see everything.

Stats and effects are read from the NewMoon client's current item tables, not from the spreadsheets. 963 spreadsheet
rows matched no item in the game data (different spelling); they are listed with the sheet's own text, marked
"sheet only". The cosmetic ticket tabs (Vanities) are left out.

Each piece shows the icon the game shows for it (10,168 of 10,270; 102 have no picture in the client).
The pictures are the game's own art, so they are not in the repository: they are made from your game folder by
`newmoon-reader geardb ... --client <game folder>` (reference\README.md), into `app\ui\gear-db-icons\`. Without
them the list shows empty boxes.

### Tools: Fusion Planner

Under **Tools** in the left bar. Two views:

- **Fuse demons**: pick two demons (or three for a tri-fusion) and see what they make, with why: the races, the
  fusion level worked out from their levels, elemental rank up or down, special fusions and the plugin they need. Each
  demon starts at its base level; type its real level, since the result depends on it.
- **Find a recipe**: pick the demon you want and get every way to make it: special fusions, every
  two-demon fusion and every tri-fusion, cheapest first (lowest levels added up), each as a card in an even grid. A special
  fusion that needs a plugin says which key item; hover it for how to get it in game. Recipes with an elemental are marked
  **Rank up** or **Rank down**. Click any demon in a recipe to see how to make that one too ("Back to" returns).

Mitama-fused demons are not in the lists: the game won't fuse them. **Find a recipe** only offers demons some fusion
makes (417 of 850); the others come from quests, events or contracts.

Demons, levels, races and special fusions are the NewMoon client's. The race chart and the tri-fusion and elemental
rules are the comp_hack server's fusion code, which NewMoon may have changed. Checked in game (2026-11-23): two-demon fusions, a tri-fusion and the refusal of Mitama-fused demons, all as the planner says.
### Tools: Dungeon Explorer

Under **Tools** in the left bar. Every dungeon (175; each difficulty such as Bronze, Silver and Gold on its own), field and other place with enemies, and the enemies in it that
can't be contracted (4,003 kinds): the ones you can't talk to at all, and the ones that talk but never join. Click a
place for its enemies (picture, race, level, Max HP, boss mark, the floor when the floors have names, and what each is
weak to and nulls, reflects or absorbs, so you can read them without opening anything), and an
enemy for its details:

- **Stats**: Max HP and MP, STR to LUCK, and CLS-R, LNG-R, Spell, Support, P.Def, M.Def at that level.
- **Affinities**: what it is weak to and resists (in %), and what it nulls, reflects or absorbs.
- **Skills**, **enemy-only skills** and **traits** as the game's icons; hover one for its name and the game's own
  description. Skills the game has no icon for (most enemy-only moves and all traits) show their name instead.
- **Drops** with the item's picture and the chance per kill, and the XP it gives.

An opened place also shows **Drops here**: every item its enemies can drop, as the game's item pictures with the best
chance under each. Hover one for the game's description and which enemies drop it there, at what rate ("Every enemy
here" for the place's own drops). Below it: the **End chest** (what the boss chest at the end of a stage can hold,
over every stage of the place) and **Other chests**. The game-wide drops any enemy anywhere can give (the
Jack/Queen/King/Ace boxes, the Invoke emblems) are left out. A place whose enemies drop nothing of their own
(Dark Babel) says so: NewMoon sets those drops on its own server. All rates are the vanilla comp_hack server data's
(your local server's), before luck and drop bonuses; one short warning banner above the drops says so.

Places are grouped by where you enter them from (all of Dark Babel's floors under "Entered from Dark Babel", Celu
Tower's Bronze, Silver and Gold under "Entered from Celu Tower"); click a group to open it. Fields, other places and
the few dungeons whose entrance the data doesn't give come last. Untick "Group by entrance" for one flat list.
Each place shows small pictures of its enemies beside its name (up to 8, bosses first with a purple ring, then the
strongest; "+N" for the rest); hover one for its name and level.
Search a place, an enemy, a skill, a drop or an entrance; the chips filter dungeons, fields and other places. A search
opens every group it finds something in, and searching an enemy lists only the matching enemies inside each place.

Names, stats, skills and traits are the NewMoon client's. Which enemies are where, their levels, whether they talk and
their drops come from the public server data, so NewMoon may have changed them. Stats are the server's formula for an
enemy at that level (with the extra HP the game gives enemies), before its passive skills and traits. Not checked in
game.
### Tools: Demon Database

Under **Tools** in the left bar. Every demon you can own (the compendium's 359) with its picture, race, base level and
alignment. Click a demon for its stats at any level from its base level to 99 (a fresh demon: no Demon Force, rebirth
or Mitama), its affinities (resistance in %, weaknesses in red, natural null / reflect / drain chances), its features and its
Mitama features (what it gains, or swaps, when Mitama-fused),
its skills (what it knows and the level it learns each one at; hover for the game's description), what its tarot and
soul give and on which gear, and where to get it: the places it can be contracted with their levels, its DCM (with a
button to the DCM Locator) and normal fusion. "Normal" / "Mitama-fused" at the top of a demon switches its stats,
affinities, features and skills to its Mitama-fused form (340 demons have one).

The list opens on the demons that can be contracted somewhere (140); "Not contracted anywhere" and "All demons" show
the rest. Dungeon and boss copies of a demon are not listed on their own: the places they appear, where they cannot be
talked into a contract, are under "Enemy only". Search by name, race, skill, feature, tarot text or zone; filter by
race and alignment (Law, Neutral, Chaos); sort by level, name or race. Demons, stats, skills, tarot and soul are the NewMoon client's; spawns are the
public server data, so NewMoon may have changed them.

### Tools: Skill Database

Under **Tools** in the left bar. Every skill in the game, separated by action type: Attack, Spin, Rush, Shot, Rapid,
Guard, Counter, Dodge, Talk, Threaten, Provoke and Curative (the game's own names and order, from its Help), then
Passive and Switch skills, which have no action type. Each row has the skill's picture, element, cost (MP, HP,
bullets or items), cast time, cooldown, modifier and stiffness (the two numbers the game's description ends with) and who can get it. Pick a type at the top to
see only those skills and what the game's Help says about that type (what it beats and what beats it). Click a skill
for its in-game description, its target, reach and the stat it uses, the expertise rank that teaches it and every
demon that learns it (with the level) or has it as a feature; click a demon to open it in the Demon Database.

The list opens on the skills you can get (learned by a compendium demon, a demon feature, from expertise, or a fusion
skill: 1,451). "Who can get it" narrows that down or shows every skill in the game data (3,120, with enemy and event
skills). Search by name, element, words of the description, expertise or demon; filter by element; sort by name,
modifier, stiffness or cooldown. Item uses, debug skills and the 777 skills the client only names in Japanese are left out.
Destruction magic (Agi, Zio...) is a Shot skill in the game data; healing, support and curse spells are Curative.

### Tools: Simulator

Under **Tools** in the left bar. Try other gear and expertise ranks and watch your stats change, without touching the
game or your saved data. It starts from your character as last read (take a snapshot first).

- **Gear**: open a slot to change any part of the piece, numbered as in the game's fusion window: Slot 1 (the piece it
  counts as: set bonus and appearance bonus), Slot 2 (whose basic features it carries), Slot 3 (whose characteristics),
  the tarot, the soul fusion, and the fusion bonus (the LIMIT switch: every fusion bonus of that kind of piece at MAX,
  +45 on a weapon). "Replace the whole piece" changes Slots 1 to 3 at once; a piece can also be taken off. Mods stay as
  they are.
- **Picking**: the list shows what each choice gives for that part. Search a name or an effect ("rapid", "limit break
  power"): searching an effect puts the pieces that give the most of it first. The highlighted row (arrow keys or the
  pointer) shows what it would change before you pick it; Enter picks.
- **Picked before**: everything you pick stays as a chip under its dropdown. Click a chip to use it again (handy to
  flip between two or three candidates), the x takes it off the list. Undo still goes back to what you wear.
- **Expertise**: one slider per expertise (rank by rank, or a class at a time), with the points spent against your cap.
  The cap itself can be set (type it, or − and + by one expansion) to plan for a cap you don't have yet.
  Chain expertises follow by themselves. Click a name to see what its ranks unlock.
- **Result** (right): every difference first (your 12 stats, Max HP and MP, then each bonus, with what it was), then the
  full stat sheet with the changes marked. Sets won and lost are named.
- Every change has an Undo. **Reset to default** (top right of the gear list) goes back to your character as last
  read: gear, expertise ranks and point cap ("Undo reset" brings back what you had, until you change something else).
  The chips stay.
- **Builds**: the Simulator opens on your builds: a "New build" card and one card per saved build (its name, how many
  changes from what you wear, the pieces changed and the biggest differences). In a new build, "Save build…" gives
  it a name; from then on it keeps every change you make to it. A build can be renamed (click its name), duplicated
  and deleted. Saved builds are also listed under **Simulator** in the left bar; the small arrow there folds the list.
  A build is a whole plan: when your gear changes, it is compared with your new gear. Each card names its base.
- **Builds are independent of the character shown.** A new build starts empty ("No base"): its numbers are only what
  its pieces and expertise ranks give, for the level and alignment set under its name.
- **Import base** (top right of a build): pick one of your characters and the build starts again from that
  character's gear and expertise as last read; its numbers are then that character's, and every change is shown
  against them. Nothing of the character is changed. "Undo import" brings back what the build had; "No base" in the
  same list goes back to an empty build.
- **Normal or digitalized**: under a build's name, a demon band like the Overview's. Its picker lists the demons of
  the build's base (the imported character's COMP demons as last seen, whoever the app is showing) and "No demon".
  The demon is part of the build: picking one changes nothing for the character, and it is saved with the build.
- A stat that moved is tinted green (better) or red (worse).

The numbers come from the same calculation as the rest of the app. Stats start from the game's own numbers and move by
what the change adds, in the server's order (a point of SPD is half a point of LNG-R, and so on). Not counted, as
everywhere: buffs, and effects the game data has no numbers for (those are listed on the piece and named in a notice).

## When something goes wrong

Everything for this is in **Settings** (the gear at the top right):

- **About this version** shows the version, the build's commit and date, your Windows and the WebView2 runtime it runs on.
  **Copy** puts those lines on the clipboard, to paste into a bug report.
- **Check my setup** looks at your files, the game and each part of the reader in one pass, and says line by line what is
  fine (pass), worth a look (warn) or broken (fail), and what to do about it. It checks that the data folder can be
  written to, the paths in `data\config.json`, which clients are running and who is logged in on each, whether the game is
  still the build the reader was made for (the app also says so at start when the game was updated: *readings may be
  wrong until the reader is updated*), that each client can be opened read-only, and that each part is found in the
  game's memory (character, nameplate, stats, gear, demons, progress) and how long it took. The command line has the same:
  `newmoon-reader.exe doctor` (add `--json` for a machine-readable version).
- **The log** is in `data\logs` beside the program (**Show logs** opens the folder). A few small text files, 5 MB at most
  altogether, the newest kept. It says what the app did and how long it took, which parts of the game were found, and any
  error. It never holds what the page asked for (that can be personal), spreadsheet rows, or anything read from the game's
  memory. `--log-level debug|info|warn` (on `NewMoonTracker.exe` and `newmoon-reader.exe`) changes how much goes in;
  `newmoon-reader.exe --version` says which build you have.
- **A crash** writes `crash-<date>-<time>.txt` beside the log (what went wrong, where, the version, Windows, a stack trace),
  and the app tells you where it is instead of just closing.
- **Save a support bundle…** writes one zip (to your Desktop unless you pick another place) to send to whoever helps you.
  In it: the version, a fresh setup check, the log and crash files, `config.json`, `characters.json`, the app's settings
  file, and a list of your data folder's files (names, sizes and dates, not what is in them). **Never** in it: your
  spreadsheets, anything read from the game's memory, pictures, item or tooltip data. Your Windows user name is taken out of
  every path (`C:\Users\<name>` becomes `%USERPROFILE%`), and character names are replaced by "Character 1", "Character 2"…
  unless you tick **Include character names**. It is a plain zip: open it and look before you send it.

## Commands

```
cargo build --release
target\release\newmoon-reader.exe clients    # which clients are running and who is logged in to each
target\release\newmoon-reader.exe doctor     # the self-check: files, game build, each reader found? (pass / warn / fail; --json for machines)
target\release\newmoon-reader.exe --version  # which build this is: version, commit, build date
target\release\newmoon-reader.exe snapshot   # read the game, add a row to the CSV, rebuild page data
target\release\newmoon-reader.exe rebuild    # rebuild page data from saved files (game not needed)
target\release\newmoon-reader.exe recalc     # recompute every CSV row's gear bonuses, then rebuild
target\release\newmoon-reader.exe sets       # list set and appearance bonuses for the worn gear
target\release\newmoon-reader.exe simulate   # check the Simulator: the build as read must give back the readings and the saved bonus list
target\release\newmoon-reader.exe simulate --spec build.json   # the result of another build, as JSON (--init: the build as read; --catalog N: what fits slot N)
target\release\newmoon-reader.exe serve --port 18765           # the app's page in a browser, with a working Simulator (for testing)
target\release\newmoon-reader.exe alignment       # the character's alignment, read from the game (LAW/NEUTRAL/CHAOS overrides until the next snapshot)
target\release\newmoon-reader.exe demon           # list the COMP demons
target\release\newmoon-reader.exe demon Specter   # one demon in detail and what digitalizing with it gives
target\release\newmoon-reader.exe demon Specter --use   # make it the digitalize demon the page adds
target\release\newmoon-reader.exe digilevels      # Digitalization Level and % per race, assists, level-unlock skills
target\release\newmoon-reader.exe expertise       # re-read expertise ranks and Digitalization Levels, rebuild (no CSV row)
target\release\newmoon-reader.exe toggles         # which toggle skills are on, read from the game ("Name, ..." overrides when it isn't running)
target\release\newmoon-reader.exe labels          # regenerate reference\stat-labels-game.tsv: the game's own name for each stat (from the client's effect text)
et10.0
ewmoon-reader.exe toggles         # which toggle skills are on, read from the game (a "Name, ..." list overrides when it isn't running)
```

`demon "<name>"` shows the demon's window values, rebirth ranks, skill slots, learned skills, traits, Mitama slots and
sets, then the digitalize result: the stat rate (10%, 30% for Mitama-fused types, raised by stat-rate assists), the
stats the character gets, and the effects of its passives, traits, Mitama sets and affinities. The learned digitalize
assists (one pick per Digitalization Level) are read from the game; `--assists` overrides them with `all`, assist types
(`STAT_RATE,PASSIVES,TRAITS,MITAMA_SET,...`, all of that race's assists of the type) or assist ids. Parts whose assist
isn't learned are listed but not counted. Boosted Mitama set bonuses apply when the character has "Mitama Demon
Enhancement - Iron Man" (read from the learned skills; `--mitama-boost yes|no` overrides). Checked against the game:
digitalized with the Specter, Rubito's stats rose by exactly the computed amounts.

**The demon's skills.** In the Digitalized view, the demon's card (Overview and Digitalize) shows the demon's own skills.
Closed, its line has the icons of the skills in its slots. "Details" also lists them: the skills, then its features. Each has its icon, what it gives you, and the game's description on hover. A
skill you don't get while digitalized is dimmed (for example, its features when the race's Features boost isn't learned).

**Demons you have unlocked.** The Digitalize page shows them as a grid of small pictures; "Details" lists, per race you have a Digitalization Level in, the demons those
levels unlock, with the level that unlocks each. Demons of a level you haven't reached are dimmed, and one you have in
your COMP is marked. The demon dropdown shows each COMP demon as a card: the share of its stats you would get, the
skills in its slots and the key bonuses it would add, worked out with your learned Digitalize boosts.

**Digitalize demon on the page.** A snapshot taken while digitalized saves that demon as the character's digitalize
demon (`data\cache\digitalize.json`). To pick one without digitalizing, click the demon's name on the Overview's demon line (in the
app; a dropdown of "No demon" and the demons seen before, with "Load demons from the game" at the end to read the COMP again), or `demon "<name>" --use`. The picker lists your COMP as the game has it
(open the COMP in game, then "Load demons from the game"); each demon says whether your Digitalization Levels unlock it. The game only
loads a demon's stats and skills when you look at it, so a demon shows "Not read yet" until you click it in the COMP in game, or open the
fusion window at the Cathedral of Shadows (which loads your whole COMP at once), and load again (the summoned demon is always read). What was read is kept, also for a demon you later move out of the
COMP (its card then says "Not in your COMP") (`data\cache\comp-demons.json`, with icons in
`data\cache\demon-icons\`), so the picker also works without the game. In the app, picking never reads the game: it uses the demons and the learned
assists last read (by a snapshot, or by "Load demons from the game" in the dropdown). Picking one saves it, recalculates the latest snapshot's
demon columns with it (the CSV is copied to `<csv>.backup-before-demon-switch.csv` first) and rebuilds the page. Later
snapshots refresh it from the COMP, and one taken while digitalized switches to the demon in use. The demon that was summoned when the game was last read (a snapshot, or "Load demons from the game") shows as a small card to the right of your stats on the Overview. The Digitalized view then adds it: a demon line on the Overview (framed in its alignment colour, as the player picture is in theirs; its 32x32 game icon, converted from the
client's `Interface\tga` into `data\dash\demon.png` when the demon is saved; **Details** opens the stats it adds and its assists per level), a
third part in each key stat's breakdown, and
a demon column on Bonuses (rows only the demon gives appear in that view only). The CSV gets "Digitalize demon",
"<stat> (demon, digitalized)" per key stat, and "Digitalize demon record" (so `recalc` can redo it). A snapshot taken
while digitalized doesn't take the window's 12 stats/HP/MP (they include the demon); the saved normal ones are used.

Options: `--name <character>` (default: whoever is logged in, or the only character with saved data),
`--csv <path>` and `--data <dir>` (default: that character's own, from `data\characters.json`), `--ignore-window`.

The quest bonus and the Demonic Compendium bonus need two counts, which the game only has once their windows have
been opened: Quest Completion Bonuses (its completed-quest list) and Demonic Compendium (its "Demons: 60/339" text).
After that, each snapshot (or `extras`) reads them and saves them, and the saved counts are kept until the next
reading. The Expertise tab shows a "not counted" row for whichever is missing. They can also be set by hand:

```
newmoon-reader extras --quests 32 --compendium 60      # --compendium-v2 no if you don't own Compendium v2; none = unset
target\release\newmoon-reader.exe model           # the assistant's 3D model (Black Frost) from the game folder into data\cache\models, with a summary
target\release\newmoon-reader.exe model --check "<game folder>\Devil\bone"   # which of the client's model/animation files the reader can read
```

Without options, `extras` shows the clan (read from the game), reads both counts when the game has them, and lists
every bonus they give.

## Where things come from

| What | Source |
|---|---|
| 12 stats, HP/MP | The own characters status object (always in memory; the CSV says "Character data"). While the Information window is open its block is used instead; it gives the same numbers |
| Equipped items | Gear grid (always in memory; newest complete grid wins) |
| Level | Nameplate record |
| Item bonuses (stats, characteristics, tarot, soul, mods) | Each worn item's record in memory (stats source item, special-effect source item, mods, fusion great-success bonuses, tarot, soul) worked out with the server data (`Calc\ItemEffects.cs`). A part with no server data (e.g. Ogami's Crystal) uses the item's tooltip text (cached per item id); so do old snapshots without records |
| Set and appearance bonuses | The client's equipment-set table (one-piece entries are appearance bonuses), with effect numbers from the server data |
| Fusion (soul/tarot) set bonuses | Each slot's tarot/soul enchant ids (the worn item's own record; the gear grid as fallback) checked against the enchant-set table: active only when every enchant in the set is worn |
| Expertise ranks | The client's expertise tree; chain expertises use NewMoon's chain formulas and rank skills (client data, `reference/expertise-*.tsv`). Toggle skills count only while switched on (read from the game: the learned-skill map's "on" flag) |
| Other learned passives | Every learned passive skill that no expertise rank or Digitalization Level gives (Additional HP/MP, "Additional" affinity skills, Quickness, clan skills, Quickdraw...), from the learned-skill map; shown as "Other learned skills" on the Expertise tab |
| Digitalization Levels | The character's progress data in memory (level and points per race); each level's "extend" passive skill counts like an expertise passive (one group per race on the Digitalize page, which also shows the digitalize demon) |
| The assistant (Black Frost) | The client's own 3D files: `Devil\bone\D2_11_003a_Base.nif` (model, bones, texture) and the 15 animations its `.kfm` lists (Jack Frost's `.kf` files, which the game itself uses for Black Frost). Read by `src\Data\Nif.cs`, turned into one file for the page by `src\Build\DemonModel.cs` |
| COMP demons (`demon`) | Each demon's records in memory (`Game\DemonReader.cs`): level, growth type, rebirth ranks, Mitama slots, skill slots, learned skills; base values, growth, traits and Mitama tables from the server's demon tables (`reference\`) |

Layouts are documented in `src/Game/ClientReader.cs`.

## Files

- `crates/` the program (Rust): `newmoon/` the reader library (`memory`, `game`, `data`, `calc`, `build`, one file per
  former C# file), `newmoon-reader/` the command line, `newmoon-tracker/` the desktop app (window, WebView2, page bridge).
- `src/` the previous C# code, kept for comparison. `Memory/` read-only process access; `Game/` structure layouts; `Calc/` tooltip parsing and bonus math;
  `Build/` snapshot, CSV, expertise and page-data builders.
- `data/cache/` last stats, tooltip text per item, the set table, alignment. `data/cache/models/` the assistant's model,
  made from the game's files (the game's art: not in the repository).
- `app/ui/assistant.js` draws the assistant (plain WebGL, no library).
- `reference/enchants-game.tsv` the client's own enchant table; `reference/devils-game.tsv`, `devil-growth.tsv`,
  `mitama-*.tsv`, `guardian-assist.tsv`, `guardian-levels.tsv`, `skills-digitize.tsv` the server's demon and digitalize tables;
  `tokusei-*.tsv`, `expertise-newmoon.tsv`, `traits-unresolved.tsv` NewMoon data from the client (see
  `reference/README.md`; `tools/sbinexport` regenerates them).
- `tools/memdbg` is a read-only memory explorer for finding new data in the client (modes listed in its header).
- `.claude/skills/` holds skills for AI agents working on this repo: dev workflow, memory investigation, checking
  numbers against the game, the server's rules, and decoding game data.
- `data/tooltips.txt` also saves each slot's item record (`(item …)` in the header) and the level; the CSV's
  "Gear records" column does the same per row, so `rebuild` and `recalc` work from ids too.
- `data/dash/` page data: `snapshots.json`, `gear.json`, `expertise.json`, `bonuses.json`. `skill-icons/` holds the skill icons the Expertise and Digitalize pages show, converted from the game client's own icon files.
- Machine-specific paths live in `data/config.json` (created with detected defaults on first run; edit when something is elsewhere): `websiteContent` (the website's content folder), `tokuseiXml` (comp_hack's `datastore/data/tokusei`), `serverConstants` (the local server's `config/constants.xml`: digitalize rates, Mitama set boost skill), `statsFolder` (where new characters' CSVs go), `defaultCharacter`.
