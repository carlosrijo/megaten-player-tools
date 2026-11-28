# babel-gallery: Community Builds

The service behind Community Builds. BABEL's "Publish build" sends a build here, and the website's Community Builds pages
(Player builds, Demon builds) read from here. It runs as a Cloudflare Worker with a D1 database, on the free plan.

- `src/build.js`: the gate. It uses the same rules as BABEL's own check (`crates\imagine\src\build\community.rs`).
  `test\builds.json` holds the cases both run.
- `src/gallery.js`: the routes, limits, secrets and reports (the list is at the top of the file). `src/index.js` is
  the entry.
- `schema.sql`: the tables.

## Tests

```powershell
node --test test/
```

No Cloudflare is needed: D1 is stood in for by Node's own SQLite, with the real `schema.sql`.

To try it in the real Workers runtime on this PC, with a local database:

```powershell
copy wrangler.example.toml wrangler.toml   # any database_id works locally
npx wrangler d1 execute babel-gallery --local --file schema.sql
npx wrangler dev --local --port 8799
```

`.dev.vars` holds the local secrets (`ADMIN_KEY=...`, `IP_SALT=...`). It is git-ignored.

## Setting it up on Cloudflare

1. Run `npx wrangler d1 create babel-gallery`. Copy `wrangler.example.toml` to `wrangler.toml` (git-ignored, kept
   in the main checkout) and put the printed id in `database_id`.
2. Run `npx wrangler d1 execute babel-gallery --remote --file schema.sql`.
3. Set the secrets with `npx wrangler secret put ADMIN_KEY` (keep a copy in `.secrets\`) and
   `npx wrangler secret put IP_SALT` (any long random text).
4. Run `npx wrangler deploy`. The address is `https://babel-gallery.<account subdomain>.workers.dev`.

## Looking after it

Send `Authorization: Bearer <ADMIN_KEY>` with these:

- `GET /admin/reports`: reported builds, the hidden ones first.
- `POST /admin/builds/<id>` with `{ "hidden": false }`: show a build again. The reports so far count as seen.
- `DELETE /admin/builds/<id>`: remove a build.
