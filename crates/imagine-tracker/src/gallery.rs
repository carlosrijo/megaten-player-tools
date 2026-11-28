//! Community Builds (gallery-worker\): what changes the gallery (publishing a build, updating it, taking it down,
//! reporting one) BABEL sends itself, not its page; the page reads the gallery with its own fetch. The address and the
//! calls that can be made are fixed here. The page gives the method, the path, the build and the build's secret
//! (from community.json, build::community::published).
//!
//! Test builds: BABEL_GALLERY_URL set at build time (may be http://127.0.0.1:<port>, a `wrangler dev`) replaces the
//! real address. Never set it for a release.

use anyhow::Result;
use imagine::build::community;
use imagine::user_error;
use serde_json::{json, Value};

use crate::update::http;

const URL: &str = "https://babel-gallery.babel-feedback.workers.dev";
/// More than a build can be (the gallery takes 40 KB).
const MAX_SEND: usize = 64 * 1024;
const MAX_ANSWER: usize = 64 * 1024;

fn base() -> &'static str {
    option_env!("BABEL_GALLERY_URL").filter(|s| !s.is_empty()).unwrap_or(URL)
}

/// The calls that change the gallery: POST /builds, PUT or DELETE /builds/<id>, POST /builds/<id>/report.
fn allowed(method: &str, path: &str) -> bool {
    if method == "POST" && path == "/builds" {
        return true;
    }
    let Some(rest) = path.strip_prefix("/builds/") else { return false };
    match rest.split_once('/') {
        None => matches!(method, "PUT" | "DELETE") && community::is_build_id(rest),
        Some((id, "report")) => method == "POST" && community::is_build_id(id),
        _ => false,
    }
}

/// One call: { status, data } (the gallery's answer), or { offline: true } when it can't be reached.
pub fn call(method: &str, path: &str, body: Option<&Value>, secret: Option<&str>) -> Result<Value> {
    call_to(base(), method, path, body, secret)
}

fn call_to(base: &str, method: &str, path: &str, body: Option<&Value>, secret: Option<&str>) -> Result<Value> {
    if !allowed(method, path) {
        return Err(user_error("BABEL doesn't send that to the gallery."));
    }
    let mut headers = vec!["Accept: application/json".to_string()];
    if let Some(s) = secret {
        if !community::is_secret(s) {
            return Err(user_error("The saved secret for this build isn't one the gallery gave."));
        }
        headers.push(format!("Authorization: Bearer {s}"));
    }
    let body = body.map(serde_json::to_vec).transpose()?;
    if let Some(b) = &body {
        if b.len() > MAX_SEND {
            return Err(user_error("That build is too big for the gallery."));
        }
        headers.push("Content-Type: application/json".to_string());
    }
    match http::send(method, &format!("{base}{path}"), &headers, body.as_deref(), MAX_ANSWER) {
        Ok((status, bytes)) => {
            let data = serde_json::from_slice::<Value>(&bytes)
                .ok()
                .filter(Value::is_object)
                .unwrap_or_else(|| json!({ "ok": false, "error": format!("The gallery answered {status}.") }));
            Ok(json!({ "status": status, "data": data }))
        }
        Err(_) => Ok(json!({ "offline": true })),
    }
}

#[cfg(test)]
mod tests {
    use super::allowed;

    #[test]
    fn only_the_gallery_calls() {
        assert!(allowed("POST", "/builds"));
        assert!(allowed("PUT", "/builds/AbCdEf1234"));
        assert!(allowed("DELETE", "/builds/AbCdEf1234"));
        assert!(allowed("POST", "/builds/AbCdEf1234/report"));
        for (m, p) in [
            ("GET", "/builds"),
            ("PUT", "/builds"),
            ("POST", "/builds/AbCdEf1234"),
            ("GET", "/builds/AbCdEf1234"),
            ("DELETE", "/builds/AbCdEf1234/report"),
            ("PUT", "/builds/../admin/x"),
            ("PUT", "/builds/AbCdEf123"),
            ("POST", "/builds/AbCdEf1234/report/x"),
            ("POST", "/admin/builds/AbCdEf1234"),
            ("POST", "/builds?x=1"),
            ("PUT", "/builds/AbCdEf1234?x"),
        ] {
            assert!(!allowed(m, p), "{m} {p}");
        }
    }

    /// Against a gallery on this PC (gallery-worker README, `wrangler dev`):
    /// BABEL_TEST_GALLERY=http://127.0.0.1:8799 cargo test -p imagine-tracker -- --ignored gallery_
    #[test]
    #[ignore]
    fn gallery_publish_update_report_remove() {
        let base = std::env::var("BABEL_TEST_GALLERY").expect("BABEL_TEST_GALLERY");
        let cases: serde_json::Value = serde_json::from_str(include_str!("../../../gallery-worker/test/builds.json")).unwrap();
        let build = &cases["good"]["demon"];
        let made = super::call_to(&base, "POST", "/builds", Some(build), None).unwrap();
        assert_eq!(made["status"], 201, "{made}");
        let id = made["data"]["id"].as_str().unwrap().to_string();
        let secret = made["data"]["secret"].as_str().unwrap().to_string();
        let mut changed = build.clone();
        changed["about"]["title"] = "Changed".into();
        assert_eq!(super::call_to(&base, "PUT", &format!("/builds/{id}"), Some(&changed), Some(&secret)).unwrap()["status"], 200);
        assert_eq!(super::call_to(&base, "PUT", &format!("/builds/{id}"), Some(&changed), Some(&"x".repeat(43))).unwrap()["status"], 403);
        let rep = super::call_to(&base, "POST", &format!("/builds/{id}/report"), Some(&serde_json::json!({ "reason": "spam" })), None).unwrap();
        assert!(matches!(rep["status"].as_u64(), Some(200 | 429)), "{rep}");
        assert_eq!(super::call_to(&base, "DELETE", &format!("/builds/{id}"), None, Some(&secret)).unwrap()["status"], 200);
        assert_eq!(super::call_to(&base, "DELETE", &format!("/builds/{id}"), None, Some(&secret)).unwrap()["status"], 404);
        assert_eq!(super::call_to("http://127.0.0.1:1", "POST", "/builds", Some(build), None).unwrap()["offline"], true);
    }
}
