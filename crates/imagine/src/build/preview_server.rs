//! Port of `src/Build/PreviewServer.cs`.
//!
//! `imagine-reader serve`: the app's page in a plain browser, for testing page changes (localhost only).
//! It is tools\serve.ps1 (static files under the root) plus two things:
//!   /data/...       is answered from the character's folder in use (--data, so a copy can be shown)
//!   POST /host/cmd  answers the Simulator's commands with the real calculation (simInit, simulate, simCatalog) and
//!                   keeps its saved builds (simBuilds, simSaveBuild, simDeleteBuild: sim-builds.json in the data folder);
//!                   the page's browser stand-in calls these, so the Simulator works in the preview as in the app.
//! Everything else the page asks its host stays the stand-in's (fake game, fake snapshots): see app\ui\app.js.
//!
//! What it answers: only requests addressed to this PC by name (Host `localhost`, `127.0.0.1` or `[::1]`), only the
//! files the page loads (`app/ui/`, `reference/` under the root, and `/data/`), and commands only from the page itself
//! or a tool with no Origin (a POST from any other site's page is refused).
//!
//! The desktop app calls [`sim_command`] (the same dispatcher) from its page-command handler; see its documentation.
//!
//! The C# used `HttpListener` (one request at a time); this uses `tiny_http` on 127.0.0.1 (and [::1] when it can be
//! bound, since a browser may try either for "localhost"), and answers one request at a time as well.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value, json};

use crate::console;
use crate::data::json::{self, At};
use crate::error::is_user_error;
use crate::{Result, UserError};

use super::pipeline::Pipeline;
use super::simulator::{self, to_json};

/// `Func<string, Pipeline> pipeOf`: the pipeline of a character by name ("" = a build with no base). See [`sim_command`].
pub type PipeOf<'a> = &'a dyn Fn(&str) -> Result<Arc<Pipeline>>;
/// `Func<string, string> skillIconsOf`: where the page finds a character's skill icons (None = not known).
pub type SkillIconsOf<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The Content-Type of a served file (`Mime`).
fn mime(path: &str) -> &'static str {
    let ext = Path::new(path).extension().map(|e| format!(".{}", e.to_string_lossy().to_lowercase())).unwrap_or_default();
    match ext.as_str() {
        ".html" => "text/html; charset=utf-8",
        ".css" => "text/css; charset=utf-8",
        ".js" => "text/javascript; charset=utf-8",
        ".json" => "application/json; charset=utf-8",
        ".svg" => "image/svg+xml",
        ".png" => "image/png",
        _ => "application/octet-stream",
    }
}

