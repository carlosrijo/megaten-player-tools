# Bundled server data

Copies compiled into the reader, so a player with only the exe gets the same numbers as one with the server's folders.
Made by `tools\refresh-bundled-data.ps1` on 2026-11-24; do not edit by hand. The folders set in
`data\config.json` (websiteContent, tokuseiXml, serverConstants) win when they exist.

| Here | From |
|---|---|
| `tokusei\*.xml` | comp_hack's datastore, `data\tokusei` ([colpertac/datastore](https://github.com/colpertac/datastore)), CC BY-SA 4.0 (`tokusei\LICENSE`) |
| `constants.xml` | the IMAGINE server's `config\constants.xml` (also in [colpertac/ai-custom-smt-server](https://github.com/colpertac/ai-custom-smt-server), `deploy\seed\config`) |
| `website\` | the IMAGINE website's `content` folder ([colpertac/ai-custom-smt-server](https://github.com/colpertac/ai-custom-smt-server)); `items.json`, `skills.json` and `devils.json` keep only the fields the reader reads |
