//! Talking to the window from other threads.
//!
//! WebView2 (and the window) belong to the UI thread. The C# code was `async`: work ran on the thread pool and the
//! continuation came back to the UI thread. Here the work runs on plain worker threads and asks the UI thread for
//! anything that touches the page, through the event loop: `Ui` is the cloneable handle for that. The calls that
//! return something (`exec`, `capture_png`, `dev_tools`) block the calling worker thread until the UI thread has the
//! answer, so the tour code can read like the C# `await` sequences. Never call them on the UI thread.

use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use tao::event_loop::EventLoopProxy;

pub type Reply<T> = Sender<Result<T, String>>;

/// Called on the UI thread with the parameters (JSON text) of each DevTools protocol event.
pub type EventSink = Arc<dyn Fn(String) + Send + Sync>;

pub enum UserEvent {
    /// `CoreWebView2.PostWebMessageAsJson(json)`
    Post(String),
    /// `CoreWebView2.ExecuteScriptAsync(script)`: the script's result as JSON text
    Exec { script: String, reply: Reply<String> },
    /// `CoreWebView2.CapturePreviewAsync(Png, stream)`: the PNG bytes
    Capture { reply: Reply<Vec<u8>> },
    /// `CoreWebView2.CallDevToolsProtocolMethodAsync(method, parametersAsJson)`
    DevTools { method: String, params: String, reply: Option<Reply<String>> },
    /// `CoreWebView2.GetDevToolsProtocolEventReceiver(name).DevToolsProtocolEventReceived += ...`
    DevToolsEvents { name: String, sink: EventSink },
    /// `SetScheme(theme)`: the page's prefers-color-scheme ("dark" | "light" | anything else = Auto)
    SetScheme(Option<String>),
    /// `ApplyChrome(dark)`: title bar and the colour behind the page
    ApplyChrome(bool),
    /// `ResizeAboutCentre(w, h)`: the page area becomes w x h, the window staying where its centre is
    Resize { w: i32, h: i32, reply: Reply<()> },
    /// `Close()`
    Close,
    /// The quest panel (a small always-on-top window over the game, panel.html): open it (near the game window `near`,
    /// when known) or hide it.
    Panel { open: bool, near: Option<isize> },
    /// A message for the quest panel's page (the live quest progress, as JSON text).
    PanelPost(String),
    /// The panel's page asks to be moved: start dragging its window with the mouse button that is down.
    PanelDrag,
}

#[derive(Clone)]
pub struct Ui {
    proxy: EventLoopProxy<UserEvent>,
}

impl Ui {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Ui {
        Ui { proxy }
    }

    fn send(&self, e: UserEvent) -> Result<(), String> {
        self.proxy.send_event(e).map_err(|_| "The window is closed.".to_string())
    }

    pub fn post(&self, json: String) {
        let _ = self.send(UserEvent::Post(json));
    }

    pub fn exec(&self, script: &str) -> Result<String, String> {
        let (tx, rx) = mpsc::channel();
        self.send(UserEvent::Exec { script: script.to_string(), reply: tx })?;
        rx.recv().map_err(|_| "The window is closed.".to_string())?
    }

    pub fn capture_png(&self) -> Result<Vec<u8>, String> {
        let (tx, rx) = mpsc::channel();
        self.send(UserEvent::Capture { reply: tx })?;
        rx.recv().map_err(|_| "The window is closed.".to_string())?
    }

    pub fn dev_tools(&self, method: &str, params: &str) -> Result<String, String> {
        let (tx, rx) = mpsc::channel();
        self.send(UserEvent::DevTools { method: method.to_string(), params: params.to_string(), reply: Some(tx) })?;
        rx.recv().map_err(|_| "The window is closed.".to_string())?
    }

    /// Start a DevTools protocol call and do not wait for it (the C# `_ = core.CallDevToolsProtocolMethodAsync(...)`).
    pub fn dev_tools_nowait(&self, method: &str, params: &str) {
        let _ = self.send(UserEvent::DevTools { method: method.to_string(), params: params.to_string(), reply: None });
    }

    pub fn dev_tools_events(&self, name: &str, sink: EventSink) {
        let _ = self.send(UserEvent::DevToolsEvents { name: name.to_string(), sink });
    }

    pub fn set_scheme(&self, theme: Option<String>) {
        let _ = self.send(UserEvent::SetScheme(theme));
    }

    pub fn apply_chrome(&self, dark: bool) {
        let _ = self.send(UserEvent::ApplyChrome(dark));
    }

    /// A live tour's "resize" step (C# `MainForm.ResizeAboutCentre`): returns when the window has its new size.
    pub fn resize_about_centre(&self, w: i32, h: i32) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.send(UserEvent::Resize { w, h, reply: tx })?;
        rx.recv().map_err(|_| "The window is closed.".to_string())?
    }

    pub fn close(&self) {
        let _ = self.send(UserEvent::Close);
    }

    pub fn panel(&self, open: bool, near: Option<isize>) {
        let _ = self.send(UserEvent::Panel { open, near });
    }

    pub fn panel_post(&self, json: String) {
        let _ = self.send(UserEvent::PanelPost(json));
    }

    pub fn panel_drag(&self) {
        let _ = self.send(UserEvent::PanelDrag);
    }
}