/// The Simulator's page commands, shared by the app (MainForm.Dispatch) and the preview server. `Ok(None)` = not one
/// of them (the C# returned null). Otherwise the command's result as JSON text (what goes in `"result"` of the reply).
///
/// - `pipe_of`: the pipeline of the character a build stands on: its name, or "" for a build with no base (a pipeline
///   made with `blank_sets`, see `Pipeline::new`). Returns an error (a `UserError`, for a name it does not have) when
///   there is none. The pipelines are shared (`Arc`), the simulator in each runs one call at a time under its own lock,
///   so the app can keep its pipelines and hand out clones of the `Arc`s from any thread.
/// - `default_base`: the base when the page names none: the character being shown.
/// - `builds_path`: sim-builds.json: one list of saved builds for all characters.
/// - `owner`: whose builds the ones saved before builds had bases are (see `simulator::builds`); `None` = `default_base`.
/// - `skill_icons_of`: where the page finds a character's skill icons (the app's address for its dash\skill-icons
///   folder); `None` = not known.
/// - `cmd` is the command's name without the "host/" the preview server's URL has; `args` the page's JSON arguments.
pub fn sim_command(
    pipe_of: PipeOf,
    default_base: &str,
    builds_path: &Path,
    cmd: &str,
    args: Option<&Value>,
    owner: Option<&str>,
    skill_icons_of: Option<SkillIconsOf>,
) -> Result<Option<String>> {
    let who: String = if args.at("base").is_some() { json::str(args.at("base")).unwrap_or_default() } else { default_base.to_string() };
    let owner = owner.unwrap_or(default_base);
    Ok(Some(match cmd {
        // the base character's COMP demons as last seen (what its demon picker lists; the game is not looked at), each
        // with what digitalizing with it gives: a build's demon is picked from these
        "simDemons" => {
            let demons = if who.is_empty() { Value::Null } else { Value::Array(pipe_of(&who)?.comp_demons(&who, false)?.iter().map(|d| d.to_value()).collect()) };
            let icons = if who.is_empty() { None } else { skill_icons_of.and_then(|f| f(&who)) };
            to_json(&json!({ "demons": demons, "skillIcons": icons }))
        }
        "simInit" => pipe_of(&who)?.sim.init(args)?,
        "simulate" => pipe_of(&who)?.sim.run(args)?,
        "simCatalog" => pipe_of(&who)?.sim.catalog(json::int_or(args.at("slot"), -1), args)?,
        // a build as a code for another player, and such a code back as a build (neither saves anything)
        "simShareCode" => pipe_of(&who)?.sim.share(args)?,
        "simImportCode" => pipe_of(&who)?.sim.import(args)?,
        "simBuilds" => simulator::builds(builds_path, Some(owner))?,
        "simSaveBuild" => simulator::save_build(builds_path, args, Some(owner))?,
        "simDeleteBuild" => simulator::delete_build(builds_path, json::str(args.at("id")).as_deref(), Some(owner))?,
        _ => return Ok(None),
    }))
}

/// [`sim_command`] with the try/catch of the preview server around it: the whole reply for the page, `{"ok":true,
/// "result":...}`, or `{"ok":false,"error":...,"kind":"user"|"error"}` (a plain error is also written to stderr).
/// `None` = not a Simulator command.
pub fn sim_reply(
    pipe_of: PipeOf,
    default_base: &str,
    builds_path: &Path,
    cmd: &str,
    body: &str,
    owner: Option<&str>,
    skill_icons_of: Option<SkillIconsOf>,
) -> Option<String> {
    let run = || -> Result<Option<String>> {
        let args = if body.chars().all(char::is_whitespace) { None } else { Some(json::parse(body)?) };
        Ok(sim_command(pipe_of, default_base, builds_path, cmd, args.as_ref(), owner, skill_icons_of)?.map(|result| format!("{{\"ok\":true,\"result\":{result}}}")))
    };
    match run() {
        Ok(reply) => reply,
        Err(e) if is_user_error(&e) => Some(to_json(&json!({ "ok": false, "error": e.downcast_ref::<UserError>().map(|u| u.0.clone()).unwrap_or_default(), "kind": "user" }))),
        Err(e) => {
            let reply = to_json(&json!({ "ok": false, "error": e.to_string(), "kind": "error" }));
            console::error_line(&format!("{e:?}"));
            Some(reply)
        }
    }
}

/// `Uri.UnescapeDataString`: %XX sequences become the bytes they stand for (read as UTF-8); a bad one stays as it is.
fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() && b[i + 1].is_ascii_hexdigit() && b[i + 2].is_ascii_hexdigit() {
            out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap_or(b'%'));
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The request's path as `Url.AbsolutePath` has it: without query and fragment, dot segments resolved (HTTP.sys and
/// `Uri` do that before the C# code sees it).
fn absolute_path(raw: &str) -> String {
    let raw = raw.split(['?', '#']).next().unwrap_or("");
    let mut segs: Vec<&str> = Vec::new();
    let parts: Vec<&str> = raw.split('/').collect();
    for (i, seg) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        match *seg {
            "." => {
                if last {
                    segs.push("");
                }
            }
            ".." => {
                segs.pop();
                if last {
                    segs.push("");
                }
            }
            s => segs.push(s),
        }
    }
    let joined = segs.join("/");
    if joined.starts_with('/') { joined } else { format!("/{joined}") }
}

