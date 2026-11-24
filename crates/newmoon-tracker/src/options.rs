//! Command line (C# `AppOptions` in Program.cs).
//!
//!   BABEL.exe                 normal use
//!   BABEL.exe --dev           load the page from app\ui in the repo (no rebuild after page edits) + DevTools (F12)
//!   BABEL.exe --capture out.png [--page gear] [--theme dark|light] [--mode digi] [--size 1280x860]
//!                                      open, save a picture of the window's page, and exit (used to check the UI)
//!   ... --capture out.png --page compare --query "short=1"
//!                                      extra page options for that run only (compare = Gear's Compare view)
//!   ... --character Name                show that saved character for this run
//!   ... --data <folder>                 use another data folder (a copy, for tests; scripted runs may then save into it)
//!   ... --log-level debug|info|warn     how much goes into the log in <data folder>\logs (default info)
//!   BABEL.exe --tour steps.json --capture folder [--size 1280x720]
//!                                      click through the page as scripted, saving a picture per step (for the demo video)
//!   ... --tour steps.json --capture folder --live
//!                                      the same steps filmed as they happen, with a pointer that moves and clicks
//!   ... --capture ... --offscreen       that run's window is placed beside the desktop, where nobody sees it or mouses over it
//!   ... --capture out.png --then-scheme light|dark
//!                                      stand in for Windows switching its light/dark setting while the app is open:
//!                                      the page is told the new scheme after it has loaded, then the picture is taken

#[derive(Debug, Clone)]
pub struct AppOptions {
    pub dev: bool,
    pub live: bool,
    pub offscreen: bool,
    pub capture: Option<String>,
    pub page: Option<String>,
    pub theme: Option<String>,
    pub mode: Option<String>,
    pub then_scheme: Option<String>,
    pub query: Option<String>,
    pub character: Option<String>,
    pub tour: Option<String>,
    pub data: Option<String>,
    /// `--log-level debug|info|warn`: how much goes into the local log (default info)
    pub log_level: Option<String>,
    pub width: i32,
    pub height: i32,
}

impl AppOptions {
    pub fn parse(a: &[String]) -> AppOptions {
        let mut o = AppOptions {
            dev: false,
            live: false,
            offscreen: false,
            capture: None,
            page: None,
            theme: None,
            mode: None,
            then_scheme: None,
            query: None,
            character: None,
            tour: None,
            data: None,
            log_level: None,
            width: 1280,
            height: 860,
        };
        let mut i = 0;
        while i < a.len() {
            // C# `Next()`: the following argument (and skip it), or null at the end
            let flag = a[i].clone();
            let mut next = || -> Option<String> {
                if i + 1 < a.len() {
                    i += 1;
                    Some(a[i].clone())
                } else {
                    None
                }
            };
            match flag.as_str() {
                "--dev" => o.dev = true,
                "--capture" => o.capture = next(),
                "--page" => o.page = next(),
                "--theme" => o.theme = next(),
                "--then-scheme" => o.then_scheme = next(),
                "--mode" => o.mode = next(),
                "--query" => o.query = next(),
                "--character" => o.character = next(),
                "--tour" => o.tour = next(),
                "--live" => o.live = true,
                "--offscreen" => o.offscreen = true,
                "--data" => o.data = next(),
                "--log-level" => o.log_level = next(),
                "--size" => {
                    let s = next().unwrap_or_default();
                    let p: Vec<&str> = s.split('x').collect();
                    if p.len() == 2 {
                        if let (Ok(w), Ok(h)) = (p[0].trim().parse::<i32>(), p[1].trim().parse::<i32>()) {
                            o.width = w;
                            o.height = h;
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
        o
    }
}
