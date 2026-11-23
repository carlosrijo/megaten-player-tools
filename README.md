# newmoon-reader

Read-only snapshots of a NewMoon character's stats and gear from the running ImagineClient,
plus the data files for the "Rubito Build Tracker" page.

It only ever opens the game process with read access (`PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`):
it never writes to the game or changes anything.

## Desktop app

```
dotnet build app -c Release
app\bin\Release\net10.0-windows\NewMoonTracker.exe
```

After a change, with the app possibly open: `pwsh -NoProfile -File tools\rebuild-app.ps1` closes it, builds the reader
and the app, and opens it again. (A build with the app open only updates the page files, not the program: Windows
locks those.)

"NewMoon Build Tracker": one window with Overview, Bonuses, Gear, Expertise and History, a digitalize demon picker on the Overview ("No demon" is the normal view, a demon is the digitalized one, for every page),
and a **Take snapshot** button (F5) that runs the same read-only snapshot as the command line and then refreshes the page.
It shows `data\dash\*.json`, so it opens with the last saved data even when the game is closed.

- **Overview:** beside your picture, Max HP, Max MP and the 12 stats as compact bars (base, bonus and, in the
  Digitalized view, what the digitalize demon adds). Then one line per key stat (a bar only where the stat has a cap,
  showing how close you are; it changes colour past the cap). Click a line to see what gear, expertise and the
  digitalize demon each add, and to jump to every source on Bonuses. Under it, the 5 biggest changes since an earlier
  snapshot ("Show all" lists the rest); beside it, one radar chart per element with your affinity boost,
  the cap it stops at (100%, or more for an element a passive raises: Ice Limit Mastery Lv1 makes Ice 150%) and your resistance; the buttons under it bring one of the three forward and show its numbers. The digitalize demon's name is a dropdown to pick another one.
- **Damage per shot** (to the right of the stats): the expected damage of the character's main skill against the
  enemy its build is planned for, worked out by the gear analysis (`smt-gear-analysis`, the folder next to this repo
  or `gearAnalysis` in `data\config.json`) from this reader's data, by `tools\damage.ps1`. It is recalculated after a
  snapshot or a demon switch (about 5 s; the old number stays, dimmed, meanwhile). Only for characters the gear
  analysis has a folder for (`characters\<name>\flags.ps1`); hover the number for what it assumes.

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
  A stand-in for the app reads `data\dash`. Add `?empty=1` (first run), `?snap=fail` or `?snap=live` (snapshot results),
  `?game=1`, `?page=…`, `?theme=…`, `?mode=digi`.

## Commands

```
dotnet build -c Release
bin\Release\net10.0\newmoon-reader.exe clients    # which clients are running and who is logged in to each
bin\Release\net10.0\newmoon-reader.exe snapshot   # read the game, add a row to the CSV, rebuild page data
bin\Release\net10.0\newmoon-reader.exe rebuild    # rebuild page data from saved files (game not needed)
bin\Release\net10.0\newmoon-reader.exe recalc     # recompute every CSV row's gear bonuses, then rebuild
bin\Release\net10.0\newmoon-reader.exe sets       # list set and appearance bonuses for the worn gear
bin\Release\net10.0\newmoon-reader.exe alignment       # the character's alignment, read from the game (LAW/NEUTRAL/CHAOS overrides until the next snapshot)
bin\Release\net10.0\newmoon-reader.exe demon           # list the COMP demons
bin\Release\net10.0\newmoon-reader.exe demon Specter   # one demon in detail and what digitalizing with it gives
bin\Release\net10.0\newmoon-reader.exe demon Specter --use   # make it the digitalize demon the page adds
bin\Release\net10.0\newmoon-reader.exe digilevels      # Digitalization Level and % per race, assists, level-unlock skills
bin\Release\net10.0\newmoon-reader.exe expertise       # re-read expertise ranks and Digitalization Levels, rebuild (no CSV row)
bin\Release\net10.0\newmoon-reader.exe toggles         # which toggle skills are on, read from the game ("Name, ..." overrides when it isn't running)
bin\Release\net10.0\newmoon-reader.exe labels          # regenerate reference\stat-labels-game.tsv: the game's own name for each stat (from the client's effect text)
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

**Digitalize demon on the page.** A snapshot taken while digitalized saves that demon as the character's digitalize
demon (`data\cache\digitalize.json`). To pick one without digitalizing, click the demon's name on the Overview's demon line (in the
app; a dropdown of "No demon" and the demons seen before, with "Load demons from the game" at the end to read the COMP again), or `demon "<name>" --use`. The picker lists the COMP demons the game has loaded (all of them while the COMP or Demon
List is open, otherwise only the summoned one), plus every one seen before (`data\cache\comp-demons.json`, with icons in
`data\cache\demon-icons\`), so it also works without the game. In the app, picking never reads the game: it uses the demons and the learned
assists last read (by a snapshot, or by "Load demons from the game" in the dropdown). Picking one saves it, recalculates the latest snapshot's
demon columns with it (the CSV is copied to `<csv>.backup-before-demon-switch.csv` first) and rebuilds the page. Later
snapshots refresh it from the COMP, and one taken while digitalized switches to the demon in use. The Digitalized view then adds it: a demon line on the Overview (framed in its alignment colour, as the player picture is in theirs; its 32x32 game icon, converted from the
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
| Digitalization Levels | The character's progress data in memory (level and points per race); each level's "extend" passive skill counts like an expertise passive ("Digitalize: <race>" on the Expertise page) |
| COMP demons (`demon`) | Each demon's records in memory (`Game\DemonReader.cs`): level, growth type, rebirth ranks, Mitama slots, skill slots, learned skills; base values, growth, traits and Mitama tables from the server's demon tables (`reference\`) |

Layouts are documented in `src/Game/ClientReader.cs`.

## Files

- `src/` code. `Memory/` read-only process access; `Game/` structure layouts; `Calc/` tooltip parsing and bonus math;
  `Build/` snapshot, CSV, expertise and page-data builders.
- `data/cache/` last stats, tooltip text per item, the set table, alignment.
- `reference/enchants-game.tsv` the client's own enchant table; `reference/devils-game.tsv`, `devil-growth.tsv`,
  `mitama-*.tsv`, `guardian-assist.tsv`, `guardian-levels.tsv`, `skills-digitize.tsv` the server's demon and digitalize tables;
  `tokusei-*.tsv`, `expertise-newmoon.tsv`, `traits-unresolved.tsv` NewMoon data from the client (see
  `reference/README.md`; `tools/sbinexport` regenerates them).
- `tools/memdbg` is a read-only memory explorer for finding new data in the client (modes listed in its header).
- `.claude/skills/` holds skills for AI agents working on this repo: dev workflow, memory investigation, checking
  numbers against the game, the server's rules, and decoding game data.
- `data/tooltips.txt` also saves each slot's item record (`(item …)` in the header) and the level; the CSV's
  "Gear records" column does the same per row, so `rebuild` and `recalc` work from ids too.
- `data/dash/` page data: `snapshots.json`, `gear.json`, `expertise.json`, `bonuses.json`.
- Machine-specific paths live in `data/config.json` (created with detected defaults on first run; edit when something is elsewhere): `websiteContent` (the website's content folder), `tokuseiXml` (comp_hack's `datastore/data/tokusei`), `serverConstants` (the local server's `config/constants.xml`: digitalize rates, Mitama set boost skill), `statsFolder` (where new characters' CSVs go), `defaultCharacter`.
