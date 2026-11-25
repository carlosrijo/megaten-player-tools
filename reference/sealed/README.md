# Sealed data

`data.bin` is data the app uses that the repository keeps only encrypted (ChaCha20-Poly1305; the key is not in the
repository). It holds tokusei XML, wording for ids comp_hack's schema doesn't have, and the Gear Database page made with
them. The readable files are never committed: `.gitignore` and the release workflow check for them.

`crates\imagine\build.rs` puts this file and its key into the exe, and `imagine::data::pack` opens it in memory. Nothing
is written to disk. A build without the key (a fork, `BABEL_PACK_KEY=none`) works without it: those effects fall back to
the game's own description text.

## Updating it

The readable files stay outside the repository's tracked folders (`.secrets\`, git-ignored).

```
target\release\imagine-reader.exe seal --tokusei <tokusei XML> --labels <labels.tsv>
cargo build --release
target\release\imagine-reader.exe geardb ... --out app\ui\gear-db.json
target\release\imagine-reader.exe seal --gear-db app\ui\gear-db.local.json
cargo build --release
```

`geardb` writes `gear-db.json` (made without the sealed data; committed) and `gear-db.local.json` beside it (made with
it; git-ignored, never built into the exe). `seal` keeps any entry it is not given. Commit `data.bin` and
`app\ui\gear-db.json`.

## The key

64 hex digits: `.secrets\pack.key` in the main checkout (found from worktrees too) and the GitHub Actions secret
`BABEL_PACK_KEY`, which the release build reads. The release workflow never caches `target\`, where the build keeps it.
A new key means sealing again with it and updating the secret.