/// `Path.GetFullPath(p).TrimEnd('\\', '/') + DirectorySeparatorChar`.
fn folder_with_separator(p: &str) -> Result<String> {
    let full = std::path::absolute(p)?.to_string_lossy().into_owned();
    Ok(format!("{}{}", full.trim_end_matches(['\\', '/']), std::path::MAIN_SEPARATOR))
}

fn header(name: &str, value: &str) -> tiny_http::Header {
    tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("a header made of plain text")
}

/// A request header's value (`None` = not sent).
fn header_value<'a>(req: &'a tiny_http::Request, name: &'static str) -> Option<&'a str> {
    req.headers().iter().find(|h| h.field.equiv(name)).map(|h| h.value.as_str())
}

/// `localhost`, `127.0.0.1` or `[::1]`, with or without a port: a name for this PC.
fn is_this_pc(host: &str) -> bool {
    let host = host.trim().to_ascii_lowercase();
    // the name ends at the port's ':' ("[::1]" has its own colons, so it ends at the ']')
    let end = if host.starts_with('[') { host.find(']').map(|i| i + 1) } else { Some(host.find(':').unwrap_or(host.len())) };
    let Some(end) = end else { return false };
    let (name, port) = host.split_at(end);
    let port_ok = port.is_empty() || port.strip_prefix(':').is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    matches!(name, "localhost" | "127.0.0.1" | "[::1]") && port_ok
}

/// A page's Origin that is this PC over http (`http://localhost:18770` etc.).
fn is_this_pc_origin(origin: &str) -> bool {
    origin.strip_prefix("http://").is_some_and(is_this_pc)
}

/// The folder a request path is answered from and the path inside it; `None` = not one of the page's files. The
/// prefixes are matched without regard to case (the folders are on Windows).
fn served_file<'p>(path: &'p str, root: &str, data_dir: &str) -> Option<(String, &'p str)> {
    let lower = path.to_ascii_lowercase();
    for (prefix, under_root) in [("data/", None), ("app/ui/", Some("app\\ui\\")), ("reference/", Some("reference\\"))] {
        if lower.starts_with(prefix) {
            let base = match under_root {
                None => data_dir.to_string(),
                Some(sub) => format!("{root}{}", sub.replace('\\', std::path::MAIN_SEPARATOR_STR)),
            };
            return Some((base, &path[prefix.len()..]));
        }
    }
    None
}

/// One request: the body of the C# loop.
fn serve_request<F>(mut req: tiny_http::Request, pipe_of: &F, default_base: &str, builds_path: &Path, root: &str, data_dir: &str)
where
    F: Fn(&str) -> Result<Arc<Pipeline>>,
{
    let path = unescape(&absolute_path(req.url())).trim_start_matches('/').to_string();
    let no_store = header("Cache-Control", "no-store");
    let post = *req.method() == tiny_http::Method::Post;
    // only requests that name this PC, and commands only from the page itself (or a tool, which sends no Origin)
    let refused = !header_value(&req, "Host").is_some_and(is_this_pc) || (post && header_value(&req, "Origin").is_some_and(|o| !is_this_pc_origin(o)));
    let result = if refused {
        req.respond(tiny_http::Response::empty(403).with_header(no_store))
    } else if post && path.starts_with("host/") {
        let mut raw = Vec::new();
        let _ = std::io::Read::read_to_end(req.as_reader(), &mut raw);
        let body = json::decode_text(&raw);
        match sim_reply(pipe_of, default_base, builds_path, &path[5..], &body, None, None) {
            None => req.respond(tiny_http::Response::empty(404).with_header(no_store)),
            Some(reply) => req.respond(tiny_http::Response::from_data(reply.into_bytes()).with_header(header("Content-Type", "application/json; charset=utf-8")).with_header(no_store)),
        }
    } else if path.eq_ignore_ascii_case("app/ui/gear-db.json") && crate::data::pack::entry(crate::data::pack::GEAR_DB).is_some() {
        // the Gear Database made with the sealed data (as the app does: site.rs)
        let body = crate::data::pack::entry(crate::data::pack::GEAR_DB).unwrap_or_default().to_vec();
        req.respond(tiny_http::Response::from_data(body).with_header(header("Content-Type", "application/json; charset=utf-8")).with_header(no_store))
    } else if path.eq_ignore_ascii_case("app/ui/rbp-effects.json") && super::rebirth_planner::effects_bytes().is_some() {
        // the Rebirth Planner's skill effects made with the sealed data (as the app does: site.rs)
        let body = super::rebirth_planner::effects_bytes().unwrap_or_default().to_vec();
        req.respond(tiny_http::Response::from_data(body).with_header(header("Content-Type", "application/json; charset=utf-8")).with_header(no_store))
    } else {
        // Only the page's own files (app/ui/, reference/ and the data folder) are ever served, and only from inside them.
        let found = served_file(&path, root, data_dir).and_then(|(base_dir, rel)| {
            let file = std::path::absolute(PathBuf::from(&base_dir).join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))).ok()?.to_string_lossy().into_owned();
            (file.to_uppercase().starts_with(&base_dir.to_uppercase()) && Path::new(&file).is_file()).then_some(file)
        });
        if let Some(file) = found {
            match std::fs::read(&file) {
                Ok(bytes) => req.respond(tiny_http::Response::from_data(bytes).with_header(header("Content-Type", mime(&file))).with_header(no_store)),
                Err(_) => req.respond(tiny_http::Response::empty(200).with_header(no_store)), // the browser went away / unreadable: an empty answer
            }
        } else {
            req.respond(tiny_http::Response::empty(404).with_header(no_store))
        }
    };
    let _ = result; // catch (IOException or HttpListenerException) { }: the browser went away
}

