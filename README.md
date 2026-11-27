# imagine-reader

Read-only snapshots of an IMAGINE character's stats and gear from the running ImagineClient,
and BABEL, the desktop app that shows them.

It only ever opens the game process with read access (`PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`):
it never writes to the game or changes anything.

## Download (portable, nothing to install)

Get `BABEL.exe` from the [latest release](https://github.com/carlosrijo/megaten-player-tools/releases/latest),
put it in a folder of its own and run it. It needs 64-bit Windows 10 or 11 with the Microsoft Edge WebView2 Runtime
(already part of Windows 11). Your data goes in a `data` folder made beside the exe, so keep the exe somewhere you
can write to (not Program Files); to move it to another PC, copy the exe together with that folder. Your stats CSV
goes in `Documents\smt-stats` (change `statsFolder` in `data\config.json`).

### Check the download

Each release lists the SHA-256 of its `BABEL.exe` in its notes (and in `BABEL.exe.sha256` beside
it). It is the fingerprint of the exact exe GitHub built from the release's tag, the one the team tested. Before the
first run, in PowerShell in the exe's folder:

```powershell
(Get-FileHash .\BABEL.exe -Algorithm SHA256).Hash
```

It must match the release's value (capital or small letters do not matter). If it does not, the file is not the one
released: delete it and download it again from the release page, never from a copy someone passed on.

### Updates

From 0.10.0 on, BABEL looks for a newer release a few seconds after it starts (and every hour while it is
open). When one is out, a notice says so. **Update and restart** downloads it, checks it, puts it in place of the
exe and opens BABEL again; nothing is installed until you click. Settings > Updates shows the release notes and
has **Check now**, plus two switches: *Look for a new version when BABEL starts* (on) and *Include test versions
(pre-releases)* (off).

The check before installing: the new exe's size and SHA-256 must match the release, and its signature
(`BABEL.exe.sig`, made by the release build with a key only it has) must check with the key built into BABEL.
Only a version newer than the running one is installed. The old exe is kept as `BABEL.exe.old` until the new one
has started, then removed. The `data` folder is not touched. BABEL must be able to write in its own folder;
where it can't (Program Files), it says so and the release page has the download. Builds made with `cargo build`
never update themselves.

The release has no pictures from the game: they are the game's own art. The app makes the Tools pages' pictures
(Gear, Demon and Skill Database, DCM Locator, Dungeon Database) from the player's own game folder the first time a
page shows them, and keeps them in `data\cache\page-icons`. It needs to have seen the game running once (to know
where it is installed); until then those pages show names without pictures.

`reference\page-icons.tsv` says which file of the game's `Interface` folder each picture is. After the page builders
below make new pictures, make it again from them (it compares pixels, so renamed pictures are found too):

```
imagine-reader page-icons --client <game folder>
```

### Making a release

Push a version tag; GitHub builds the exe and publishes the release (`.github\workflows\release.yml`):

```
git tag v1.0.0
git push origin v1.0.0
```

Before the tag, write what is new for players in `release-notes\v1.0.0.md` (plain words, a short bullet list) and
commit it with the release: the workflow puts that file at the top of the release page, above the SHA-256. Without
the file the release page has only the SHA-256 and a link to the full list of changes.

The workflow also signs the exe for the updater (`BABEL.exe.sig`, see Updates above) with the repository's Actions
secret `BABEL_UPDATE_KEY`: the private half of the key whose public half is in
`crates\imagine-tracker\src\update.rs`. A tag fails without the secret, since players' BABEL would refuse that
release. Never change the key without a release that carries the new public half first: versions with the old
key built in can only install a release signed with it.

A tag with a dash (`v1.1.0-beta.1`) makes a pre-release. To try the build first, run the Release workflow by hand
(Actions > Release > Run workflow) and download the exe from the run, or build it here:
`pwsh -NoProfile -File tools\publish-portable.ps1` (gives `dist\BABEL.exe`, and `dist-symbols\BABEL.pdb`: the debug
symbols of that exe, which are never shipped; the workflow keeps them as a second artifact, for reading crash reports).

### Feedback from players

The app's **Send feedback** button (top bar) posts to a small Cloudflare Worker (`feedback-worker\`, free plan), which
opens a GitHub issue as the "BABEL feedback" GitHub App (Issues only, this repo only), labelled `feedback` and
`bug` / `enhancement` / `question`. Players need no GitHub account; the app holds no GitHub key.

- Attached diagnostics files are encrypted in the app with the public key in `app.js` (`FEEDBACK_FILES_KEY`) and kept
  180 days in the Worker's KV store. Open one with the id from its issue:
  `pwsh -NoProfile -File feedback-worker\open-files.ps1 <id>` (files land in `.secrets\feedback-files\<id>\`).
- Secrets stay in `.secrets\` (git-ignored): the GitHub App's key (loaded into Cloudflare with
  `feedback-worker\set-key.ps1`) and `feedback-files-private.pem`. Without that file nobody can open attachments:
  keep a copy somewhere safe.
- Change the Worker: `cd feedback-worker`, `npm install`, `npm test`, `npx wrangler deploy` (wrangler signed in with
  `npx wrangler login`). The browser preview of the app only sends test requests (no issue; files kept an hour).
- `feedback-worker\wrangler.toml` (the account's ids) is git-ignored and kept in the main checkout only;
  `wrangler.example.toml` is its shape. A new setup: a Cloudflare account, `npx wrangler kv namespace create
  FEEDBACK_FILES`, a GitHub App with Issues read/write installed on this repo, then fill in the copy and deploy.

## Desktop app

The program is written in Rust (`crates\`, a Cargo workspace). It needs Rust (rustup, stable, `x86_64-pc-windows-msvc`)
and the Visual Studio C++ build tools.

```
cargo build --release
target\release\BABEL.exe
```

`cargo build --release` makes both programs in `target\release`: the app (`BABEL.exe`, with the page files
inside it) and the command line reader (`imagine-reader.exe`). After a change, with the app possibly open:
`pwsh -NoProfile -File tools\rebuild-app.ps1` closes it, builds both and opens it again (Windows locks a running
program, so a build with the app open fails to replace it).
`--release` is the everyday build (no link-time optimization, so a change rebuilds in seconds); the exe players get is
built with `cargo build --profile dist` (in `target\dist`: smaller, about two minutes), which `tools\publish-portable.ps1`
does for you.

"BABEL": one window with Overview, Bonuses, Gear (opens on "By piece", each piece with its icon from the game; hovering an icon shows a game-style tooltip), Expertise, Digitalize and History, a digitalize demon picker on the Overview ("No demon" is the normal view, a demon is the digitalized one, for every page),
and a **Take snapshot** button (F5) that runs the same read-only snapshot as the command line and then refreshes the page.
It shows `data\dash\*.json`, so it opens with the last saved data even when the game is closed.

**Back and forward**: the arrows on either side of the page's name in the top bar (also Alt+Left / Alt+Right and the
mouse's side buttons) go back to the page you came from, for example after a demon link opened the Demon Database
or the Overview's expertise opened the Expertise page. The page comes back as you left it: its search, filters,
opened rows and scroll position. Opening and closing a Build Planner build counts as a step too. Hovering an arrow says
where it goes. The list lasts while the app is open (up to 50 steps).

The info button beside **Take snapshot** (and the Overview before a character's first snapshot) lists what to open in
game first, because the game only loads some data when its window is opened: start a Double Fusion at the Cathedral
of Shadows and cancel it (loads every COMP demon, so the digitalize demon can be read), open Quest Completion Bonuses
(Menu > Quest > View Quest Bonuses) and the Demonic Compendium (Menu > Devil > Demonic Compendium) once (the quest and
compendium counts), and keep the Information window (Menu > PC > Status Equipment) open (exact stats, tarot and soul,
the picture). The menu paths show the game's own menu pictures, cut from the client's
`Interface\tga\menu_icon_00.tga` when the page asks (page command `menuIcons`; nothing is saved).

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
  digitalize demon each add, and to jump to every source on Bonuses. An Experience group shows how much extra EXP
  and expertise points you earn (gear and clan skills), and how much extra EXP your summoned partner earns (the
  Demonology Sympathy skill for its race, if you have one). On the right, one radar chart per element with
  your affinity boost, the cap it stops at (100%, or more for an element a passive raises: Ice Limit Mastery Lv1 makes
  Ice 150%) and your resistance; the buttons under it show all three or just one (its numbers then appear on the
  spokes, and the scale fits what is shown). With all three, the outer ring fits your boosts and resistances, and a
  cap that runs past it stops at the rim with its real value beside it.
  Under the key stats, **What changed**: the 5 biggest changes since an earlier snapshot ("Show all" lists
  the rest). The snapshot picker says how many changes each one has. The bin beside it deletes the picked snapshot
  (its row in your spreadsheet, which is copied first to `<name>.backup-before-delete-<time>.csv`); the latest
  snapshot is not in the picker and can't be deleted.
- **Expertise page:** opens with a **Summary**: points spent against the cap (with what is free), how many expertises are raised, at their highest class and chained, and how many passives are counted; then every raised expertise and chain expertise with its class-rank, a bar to its highest class ("Max" once reached; the highest class is the game client's own) and its points, with the icons of the skills its ranks gave you under it. Click one to show only its passives in the list below; hover a skill icon for its name, the expertise rank that gives it and the game's description. Under it, **Expertise cap**: what your cap is made of (17,000 plus 1,000 per 10 levels, and at 99, then the raises; the page shows points without ".00pt"), how much of every raise there is you have, and one line per place that raises it: Golden Ark Casino, Gomory (Diaspora Suginami and Shinagawa), Shiho (Digitalize, with the races each reward asks for), Naotura's and St. Germain's quests, and on NewMoon its own, marked NewMoon: the main story (Act 7, Act 16, Act XX) and the two Black Frost exchanges in Dark Babel. Each raise is a chip: orange when you have it, outlined when not, outlined in orange when a Shiho reward is ready to claim (one of its races already has the Digitalization Level), dashed when the game data doesn't say (the Black Frost exchanges); hover one for how to get it. They come from your valuables and completed quests, read from the game at each snapshot, and must add up to the cap the game holds: what doesn't is shown as not matched. NewMoon's raises show only when the game is on NewMoon (the game folder's `VersionData.txt` server list and the server the client is connected to). Where a rank lets you learn only one of several skills (Pursuit class 1: Strength, Magic, Intelligence or Speed), only the one you learned counts, read from your learned skills at the snapshot; until the game has been read once, those ranks say so and add nothing. A skill's effects come from the game's tables; where the server's older effect data for a skill (the public server's) disagrees with the numbers the game's own text for it lists (Lord of Victory says "Close-Range Damage +15%, FCC +5%, Healing received -25%", the old data had +25% and Critical +100), the page leaves the old effect out and shows the game's text, marked "from the game's description". A skill whose effect only works while a switch skill is on (Cataclysmic Mind's Spell Damage +500% needs Radiant Hands) follows that switch as the game reports it: counted while it is on, listed with the reason and not counted while it is off, and the switch's own row says which skill's effect needs it. A switch at the top right turns the page into the **Expertise planner**: every expertise, starting from your ranks as last read, each on a slider ("Expand all" shows what every rank unlocks) (with what its ranks unlock), the points against your cap (set another cap to plan for more expansions), and beside it what the plan changes: the skills it gains or loses and every number that moves, worked out by the same code as the Build Planner. The plan is kept per character until you reset it; it never changes the game or your data.
- **Assistant:** Black Frost stands in the window's bottom right corner, drawn in 3D from the game's own model with the
  game's own animations (the app reads them from your game folder the first time, so the game must have been running
  once with the app open). It idles, does something by itself now and then, plays a trick when you click it, casts
  while a snapshot is being read and cheers or flinches at the result. The app's messages are what it says: speech
  bubbles above its head. Hold the right mouse button on it and drag sideways to turn it around (it stays the way you
  leave it). Settings > Assistant hides it and brings it back.
- **Characters:** the name is read from the running game, nothing to type. With one character in game the app shows it;
  with two clients open it asks which one. Each character keeps its own folder and spreadsheet (`data\characters.json`);
  click the name in the header to switch.
- `crates\imagine-tracker\src\main_form.rs` the window (WebView2) and the handful of requests the page can make.
- `app\ui\` the page: `index.html`, `app.css`, `app.js` (plain JavaScript, no libraries, works offline).
- `crates\imagine\src\build\pipeline.rs` the snapshot / rebuild steps shared by the app and the command line.
- Preferences (character, CSV path, theme, window position) live in `%AppData%\BABEL\settings.json`.
- Theme: by default the app follows Windows' light/dark app setting, also when it changes while the app is open
  (Settings can pin Light or Dark). The window sets the page's colour scheme (`set_scheme` in `host.rs`); the page
  only asks `prefers-color-scheme`. `--capture out.png --then-scheme light|dark` checks the live switch.

Working on the app's page:
- `BABEL.exe --dev` loads the page straight from `app\ui`, so page edits need no rebuild (F12 opens DevTools).
- `BABEL.exe --capture out.png [--page gear] [--theme dark|light] [--mode digi] [--size 1280x860]` saves a
  picture of the page and exits.
- In a plain browser: `imagine-reader.exe serve --port 18765 [--data <copy of the data folder>]`, then
  `http://localhost:18765/app/ui/index.html` (the same page, with the real calculation behind the Build Planner).
  A stand-in for the app reads `data\dash`. Add `?empty=1` (first run), `?snap=fail` or `?snap=live` (snapshot results),
  `?game=1`, `?page=…`, `?theme=…`, `?mode=digi`.
- The assistant: `--capture` pictures leave it out unless the run adds `--query assistant=1`. In a plain browser it
  needs the model file first (`imagine-reader.exe model`, below); `?assistant=0` leaves it out, `?assistant=none`
  stands in for "the game's folder isn't known yet".
- Feature flags: features still being tried out are off unless the run names them, `?features=a,b` in a browser,
  `BABEL.exe --query features=a,b` for the app (nothing is saved; a plain start has them off). The list is at the top
  of `app.js` (`FEATURES`):
  - `assistant-picker`: Settings > Assistant gets "Same as my summoned demon" (on by default: the demon summoned in game stands there, read from the game every 5 seconds, read-only, about 30 ms; with none out, the last one stays) and a demon picker (the Demon Database's base demons and their Inexperienced variants, those with a picture; picking one switches the following off). Changing demon plays a summoning effect made of the game's own pieces (its effect models' pictures, smoke and sparkles, and its summoning sound System_syoukan.wav, tinted with the demon's CModelData colour; the timing is the page's own, the game's effect\eff files are not decoded). Every demon is drawn at one size, set so Black Frost looks as he always did, times the game's own size for the model (capped), standing on the box's floor; blended parts and layers the game fades (Alilat's glyph shell, drawn as added light) are drawn as such. The picked demon's
    model is read from the game's files when it is picked (about 0.1 s; kept in memory only, not in
    `data\cache\models`) and summoned in place of the one standing there. Which model a demon has comes from
    `reference\devil-models.tsv` (the demon table's model id looked up in the client's model table, the Shield copy CModelData2.sbin: the older BinaryData\Client copy lacks 181 demons' models). In a plain
    browser a demon needs its model file first: `imagine-reader.exe model <model>` (it writes `data\cache\models\<model>.json`).

### Tools: Demon Ranking

Under **Tools** in the left bar, before the DCM Locator. The Demon Database's demons in one table with their STR, MAG,
VIT, INT, SPD, LUCK, Max HP, Max MP, CLS-R, LNG-R, Spell, Support and Critical (P.Def and M.Def are left out); click a
column heading to rank by it (again for the other way), click a demon to open it in the Demon Database. Compare them each
at their base level, all at level 1 (the server's formula at level 1, below where any demon starts, so they compare on
equal footing) or all at level 99. "Mitama-fused" uses the Mitama form's stats where a demon has one, marked with the
game's Mitama sign on the face's top left corner, as on Overview's summoned demon. Shows the demons that can be
contracted (284), every one you can get some way (contract, normal fusion, a DCM, an exchange item, a special fusion or
an event gift; the page opens on this one), or all 951. The variants (Lucky, Inexperienced, Open-minded Ishtar and the
like) are listed next to their demon, each with its own stats. Stats are a fresh
demon's (no Demon Force, rebirth or Mitama runes). Critical is the demon's own value from the game data: it does not
grow with level, and a critical roll adds Luck to it.

### Tools: DCM Locator

Under **Tools** in the left bar. Lists every DCM (Demon Compendium Memory) and where the game data says it comes from:
shops with prices, enemies with drop rates and zones, chests, event or NPC rewards, NPC exchanges with what they cost,
random boxes, and presents from the demon itself. Search by demon, enemy, zone or shop; the chips filter by kind of source. It works without a snapshot.
Each row shows the demon's own icon from your game client when the client has one.

The DCM list, zone names, shop stock and NPC exchanges are the IMAGINE client's. Drops, chests, presents, events and
which NPC offers an exchange come from the public server data, so IMAGINE may have changed them; DCMs with "No known source" are most likely IMAGINE additions.
To record what players confirmed on IMAGINE, add notes to `app\ui\dcm-notes.json`
(`"notes": { "<item id>": ["Sold by X in Y for Z"] }`; the item id is shown at the bottom of an opened row).
They appear first, marked "IMAGINE note".

### Tools: Key Items

Under **Tools** in the left bar. Lists the game's key items (the 282 it shows; its hidden progress flags are left out):
which ones your character has, what each one does and how to get it. The game names most key items only in Japanese:
each shows its English name, from the game's own English text where there is one, else a translation marked
"translated", with the Japanese name greyed under it. Chips filter by kind (modes and access such as WILDCAT and
BEARCAT, licenses, COMP and system, shops and people, expertise cap, story, quest items, records, Electronic Corridor
programs, seasonal) and by **You have** / **Missing**. Click one for how to get it, what it does in full and the quests
involved, each with a link to it in the Quest Database. Which ones you have is read from the game every few seconds while
the page is open (logged in as the character shown), and the last reading is kept, so the page still says it with the
game closed. How to get them and what they do come from the game files and the public server's event files, so NewMoon
may have changed some; NewMoon's own key items are marked NewMoon and described from the game's own text.

### Tools: Quest Database

Under **Tools** in the left bar, next to the other databases. Lists every quest (680; 665 without the inactive ones)
and, when opened, what to do step by step: who to talk to and where (zone and map grid position), which dialogue
choices to pick, what you need, what you get and give, and the kill or collect counts. For each question the answer to
pick is shown with the other answers that work as well and the ones that do not move the quest on, worded as the game
words them now (since the update of 19 November several answers are in Japanese in game, and the guide shows them that
way). When a step needs several talks (Act 1: Demonization: Kiara, Nora, Lorie and Izo) all of them are listed, with
their answers and whether the order matters. When your answers decide the outcome (Uriel in Act 13: Envoy for the
Messian Church moves your alignment to a different place for each line of answers, and one of them also gives you
Inexperienced Uriel) the talk is shown as a tree of answers, each branch with what it gives and what it does to your
alignment. Alignment changes say whether they add to it, set it, or move it part of the way to a value, and which side
that is (Law −5,000 or lower, Chaos +5,000 or higher); a change that depends on your current alignment says so.
People show their picture before their name (Quest Database, Quest Tracker and the panel): taken from their 3D model
in the client (for most humans, their skeleton dressed in the face, eyes, hair, clothes and accessories the game lists
for them), or, for someone who appears as a demon (Uriel, Surt), that demon's Demon Database picture. All but one of
the people the quests name have one.
When a step's person can only be reached a certain way, it says what comes first: who takes you there (Act 3: Curse of
the Gods: talk to DB Saito in TMG Building, "Go to the Heliport"), and what makes the person appear (walk to the spot,
answer, defeat DB Kisugi; then he is there to talk to), also for a step that ends at a spot (the elevator attendant who
takes you down to the Docks floor). A step that waits for several things done in any order lists them, quest objects
too (Act 6: The Mysterious Girl: the sword blade in each of four dungeons, each after that dungeon's last fight, then
Celu Tower), and people the game shows only during a step whom the quest does not need are listed as optional (Act 6:
"???" in Shibuya, who leaves once you have talked to her). Seasonal and tie-in event quests (Christmas Devil, May You
Never Die, ...) have their steps too, marked with the event they belong to. Where a step can be done in more than one way (for example a
Law or a Chaos answer) every way is shown with what it changes. Search by quest, person, place or item; the chips
filter by kind of quest.

The quest names, objective text and counts are the game's own. The steps come from the public server's event files
(658 quests have them; the other 22 show the game's objective text only), so IMAGINE may have changed them. Valuables
(key items) are shown with the name in the game data, which is Japanese.

### Tools: Quest Tracker

Under **Tools** in the left bar. With the game running and the character logged in, it follows your own quests (read
from the game every few seconds while the page is open; only read, never changed): one card for each quest in
progress, those with something to do in the zone you are in first (marked "here" and opened). A card shows the step
you are on (as a row of steps), and opened: the objective, your kill counts as bars (7/10), and everything about the
step from the Quest Database (who to talk to, the answers, what comes first), with the people in your zone marked "In
your zone" and the talks of a multi-talk step you have done marked "Done". Closed, it says in one line what comes
next. "Open in Quest Database" shows the whole quest. The status line says how many quests are in progress, which
zone you are in, and how many you have finished (once the game has that list, which is after Quest Completion Bonuses,
Menu > Quest > View Quest Bonuses, has been opened once). Hide quests you don't want to see with the crossed-out eye:
they move to the **Hidden** section at the top, where the eye shows them again.

**The quest panel** keeps the next step in front of you while you play, with no alt-tabbing: a small window that stays
above the game. Open it from the Quest Tracker ("Open the panel over the game"); it starts at the top right of the
game window and can be dragged anywhere. It lists the same quests as the tracker (the hidden ones hidden too), one
compact line each; click a quest to open or close it. An open quest shows the step you are on, who to talk to or where
to go, the answers to pick, what comes first, and your kill counts. It updates every few seconds from the game, also
while BABEL's own window is minimised, and never touches the game (IMAGINE has to run in a window or borderless:
nothing can be shown over exclusive full screen).
### Tools: Item Database

Under **Tools** in the left bar. Every item in the game (17,520, plus 158 key items), filtered by kind: gear,
consumables, materials, vouchers and boxes, tarot and soul crystals, demon equipment, appearance items, key items and
the rest. A second row of chips narrows a kind down: the slot for gear, the game's own item class for everything else
(Weapon Mod Tool, Demon Voucher, Junk...). The **Filters** button opens more chips, each with its count: who can use
it (sex, alignment), how it can be got (shop, exchange, enemy drop, chest, ticket or box, event, quest...; pick several),
a weapon's range and affinity, and a level limit. Click a column header to sort by it.

- **Gear** (10,266 pieces) shows what it gives: stats, effects, appearance bonus, the sets it belongs to, mod slots,
  and who can wear it, with community notes from SnowSheet's gear reference for IMAGINE beside it.
- **Other items** show the game's own description, who can use them, the stack size and the selling price. A tarot or
  soul crystal shows what each side gives when fused into a piece. A voucher, ticket or random box lists what it can
  give, when the server data links the box to the item that opens it. A fusion plugin says which quest or exchange gives it.

Where an item comes from (shop, exchange, enemy drop, chest, the ticket a piece comes in) is traced through the game data
for every item, and an item handed out by a quest names the quest, with a link to it in the Quest Database. Search with several words at once ("limit break celu", "revival demon"), and untick "Only items with a
known source" to see everything.

Stats, effects and descriptions are read from the IMAGINE client's current item tables, not from the spreadsheets.
369 spreadsheet rows matched no item in the game data (different spelling); they are listed with the sheet's own text,
marked "sheet only". The cosmetic ticket tabs (Vanities) are left out.

Each item shows the icon the game shows for it (17,382 of 17,520; 138 have no picture in the client; key items have none).
The pictures are the game's own art, so they are not in the repository: they are made from your game folder by
`imagine-reader geardb ... --client <game folder>`, into `app\ui\gear-db-icons\`. Without
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

Demons, levels, races and special fusions are the IMAGINE client's. The race chart and the tri-fusion and elemental
rules are the comp_hack server's fusion code, which IMAGINE may have changed. Checked in game (2026-11-23): two-demon fusions, a tri-fusion and the refusal of Mitama-fused demons, all as the planner says.

### Tools: Digitalize Planner

Under **Tools** in the left bar. Pick what you want from Digitalize and it works out which races to raise, to which
Digitalization Level, in what order, and how many Digitalize points (DP) that takes:

- **Your goals**: the boosts you are after: the always-on passives the levels teach, for you or for your demon
  (stats like "Intelligence +55", affinities, the Limit Break cap). Search by effect; "demon" or "you" narrows it to one
  side, and each match says what it costs from where you are, cheapest first. Tick as many as you like: the list stays
  open with your search, **Add all shown** ticks every match you don't have yet, and the plan updates as you tick. Each
  goal card shows its race as three of its demons in small round pictures. Digitalize's own choices (the assists:
  duration, cooldown, the demon's stat share and skills, Demon Force, Mitama sets) are not goals. A race level itself
  can be aimed at from the race's table (**Plan to here**), and a locked race from the search: type its name (or "race") and tick **Unlock <race>**; the list says what each race needs and its DP from where you are, and the goal card ticks the needs you already have. A demon can be searched by name too: type it and tick **Digitalize <demon>** to plan what it takes to be allowed to digitalize it (the list says its race and what lets it in). Goals are kept between visits. With no goals yet, the page
  offers six to click: the strongest boost of each kind you don't have, from the race that costs you the least.
- **The plan**: the cheapest set of race levels that meets every goal. On top: the DP in all, the races to raise, and
  one bar with each step's share of the DP. Then the steps down a numbered line, in an order that works; a step's
  card, its number, the line down to the next step and its part of the bar are in the colour of its race's side (Law,
  Neutral, Chaos). Each step says the
  DP, which demons you can raise that race with, the boosts you get on the way and the races it opens. A step that
  goes past DLv 5 is tagged **Mitama-fused demon**: past 5 a race only rises with one.
- **Races**: every race by side (Law, Neutral, Chaos) and tier, with your level (the part you are working on is
  filled as far as your points go), what the plan adds (pink) and what opens it (a tick marks a need you already
  meet). A locked race is dashed, with its pictures greyed. Click a race for its ten levels: DP per level, demons,
  boost, assist choices, and **Plan to here**; the level you are at has a dot and the plan's levels are tinted.

**From my levels** plans from the levels and points of the last snapshot; **From scratch** from the races everyone
starts with. Tiers are how many DP a race's levels take: tier 1 is 70.2M DP from DLv 1 to 10, tier 5 five times that.

The planner knows the catches: Entity and Fiend start open but no level of theirs opens a demon, so to raise them you
need a demon another race opens (Fiend: David at Fiend 1; Entity: Urthona, Urizen, Tharmas or Luvah at Earth Element
or Elemental 8, and others).

Races, unlocks, DP per level, demons, passives and assists: the server's Digitalization tables (`reference\guardian-*.tsv`),
with the client's own names and texts. The rules (DP go to the race of the demon you are digitalized with; a race opens
when all its needs are met; DLv 10 at most, 5 without a Mitama-fused demon) are the comp_hack server's code, which
NewMoon may have changed. Not checked in game yet. The page data is `app\ui\digi-planner.json`, made by
`imagine-reader digiplanner` from the reference tables.

### Tools: Rebirth Planner

Under **Tools** in the left bar. Plan a demon's rebirths and its Mitama before you spend anything:

- **The demon**: any demon from the Demon Database that the game can Mitama-fuse and that you can get some way (a
  contract, a fusion, a DCM, an exchange or a gift; fresh: no ranks), or **My COMP**: one of your own demons as BABEL
  last read it, with its real rebirth ranks, Mitama and slots. Set the level the stats are shown at (93 by default) and
  your XP bonus from gear and incense (it lowers the apples). The runes wear the game's own glyphs and the Mitamas
  their own pictures, cut from your game folder like the other pages' pictures.
- **Rebirths before the Mitama fusion**: one row per rune, like the game's Rebirth tab: its rank now, a bar of nine
  pills (click a pill to make that rank the target, or use − +), the bonus that rank gives (Tiwaz: Close-Range +4 per
  rank, Sowilo: every stat +3, Wyrd: Max HP +40 and MP +10, Ingwaz: cheaper summons...), the growth type it reaches, and
  a flag to pick the path the demon should **end on** (its stats follow that growth type). **All to 4 / 6 / 8** sets
  every rune at once. Under the rows: how many rebirths that is, the Magical Golden Apples the levels take at Vivian,
  the macca, the rank points toward the Mitama fusion (48 needed, and the Mitama rank past it) and what the ranks are
  worth as conversion points. **The steps and items** lists every rebirth in an order the game accepts (the rune the
  demon is on first, since its own growth can skip rank 1; each rune fully before the next; the end path last) and the
  cost items taking the first choice of each rank (every rank accepts one of several: runestones, incense, soul stones,
  a Primordial Runestone).
- **Mitama fusion**: a bar of the rank points toward 48; tick the fusion once it is reached, pick the Mitama (Ara, Nigi,
  Kusi or Saki, each card showing the stats its Mitama rank bonus raises), and whether you have **Mitama Demon
  Enhancement - Iron Man** (stronger sets).
- **Rebirths after the fusion**: the same rows again from zero (a Mitama-fused demon gets 60% less XP, so 2.5 x the
  apples).
- **Mitama slots**: the four kinds' caps on top (how many of each the Mitama rank allows), then per rune its eight slots
  in the kinds' colours (solid = already on the rune, striped = to add), a − + counter per kind, and the rune's four set
  bonuses as chips with their requirement per kind: a chip lights up when the set is on, and clicking one adds the
  Mitamas it still needs. Under the rows: the Mitamas to place (as Summon PGs from Rag's gem exchange, with the gems),
  the macca, and the random bonus each placed Mitama can give. A COMP demon's Mitamas already on a rune count and are
  never removed.
- On the right, always in view: **What it costs** (rebirths, apples, macca and Mitamas in all, then per phase) and
  **Stats at the level**: the demon's 14 stats as a fresh demon beside the planned one, with the change, the summon
  magnetite cut and the active sets. **Reset the plan** starts over.
- **Pick boosts first** (the other way round, an experiment): tick the set boosts you want from the 48 (search by
  effect; Iron Man shows the stronger texts) and the page works out the rest: how many Mitamas of each kind, the Mitama
  rank that allows them, the rank points the fusion then needs and the cheapest ranks that give them, each rune's rank
  after the fusion, the costs and the stats. It flags boosts that can't share a rune (more than 8 Mitamas together).
  **Open in the rune planner** hands the result over to fine-tune it.

The growth types, rank costs, Mitama bonuses, sets and PG trades are the server's tables and the client's texts. The rules
(levels per rank, macca, one cost item, the 48 points, 12 + Mitama rank of each kind, the stat formulas) are the
comp_hack server's code, and 10,000 XP per apple is the public server data's Vivian: NewMoon may have changed any of
them. Not checked in game yet. Demon Force and familiarity are left out. The page data is `app\ui\rebirth-planner.json`,
made by `imagine-reader rebirthplanner` from the reference tables.
### Tools: Dungeon Database

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
(Dark Babel) says so: IMAGINE sets those drops on its own server. All rates are the vanilla comp_hack server data's
(your local server's), before luck and drop bonuses; one short warning banner above the drops says so.

Places are grouped by where you enter them from (all of Dark Babel's floors under "Entered from Dark Babel", Celu
Tower's Bronze, Silver and Gold under "Entered from Celu Tower"); click a group to open it. Fields, other places and
the few dungeons whose entrance the data doesn't give come last. Untick "Group by entrance" for one flat list.
Each place shows small pictures of its enemies beside its name (up to 8, bosses first with a purple ring, then the
strongest; "+N" for the rest); hover one for its name and level.
Search a place, an enemy, a skill, a drop or an entrance; the chips filter dungeons, fields and other places. A search
opens every group it finds something in, and searching an enemy lists only the matching enemies inside each place.

Names, stats, skills and traits are the IMAGINE client's. Which enemies are where, their levels, whether they talk and
their drops come from the public server data, so IMAGINE may have changed them. Stats are the server's formula for an
enemy at that level (with the extra HP the game gives enemies), before its passive skills and traits. Not checked in
game.

### Tools: Dungeon Guides

Under **Tools** in the left bar. The dungeons that aren't just "go in and kill everything" (122 of the Dungeon
Database's 175): a hidden, extra or True boss you only get by doing something, demons that must be summoned at the
right moment, objects to click, choices, random rolls, timers and score runs. Click a dungeon for each outcome it can
end in (for example "Yamata no Orochi is the final boss") and the numbered steps to get it: what to do, **Where**, what
it **Needs**, and what **Breaks** it. The two Diaspora runs also list what to bring and the item recipes.

Search a dungeon, boss, demon, item or mechanic; the chips filter by three hand-picked tags: **Hidden boss** (a boss
you only get by doing something on purpose), **Summon a demon** (a specific demon must be out at some step) and
**Minigame** (an event run scored by points, waves, a race or a hunt). Hover a tag for what it means. A dungeon with several outcomes lists them as rows; click one for its steps. Each dungeon's button opens its enemies in the Dungeon Database. The 53 linear dungeons are listed at the
bottom.

Read from the public server's event scripts, so IMAGINE may have changed some; nothing was checked in game. Steps
marked **inferred** were worked out from how the script fits together rather than stated by it.

### Tools: Demon Database

Under **Tools** in the left bar. Every demon you can own (the compendium's 359 and 592 variants such as Open-minded Ishtar) with its picture, race, base level and
alignment. Click a demon and its details open on **Where to get it**: the places it can be contracted with their
levels, its DCM (with a button to the DCM Locator), whether it can be fused (how many two-demon recipes, tri-fusions and
special fusions make it, the cheapest recipe, and a button that opens it in the Fusion Planner's "Find a recipe"), each
special fusion with the plugin it needs, each plugin once with where to get it (quest, exchange; a quest has a button
that opens it in the Quest Guide), and, for a variant,
the exchange item (a "Summon PG" or a voucher) that gives it with where the item comes from, or its event gift; where the
data does not say, the page says so. Every list of ways says that any one of them is enough. Below that: its stats at any level from its base level to 99 (a fresh demon: no Demon Force, rebirth
or Mitama), its affinities (resistance in %, weaknesses in red, natural null / reflect / drain chances), its features and its
Mitama features (what it gains, or swaps, when Mitama-fused),
its skills (what it knows and the level it learns each one at; hover for the game's description), what its tarot and
soul give and on which gear. "Normal" / "Mitama-fused" above its stats switches its stats,
affinities, features and skills to its Mitama-fused form (340 demons have one).

The list opens on the demons that can be contracted somewhere (284); "Not contracted anywhere" and "All demons" show
the rest (the variants an item, a fusion or an event gives are there). A variant is a demon type with its own name,
level and stats that the game gives out. Dungeon and boss copies of a demon are not listed on their own: the places they appear, where they cannot be
talked into a contract, are under "Enemy only". Search by name, race, skill, feature, tarot text or zone; filter by
race and alignment (Law, Neutral, Chaos); sort by level, name or race. Untick "Show variants" to list only the
compendium's demons (the page remembers the choice). Demons, stats, skills, tarot and soul are the IMAGINE client's; spawns are the
public server data, so IMAGINE may have changed them.

### Tools: Skill Database

Under **Tools** in the left bar. Every skill in the game, separated by action type: Attack, Spin, Rush, Shot, Rapid,
Guard, Counter, Dodge, Talk, Threaten, Provoke and Curative (the game's own names and order, from its Help), then
Passive and Switch skills, which have no action type. Each row has the skill's picture, element, cost (MP, HP,
bullets or items), cast time, cooldown, modifier and stiffness (the two numbers the game's description ends with) and who can get it. Pick a type at the top to
see only those skills and what the game's Help says about that type (what it beats and what beats it). Click a skill
for its in-game description, its target, reach and the stat it uses, the expertise rank that teaches it, the NPC
exchanges (barter) that teach it and what they cost ("Skill: Demon" ones teach your summoned demon), and every demon
that learns it (with the level) or has it as a feature; click a demon to open it in the Demon Database.

The list opens on the skills you can get (learned by a compendium demon, a demon feature, from expertise, from an NPC
exchange, or a fusion skill: 2,034). "Who can get it" narrows that down or shows every skill in the game data (3,120, with enemy and event
skills). Search by name, element, words of the description, expertise or demon; filter by element; sort by name,
modifier, stiffness or cooldown. Item uses, debug skills and the 777 skills the client only names in Japanese are left out.
Destruction magic (Agi, Zio...) is a Shot skill in the game data; healing, support and curse spells are Curative.

### Tools: Build Planner

Under **Tools** in the left bar. Try other gear and expertise ranks and watch your stats change, without touching the
game or your saved data. It starts from your character as last read (take a snapshot first).

- **Gear**: open a slot to change any part of the piece, numbered as in the game's fusion window: Slot 1 (the piece it
  counts as: set bonus and appearance bonus), Slot 2 (whose basic features it carries), Slot 3 (whose characteristics),
  the tarot, the soul fusion, and the fusion bonus (the LIMIT switch: every fusion bonus of that kind of piece at MAX,
  +45 on a weapon). "Replace the whole piece" changes Slots 1 to 3 at once; a piece can also be taken off. Mods stay as
  they are.
- **Picking**: the list shows what each choice gives for that part. Search a name or an effect ("rapid", "limit break
  power"): searching an effect puts the pieces that give the most of it first. A stat works by the names the app shows
  (STR, MAG, CLS-R, LNG-R, P.Def, M.Def, HP, ...) or in full ("Strength", "close range"): it lists the pieces that raise
  that stat itself first, most first (MAG is the Magic stat, not Magic Damage); the line it found is in bold, and a
  piece whose set bonus gives it comes after the pieces that give it themselves. The highlighted row (arrow keys or the
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
- **Builds**: the Build Planner opens on your builds: a "New build" card and one card per saved build (its name, how many
  changes from what you wear, the pieces changed and the biggest differences). In a new build, "Save build…" gives
  it a name; from then on it keeps every change you make to it. A build can be renamed (click its name), duplicated
  and deleted. Saved builds are also listed under **Build Planner** in the left bar; the small arrow there folds the list.
  A build is a whole plan: when your gear changes, it is compared with your new gear. Each card names its base.
  On the left of a card are the build's pieces as pictures, one box per slot; a piece that differs from what the base
  character wears has a purple ring. Hovering a picture shows the piece's tooltip, with the picture of what each
  section comes from (the piece lending the basic features or the characteristics, the crystals, a set's pieces; the
  Gear page's tooltips have them too). For a ringed piece, a section the build changes shows what the character wears
  and, under it, a "Changes to" card with what the build has there.
- **Sharing builds**: the share button on a build (its card, or beside its name in the editor), or **Export build** beside
  your name on Overview (what you wear now, without opening the Build Planner), gives a short text code
  (`BABEL1-…`, about 130 to 300 characters) to paste to another player. They press "Import build" on the builds page,
  paste it, see the pieces and expertise ranks it holds, choose what to show it on (one of their characters, or "No
  base" at the level and alignment it was shared at) and save it as a new build of their own. A code holds numbers
  only (the pieces with their fusions and mods, the expertise points, the point cap, a level and an alignment): not
  the character's name, the build's name or its digitalize demon. A pasted code is treated as a stranger's input: the
  reader accepts only codes it would write itself, checks every number against the game data (a piece of the slot's
  kind, a crystal that kind of piece takes, an expertise a character can raise), never puts a number from the code
  into a path or a command, and importing only ever adds a build. The format is in
  `crates\imagine\src\build\share_code.rs`. The page also has a Content-Security-Policy (index.html): it runs only its
  own script files and loads nothing from the internet.
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
  `imagine-reader.exe doctor` (add `--json` for a machine-readable version).
- **The log** is in `data\logs` beside the program (**Show logs** opens the folder). A few small text files, 5 MB at most
  altogether, the newest kept. It says what the app did and how long it took, which parts of the game were found, and any
  error. It never holds what the page asked for (that can be personal), spreadsheet rows, or anything read from the game's
  memory. `--log-level debug|info|warn` (on `BABEL.exe` and `imagine-reader.exe`) changes how much goes in;
  `imagine-reader.exe --version` says which build you have.
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
target\release\imagine-reader.exe clients    # which clients are running and who is logged in to each
target\release\imagine-reader.exe doctor     # the self-check: files, game build, each reader found? (pass / warn / fail; --json for machines)
target\release\imagine-reader.exe --version  # which build this is: version, commit, build date
target\release\imagine-reader.exe snapshot   # read the game, add a row to the CSV, rebuild page data
target\release\imagine-reader.exe rebuild    # rebuild page data from saved files (game not needed)
target\release\imagine-reader.exe recalc     # recompute every CSV row's gear bonuses, then rebuild
target\release\imagine-reader.exe gear-names # name every CSV row's gear the game's way (copies the CSV first), then rebuild
target\release\imagine-reader.exe sets       # list set and appearance bonuses for the worn gear
target\release\imagine-reader.exe simulate   # check the Build Planner: the build as read must give back the readings and the saved bonus list
target\release\imagine-reader.exe simulate --spec build.json   # the result of another build, as JSON (--init: the build as read; --catalog N: what fits slot N)
target\release\imagine-reader.exe serve --port 18765           # the app's page in a browser, with a working Build Planner (for testing)
target\release\imagine-reader.exe alignment       # the character's alignment, read from the game (LAW/NEUTRAL/CHAOS overrides until the next snapshot)
target\release\imagine-reader.exe demon           # list the COMP demons
target\release\imagine-reader.exe demon Specter   # one demon in detail and what digitalizing with it gives
target\release\imagine-reader.exe demon Specter --use   # make it the digitalize demon the page adds
target\release\imagine-reader.exe digilevels      # Digitalization Level and % per race, assists, level-unlock skills
target\release\imagine-reader.exe damage-log      # the game's damage log: your and your demon's hits (Technical Attack, Pursuit) and totals
target\release\imagine-reader.exe damage-log --watch 60   # follow new hits for 60 s, then totals, damage per second and hits per second
target\release\imagine-reader.exe expertise       # re-read expertise ranks and Digitalization Levels, rebuild (no CSV row)
target\release\imagine-reader.exe toggles         # which toggle skills are on, read from the game ("Name, ..." overrides when it isn't running)
target\release\imagine-reader.exe labels          # regenerate reference\stat-labels-game.tsv: the game's own name for each stat (from the client's effect text)
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
Closed, its line has the icons of the skills in its slots. "Details" opens two columns: on the left who it is, the stats
it adds, its skills and its features; on the right its Digitalize boosts, one row per level: the digitalize window's
picture of the boost (a lock where none is learned yet) and the icons of that level's Digitalization passives for the
demon's race. Each skill has its icon, what it gives you, and the game's description on hover. The game has no picture
for a feature itself, so features show the digitalize window's Features boost picture; a demon equipment shows the
picture of the Epitaph Parts item it comes from. A skill you don't get while digitalized is dimmed
(for example, its features when the race's Features boost isn't learned).

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
`data\cache\demon-icons\`), so the picker also works without the game. Those cached demons have a bin on their card
that takes them off the list; demons still in your COMP, the summoned one and your digitalize demon have none (they
would come back at the next read). In the app, picking never reads the game: it uses the demons and the learned
assists last read (by a snapshot, or by "Load demons from the game" in the dropdown). Picking one saves it, recalculates the latest snapshot's
demon columns with it (the CSV is copied to `<csv>.backup-before-demon-switch.csv` first) and rebuilds the page. Later
snapshots refresh it from the COMP, and one taken while digitalized switches to the demon in use. The demon that was summoned when the game was last read (a snapshot, or "Load demons from the game") shows as a small card to the right of your stats on the Overview. The Digitalized view then adds it: a demon line on the Overview (framed in its alignment colour, as the player picture is in theirs; its 32x32 game icon, converted from the
client's `Interface\tga` into `data\dash\demon.png` when the demon is saved; **Details** opens the stats it adds and its assists per level), a
third part in each key stat's breakdown, and
a demon column on Bonuses (rows only the demon gives appear in that view only). While not digitalized, the demon summoned beside you
gives you what its effects aim at its summoner (a Demon Force entry's "Long-Range +60", its passives' cooldown and cost cuts): they
appear on Bonuses as a column and parts named "<demon> (summoned)" and in the key stats' breakdown, and are left out of the digitalized
totals. Cooldown and Incantation also count what your own stats take off (the server's rule, comp_hack
`CalculateDependentStats`): 2.5% per 10 VIT (cooldown) or INT (incantation) plus 1.5% per 10 SPD, rounded down, on the
final stats of the shown mode (the last saved normal stats; while digitalized plus the demon's stats). They show as a
"Your stats" source on Bonuses and in the Overview's Cooldown breakdown; cooldown stops at 95% off and incantation at
100% off, past which the page tags the row as over the cap. The CSV gets "Digitalize demon",
"<stat> (demon, digitalized)" per key stat, and "Digitalize demon record" (so `recalc` can redo it). A snapshot taken
while digitalized doesn't take the window's 12 stats/HP/MP (they include the demon); the saved normal ones are used.

Options: `--name <character>` (default: whoever is logged in, or the only character with saved data),
`--csv <path>` and `--data <dir>` (default: that character's own, from `data\characters.json`), `--ignore-window`.
`--data <dir>` makes that folder the data folder for the whole run: its `config.json` and `characters.json` decide where
the CSV is (a character it does not list gets `<statsFolder>\<name>-stats.csv`, else `<dir>\stats`), never those of the
program's own data folder; `--csv` still wins.

The quest bonus and the Demonic Compendium bonus need two counts, which the game only has once their windows have
been opened: Quest Completion Bonuses (its completed-quest list) and Demonic Compendium (its "Demons: 60/339" text).
After that, each snapshot (or `extras`) reads them and saves them, and the saved counts are kept until the next
reading. The Expertise tab shows a "not counted" row for whichever is missing. Whether you own Demonic Compendium
Ver. 2.0 (no compendium bonus without it) is read from your valuables at each snapshot. They can also be set by hand:

```
imagine-reader extras --quests 32 --compendium 60      # --compendium-v2 no if you don't own Compendium v2; none = unset
target\release\imagine-reader.exe model           # the assistant's 3D model (Black Frost) from the game folder into data\cache\models, with a summary
target\release\imagine-reader.exe model --check "<game folder>\Devil\bone"   # which of the client's model/animation files the reader can read
```

Without options, `extras` shows the clan (read from the game), reads both counts when the game has them, and lists
every bonus they give.

## Where things come from

| What | Source |
|---|---|
| 12 stats, HP/MP | The own characters status object (always in memory; the CSV says "Character data"). While the Information window is open its block is used instead; it gives the same numbers |
| Equipped items | Gear grid (always in memory; newest complete grid wins) |
| Level | Nameplate record |
| Item bonuses (stats, characteristics, tarot, soul, mods) | Each worn item's record in memory (stats source item, special-effect source item, mods, fusion great-success bonuses, tarot, soul) worked out with the server data (`calc/item_effects.rs`). A part with no server data (e.g. Ogami's Crystal) uses the item's tooltip text (cached per item id); so do old snapshots without records |
| Set and appearance bonuses | The client's equipment-set table (one-piece entries are appearance bonuses), with effect numbers from the server data |
| Fusion (soul/tarot) set bonuses | Each slot's tarot/soul enchant ids (the worn item's own record; the gear grid as fallback) checked against the enchant-set table: active only when every enchant in the set is worn |
| Expertise ranks | The client's expertise tree; chain expertises use IMAGINE's chain formulas and rank skills (client data, `reference/expertise-*.tsv`). Toggle skills count only while switched on (read from the game: the learned-skill map's "on" flag; the Bonuses page and the Overview tag a toggle's lines "Toggle · on", say how much of a total the toggles give and when they were read) |
| Other learned passives | Every learned passive skill that no expertise rank or Digitalization Level gives (Additional HP/MP, "Additional" affinity skills, Quickness, clan skills, Quickdraw...), from the learned-skill map; shown as "Other learned skills" on the Expertise tab |
| Digitalization Levels | The character's progress data in memory (level and points per race); each level's "extend" passive skill counts like an expertise passive (one group per race on the Digitalize page, which also shows the digitalize demon) |
| The assistant (Black Frost) | The client's own 3D files: `Devil\bone\D2_11_003a_Base.nif` (model, bones, texture) and the 15 animations its `.kfm` lists (Jack Frost's `.kf` files, which the game itself uses for Black Frost). Read by `data/nif.rs`, turned into one file for the page by `build/demon_model.rs` |
| COMP demons (`demon`) | Each demon's records in memory (`game/demon_reader.rs`): level, growth type, rebirth ranks, Mitama slots, skill slots, learned skills; base values, growth, traits and Mitama tables from the server's demon tables (`reference\`) |

Layouts are documented in `crates/imagine/src/game/client_reader.rs`.

## Files

- `crates/` the program (Rust): `imagine/` the reader library (`memory`, `game`, `data`, `calc`, `build`), `imagine-reader/` the command line, `imagine-tracker/` the desktop app (window, WebView2, page bridge).
- `data/cache/` last stats, tooltip text per item, the set table, alignment. `data/cache/models/` the assistant's model,
  made from the game's files (the game's art: not in the repository).
- `app/ui/assistant.js` draws the assistant (plain WebGL, no library).
- `reference/devil-models.tsv` each demon's 3D model (the demon table's model id in the client's model table, Shield\CModelData2.sbin: file name, size, summoning circle size), for the assistant picker.
- `reference/enchants-game.tsv` the client's own enchant table; `reference/devils-game.tsv`, `devil-growth.tsv`,
  `mitama-*.tsv`, `guardian-assist.tsv`, `guardian-levels.tsv`, `skills-digitize.tsv` the server's demon and digitalize tables;
  `tokusei-*.tsv`, `expertise-imagine.tsv`, `traits-unresolved.tsv` IMAGINE data from the client.
- `data/tooltips.txt` also saves each slot's item record (`(item …)` in the header) and the level; the CSV's
  "Gear records" column does the same per row, so `rebuild` and `recalc` work from ids too.
- `data/dash/` page data: `snapshots.json`, `gear.json`, `expertise.json`, `bonuses.json`. `skill-icons/` holds the skill icons the Expertise and Digitalize pages show, converted from the game client's own icon files.
- Machine-specific paths live in `data/config.json` (created with detected defaults on first run; edit when something is elsewhere): `websiteContent` (the website's content folder), `tokuseiXml` (comp_hack's `datastore/data/tokusei`), `serverConstants` (the local server's `config/constants.xml`: digitalize rates, Mitama set boost skill), `statsFolder` (where new characters' CSVs go), `defaultCharacter`.
  The first three are optional: `reference/bundled/` holds copies of the tokusei XML, `constants.xml` and the website
  files the reader uses, compiled into the exe, so the portable release gets the same numbers. A folder set here and
  present wins over the copy. Refresh the copies with `tools/refresh-bundled-data.ps1` when the server's data changes,
  then rebuild (`reference/bundled/README.md` says where each one comes from).
