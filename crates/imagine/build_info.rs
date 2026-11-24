// What a program's build script stamps into it: the version, the git commit and the build date. NOT a module of the
// library: crates\imagine-tracker\build.rs and crates\imagine-reader\build.rs each `include!` this file, then call
// `emit_build_info()`; the program reads the values with `env!("IMAGINE_BUILD_VERSION")` / `..._COMMIT` / `..._DATE`
// and hands them to `imagine::diag::about::init`.
//
//   version  IMAGINE_VERSION when set (tools\publish-portable.ps1 -Version, the release workflow), else the crate's version
//   commit   `git rev-parse --short=7 HEAD`, with "-dirty" added when `git status --porcelain` lists changes;
//            "unknown" when git is not there or this is not a checkout (a source archive)
//   date     the build's UTC date and time, "2026-11-24 03:41 UTC"; SOURCE_DATE_EPOCH (seconds) fixes it for a repeatable build

/// UTC "2026-11-24 03:41" from seconds since 1970.
fn utc_minute(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", rem / 3600, rem % 3600 / 60)
}

fn git_output(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Print the `cargo:` lines that stamp the build. Call it first in `main` of build.rs.
fn emit_build_info() {
    println!("cargo:rerun-if-env-changed=IMAGINE_VERSION");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=../imagine/src");
    println!("cargo:rerun-if-changed=../imagine/build_info.rs");

    let version = std::env::var("IMAGINE_VERSION")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap_or_default());

    let commit = match git_output(&["rev-parse", "--short=7", "HEAD"]).filter(|s| !s.is_empty()) {
        Some(short) => {
            let dirty = git_output(&["--no-optional-locks", "status", "--porcelain"]).is_some_and(|s| !s.is_empty());
            if dirty { format!("{short}-dirty") } else { short }
        }
        None => "unknown".to_string(),
    };
    // a new commit or a staged change moves these files: build again then, so the stamp is not stale
    let mut watch = vec!["HEAD".to_string(), "index".to_string(), "packed-refs".to_string()];
    if let Some(head) = git_output(&["symbolic-ref", "-q", "HEAD"]) {
        watch.push(head);
    }
    for w in watch {
        if let Some(p) = git_output(&["rev-parse", "--git-path", &w]) {
            if std::path::Path::new(&p).exists() {
                println!("cargo:rerun-if-changed={p}");
            }
        }
    }

    let secs = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0));

    println!("cargo:rustc-env=IMAGINE_BUILD_VERSION={version}");
    println!("cargo:rustc-env=IMAGINE_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=IMAGINE_BUILD_DATE={} UTC", utc_minute(secs));
}