/// `PreviewServer.Run`: serves until the process is stopped.
/// `pipe_of`, `default_base`, `builds_path`: as for [`sim_command`]. `root`: the folder the page's files are under
/// (`app/ui/index.html` etc.), `data_dir`: the character's folder answered at `/data/`.
pub fn run<F>(pipe_of: &F, default_base: &str, builds_path: &Path, root: &str, port: u16, data_dir: &str) -> Result<()>
where
    F: Fn(&str) -> Result<Arc<Pipeline>> + Sync,
{
    let root = folder_with_separator(root)?;
    let data_dir = folder_with_separator(data_dir)?;
    let v4 = tiny_http::Server::http(("127.0.0.1", port)).map_err(|e| anyhow::anyhow!("{e}"))?;
    let v6 = tiny_http::Server::http(("::1", port)).ok();
    console::write_line(&format!("Serving {root} on http://localhost:{port}/app/ui/index.html (data: {data_dir}). Ctrl+C stops it."));
    // one request at a time, as the C# loop did: the servers' threads take turns
    let turn = std::sync::Mutex::new(());
    std::thread::scope(|scope| {
        let serve = |server: tiny_http::Server| {
            for req in server.incoming_requests() {
                let _t = turn.lock().unwrap_or_else(|e| e.into_inner());
                serve_request(req, pipe_of, default_base, builds_path, &root, &data_dir);
            }
        };
        if let Some(v6) = v6 {
            scope.spawn(move || serve(v6));
        }
        serve(v4);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_types() {
        assert_eq!(mime("C:\\x\\index.HTML"), "text/html; charset=utf-8");
        assert_eq!(mime("a.js"), "text/javascript; charset=utf-8");
        assert_eq!(mime("a.json"), "application/json; charset=utf-8");
        assert_eq!(mime("a.svg"), "image/svg+xml");
        assert_eq!(mime("a.png"), "image/png");
        assert_eq!(mime("a.woff2"), "application/octet-stream");
        assert_eq!(mime("noext"), "application/octet-stream");
    }

    #[test]
    fn urls_are_read_like_the_csharp() {
        assert_eq!(absolute_path("/app/ui/index.html?x=1#y"), "/app/ui/index.html");
        assert_eq!(absolute_path("/data/../app/x.js"), "/app/x.js");
        assert_eq!(absolute_path("/a/./b/"), "/a/b/");
        assert_eq!(absolute_path("/../../etc"), "/etc");
        assert_eq!(unescape("a%20b%E3%81%82%zz%4"), "a b\u{3042}%zz%4");
    }

    #[test]
    fn only_names_for_this_pc_are_answered() {
        for ok in ["localhost", "LocalHost:18770", "127.0.0.1", "127.0.0.1:80", "[::1]", "[::1]:18770", " localhost:1 "] {
            assert!(is_this_pc(ok), "{ok}");
        }
        for no in ["", "example.com", "localhost.example.com", "example.com:18770", "127.0.0.2", "127.0.0.1.nip.io", "localhost:", "localhost:x", "[::1", "[::2]:80", "::1"] {
            assert!(!is_this_pc(no), "{no}");
        }
        assert!(is_this_pc_origin("http://localhost:18770"));
        assert!(!is_this_pc_origin("https://example.com"));
        assert!(!is_this_pc_origin("http://example.com"));
        assert!(!is_this_pc_origin("null"));
    }

    #[test]
    fn only_the_pages_folders_are_served() {
        let (root, data) = (r"C:\repo\", r"C:\data\");
        let sep = std::path::MAIN_SEPARATOR_STR;
        assert_eq!(served_file("app/ui/app.js", root, data), Some((format!(r"C:\repo\app{sep}ui{sep}"), "app.js")));
        assert_eq!(served_file("reference/devil-models.tsv", root, data), Some((format!(r"C:\repo\reference{sep}"), "devil-models.tsv")));
        assert_eq!(served_file("DATA/dash/gear.json", root, data), Some((data.to_string(), "dash/gear.json")));
        assert_eq!(served_file("README.md", root, data), None);
        assert_eq!(served_file(".secrets/x.pem", root, data), None);
        assert_eq!(served_file("app/x.js", root, data), None);
    }

    #[test]
    fn unknown_commands_are_not_ours() {
        let none = |_: &str| -> Result<Arc<Pipeline>> { Err(crate::user_error("no")) };
        assert_eq!(sim_command(&none, "Rubito", Path::new("x.json"), "somethingElse", None, None, None).unwrap(), None);
        assert_eq!(sim_reply(&none, "Rubito", Path::new("x.json"), "somethingElse", "", None, None), None);
        // a pipeline the app does not have is a message for the person
        let reply = sim_reply(&none, "Rubito", Path::new("x.json"), "simInit", "", None, None).unwrap();
        assert_eq!(reply, r#"{"ok":false,"error":"no","kind":"user"}"#);
    }
}

#[cfg(test)]
mod server_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    fn request(port: u16, raw: &str) -> (String, String) {
        let mut s = None;
        for _ in 0..100 {
            if let Ok(c) = TcpStream::connect(("127.0.0.1", port)) {
                s = Some(c);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let mut s = s.expect("the server answers");
        s.write_all(raw.as_bytes()).unwrap();
        let mut out = Vec::new();
        s.read_to_end(&mut out).unwrap();
        let text = String::from_utf8_lossy(&out).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        (head.to_string(), body.to_string())
    }

    fn get(port: u16, path: &str) -> (String, String) {
        request(port, &format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"))
    }

    fn post(port: u16, path: &str, body: &str) -> (String, String) {
        request(port, &format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()))
    }

    #[test]
    fn the_preview_server_serves_files_and_simulator_commands() {
        let dir = std::env::temp_dir().join(format!("imagine-preview-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("root").join("app").join("ui")).unwrap();
        std::fs::create_dir_all(dir.join("data")).unwrap();
        std::fs::write(dir.join("root").join("app").join("ui").join("x.js"), "var a = 1;").unwrap();
        std::fs::write(dir.join("root").join("secret.txt"), "no").unwrap();
        std::fs::write(dir.join("data").join("a.json"), "{\"a\":1}").unwrap();
        let port = {
            let l = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
            l.local_addr().unwrap().port()
        };
        let (root, data) = (dir.join("root").to_string_lossy().into_owned(), dir.join("data").to_string_lossy().into_owned());
        let builds = dir.join("data").join("sim-builds.json");
        std::thread::spawn(move || {
            let pipe_of = |who: &str| -> Result<Arc<Pipeline>> { Err(crate::user_error(format!("no {who}"))) };
            let _ = run(&pipe_of, "Rubito", &builds, &root, port, &data);
        });

        let (head, body) = get(port, "/app/ui/x.js?v=2");
        assert!(head.starts_with("HTTP/1.1 200"), "{head}");
        assert!(head.to_lowercase().contains("content-type: text/javascript; charset=utf-8"), "{head}");
        assert!(head.to_lowercase().contains("cache-control: no-store"), "{head}");
        assert_eq!(body, "var a = 1;");
        let (head, body) = get(port, "/data/a.json");
        assert!(head.to_lowercase().contains("content-type: application/json; charset=utf-8"), "{head}");
        assert_eq!(body, "{\"a\":1}");
        assert!(get(port, "/data/../root/secret.txt").0.starts_with("HTTP/1.1 404"));
        assert!(get(port, "/nothing.html").0.starts_with("HTTP/1.1 404"));
        assert!(get(port, "/").0.starts_with("HTTP/1.1 404"));
        assert!(get(port, "/host/simBuilds").0.starts_with("HTTP/1.1 404")); // only POST is a command
        assert!(post(port, "/host/nothing", "{}").0.starts_with("HTTP/1.1 404"));
        // only the page's folders: a file elsewhere under the root is not served, however it is asked for
        std::fs::create_dir_all(dir.join("root").join(".secrets")).unwrap();
        std::fs::write(dir.join("root").join(".secrets").join("k.pem"), "no").unwrap();
        for p in ["/secret.txt", "/.secrets/k.pem", "/app/ui/../../secret.txt", "/app/ui/%2e%2e/%2e%2e/secret.txt", "/app/ui/..%5c..%5csecret.txt", "/DATA/../secret.txt"] {
            assert!(get(port, p).0.starts_with("HTTP/1.1 404"), "{p}");
        }
        assert_eq!(get(port, "/DATA/a.json").1, "{\"a\":1}"); // the data folder's prefix in any case
        // only requests that name this PC
        let other_host = request(port, "GET /app/ui/x.js HTTP/1.1\r\nHost: example.com:1\r\nConnection: close\r\n\r\n");
        assert!(other_host.0.starts_with("HTTP/1.1 403"), "{}", other_host.0);
        assert!(request(port, "GET /app/ui/x.js HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").0.starts_with("HTTP/1.1 200"));
        // commands only from the page itself (or a tool, which sends no Origin)
        let from = |origin: &str| request(port, &format!("POST /host/simBuilds HTTP/1.1\r\nHost: localhost\r\nOrigin: {origin}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")).0;
        assert!(from("https://example.com").starts_with("HTTP/1.1 403"));
        assert!(from("null").starts_with("HTTP/1.1 403"));
        assert!(from(&format!("http://localhost:{port}")).starts_with("HTTP/1.1 200"));

        let (head, body) = post(port, "/host/simBuilds", "");
        assert!(head.to_lowercase().contains("content-type: application/json; charset=utf-8"), "{head}");
        assert_eq!(body, r#"{"ok":true,"result":[]}"#);
        assert_eq!(post(port, "/host/simSaveBuild", "{}").1, r#"{"ok":false,"error":"Give the build a name.","kind":"user"}"#);
        let saved = post(port, "/host/simSaveBuild", r#"{"name":"A","base":"Rubito","points":{"4":10000}}"#).1;
        assert!(saved.starts_with(r#"{"ok":true,"result":{"id":"b"#), "{saved}");
        let listed = post(port, "/host/simBuilds", "").1;
        assert!(listed.contains(r#""name":"A""#) && listed.contains(r#""base":"Rubito""#), "{listed}");
        // a character the preview does not have is a message for the person
        assert_eq!(post(port, "/host/simInit", r#"{"base":"Nobody"}"#).1, r#"{"ok":false,"error":"no Nobody","kind":"user"}"#);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
