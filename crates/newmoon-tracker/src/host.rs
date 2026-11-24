//! The WebView2 control (what `Microsoft.Web.WebView2.WinForms.WebView2` did for the C# form), made directly with
//! webview2-com on the window: environment and controller, settings, the page's two hosts (served from the exe by
//! WebResourceRequested), navigation lock, the bridge's message events, CapturePreview, DevTools protocol.
//!
//! Everything here runs on the UI thread. WebView2Loader is linked into the exe (no DLL beside it).

use std::sync::{mpsc, Arc};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{
    CallDevToolsProtocolMethodCompletedHandler, CapturePreviewCompletedHandler, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, DevToolsProtocolEventReceivedEventHandler, ExecuteScriptCompletedHandler,
    NavigationStartingEventHandler, NewWindowRequestedEventHandler, WebMessageReceivedEventHandler, WebResourceRequestedEventHandler, take_pwstr,
};
use windows::Win32::Foundation::{E_POINTER, HWND, RECT};
use windows::Win32::System::Com::{IStream, STREAM_SEEK_END, STREAM_SEEK_SET};
use windows::Win32::UI::Shell::SHCreateMemStream;
use windows::core::{HSTRING, Interface, PCWSTR, PWSTR};

use crate::site::{DATA_HOST, HOST, Site};
use crate::ui::{EventSink, Reply};
use crate::winutil;

/// Called on the UI thread for each message the page posts: (the message as JSON text, the page it came from).
pub type MessageHandler = Arc<dyn Fn(String, String) + Send + Sync>;

pub struct WebHost {
    hwnd: HWND,
    env: ICoreWebView2Environment,
    controller: ICoreWebView2Controller,
    core: ICoreWebView2,
    /// DevTools protocol event receivers must stay alive for their handlers to keep being called
    receivers: Vec<ICoreWebView2DevToolsProtocolEventReceiver>,
}

fn w(e: webview2_com::Error) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}

fn bg_color(dark: bool) -> COREWEBVIEW2_COLOR {
    let (r, g, b) = winutil::chrome_rgb(dark);
    COREWEBVIEW2_COLOR { A: 255, R: r, G: g, B: b }
}

impl WebHost {
    /// Make the WebView2 control fill the window. `user_data` is the profile folder (the same folders as the C# app:
    /// %LocalAppData%\NewMoonTracker\WebView2, or ...\WebView2-capture for scripted runs).
    pub fn create(
        hwnd: HWND,
        user_data: &std::path::Path,
        dark: bool,
        dev: bool,
        site: Arc<Site>,
        on_message: MessageHandler,
    ) -> anyhow::Result<WebHost> {
        let _ = std::fs::create_dir_all(user_data);
        let env = {
            let (tx, rx) = mpsc::channel();
            let folder = HSTRING::from(user_data.as_os_str());
            CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
                Box::new(move |handler| unsafe {
                    CreateCoreWebView2EnvironmentWithOptions(PCWSTR::null(), &folder, None::<&ICoreWebView2EnvironmentOptions>, &handler)
                        .map_err(webview2_com::Error::WindowsError)
                }),
                Box::new(move |error_code, environment| {
                    error_code?;
                    tx.send(environment.ok_or_else(|| windows::core::Error::from(E_POINTER))).expect("send over mpsc channel");
                    Ok(())
                }),
            )
            .map_err(w)?;
            rx.recv().map_err(|_| anyhow::anyhow!("no WebView2 environment"))??
        };
        let controller = {
            let (tx, rx) = mpsc::channel();
            let env2 = env.clone();
            CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
                Box::new(move |handler| unsafe { env2.CreateCoreWebView2Controller(hwnd, &handler).map_err(webview2_com::Error::WindowsError) }),
                Box::new(move |error_code, controller| {
                    error_code?;
                    tx.send(controller.ok_or_else(|| windows::core::Error::from(E_POINTER))).expect("send over mpsc channel");
                    Ok(())
                }),
            )
            .map_err(w)?;
            rx.recv().map_err(|_| anyhow::anyhow!("no WebView2 controller"))??
        };
        let core = unsafe { controller.CoreWebView2()? };
        let mut host = WebHost { hwnd, env, controller, core, receivers: Vec::new() };
        host.resize();
        unsafe { host.controller.SetIsVisible(true)? };
        host.set_background(dark);
        host.configure(dev, site, on_message)?;
        Ok(host)
    }

    fn configure(&mut self, dev: bool, site: Arc<Site>, on_message: MessageHandler) -> anyhow::Result<()> {
        unsafe {
            let s = self.core.Settings()?;
            s.SetAreDefaultContextMenusEnabled(dev)?;
            s.SetAreDevToolsEnabled(dev)?;
            if let Ok(s3) = s.cast::<ICoreWebView2Settings3>() {
                s3.SetAreBrowserAcceleratorKeysEnabled(dev)?; // the page handles F5 (snapshot) and Ctrl+F (search) itself
            }
            s.SetIsStatusBarEnabled(false)?;
            if let Ok(s4) = s.cast::<ICoreWebView2Settings4>() {
                s4.SetIsPasswordAutosaveEnabled(false)?;
                s4.SetIsGeneralAutofillEnabled(false)?;
            }
            if let Ok(s6) = s.cast::<ICoreWebView2Settings6>() {
                s6.SetIsSwipeNavigationEnabled(false)?;
            }

            // the page and the data folder: https://newmoon.app/ and https://data.newmoon.app/
            for host in [HOST, DATA_HOST] {
                let pattern = HSTRING::from(format!("https://{host}/*"));
                self.core.AddWebResourceRequestedFilter(&pattern, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)?;
            }
            let env = self.env.clone();
            let mut token = 0i64;
            self.core.add_WebResourceRequested(
                &WebResourceRequestedEventHandler::create(Box::new(move |_wv, args| {
                    if let Some(args) = args {
                        answer_request(&env, &site, &args);
                    }
                    Ok(())
                })),
                &mut token,
            )?;

            // no pop-up windows
            self.core.add_NewWindowRequested(
                &NewWindowRequestedEventHandler::create(Box::new(|_wv, args| {
                    if let Some(args) = args {
                        args.SetHandled(true)?;
                    }
                    Ok(())
                })),
                &mut token,
            )?;
            // navigation is limited to the app's own origin
            self.core.add_NavigationStarting(
                &NavigationStartingEventHandler::create(Box::new(|_wv, args| {
                    if let Some(args) = args {
                        let mut uri = PWSTR::null();
                        args.Uri(&mut uri)?;
                        let uri = take_pwstr(uri);
                        if !starts_with_ic(&uri, &format!("https://{HOST}/")) {
                            args.SetCancel(true)?;
                        }
                    }
                    Ok(())
                })),
                &mut token,
            )?;
            // the page's messages (chrome.webview.postMessage)
            self.core.add_WebMessageReceived(
                &WebMessageReceivedEventHandler::create(Box::new(move |_wv, args| {
                    if let Some(args) = args {
                        let mut source = PWSTR::null();
                        let mut json = PWSTR::null();
                        if args.Source(&mut source).is_ok() && args.WebMessageAsJson(&mut json).is_ok() {
                            on_message(take_pwstr(json), take_pwstr(source));
                        }
                    }
                    Ok(())
                })),
                &mut token,
            )?;
        }
        Ok(())
    }

    pub fn navigate(&self, url: &str) -> anyhow::Result<()> {
        unsafe { self.core.Navigate(&HSTRING::from(url))? };
        Ok(())
    }

    /// Fill the window's client area (the WinForms control was Dock = Fill).
    pub fn resize(&self) {
        let (w, h) = winutil::client_size(self.hwnd);
        unsafe {
            let _ = self.controller.SetBounds(RECT { left: 0, top: 0, right: w, bottom: h });
        }
    }

    pub fn focus(&self) {
        unsafe {
            let _ = self.controller.MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);
        }
    }

    /// `DefaultBackgroundColor`
    pub fn set_background(&self, dark: bool) {
        unsafe {
            if let Ok(c2) = self.controller.cast::<ICoreWebView2Controller2>() {
                let _ = c2.SetDefaultBackgroundColor(bg_color(dark));
            }
        }
    }

    /// C# `SetScheme(theme)`: what the page's prefers-color-scheme answers (Auto follows Windows).
    pub fn set_scheme(&self, theme: Option<&str>) {
        let scheme = match theme {
            Some("dark") => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_DARK,
            Some("light") => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_LIGHT,
            _ => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_AUTO,
        };
        unsafe {
            if let Ok(wv13) = self.core.cast::<ICoreWebView2_13>() {
                if let Ok(profile) = wv13.Profile() {
                    let _ = profile.SetPreferredColorScheme(scheme);
                }
            }
        }
    }

    pub fn post_json(&self, json: &str) {
        unsafe {
            let _ = self.core.PostWebMessageAsJson(&HSTRING::from(json));
        }
    }

    pub fn exec(&self, script: &str, reply: Reply<String>) {
        let r2 = reply.clone();
        let handler = ExecuteScriptCompletedHandler::create(Box::new(move |hr, result| {
            let _ = reply.send(hr.map(|_| result).map_err(|e| e.message()));
            Ok(())
        }));
        unsafe {
            if let Err(e) = self.core.ExecuteScript(&HSTRING::from(script), &handler) {
                let _ = r2.send(Err(e.message()));
            }
        }
    }

    /// `CapturePreviewAsync(Png, stream)`: the picture of the page as PNG bytes.
    pub fn capture_png(&self, reply: Reply<Vec<u8>>) {
        let r2 = reply.clone();
        unsafe {
            let Some(stream) = SHCreateMemStream(None) else {
                let _ = r2.send(Err("could not make a stream".to_string()));
                return;
            };
            let s2 = stream.clone();
            let handler = CapturePreviewCompletedHandler::create(Box::new(move |hr| {
                let r = hr.map_err(|e| e.message()).and_then(|_| read_stream(&s2));
                let _ = reply.send(r);
                Ok(())
            }));
            if let Err(e) = self.core.CapturePreview(COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG, &stream, &handler) {
                let _ = r2.send(Err(e.message()));
            }
        }
    }

    pub fn dev_tools(&self, method: &str, params: &str, reply: Option<Reply<String>>) {
        let r2 = reply.clone();
        let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |hr, result| {
            if let Some(reply) = &reply {
                let _ = reply.send(hr.map(|_| result).map_err(|e| e.message()));
            }
            Ok(())
        }));
        unsafe {
            if let Err(e) = self.core.CallDevToolsProtocolMethod(&HSTRING::from(method), &HSTRING::from(params), &handler) {
                if let Some(r) = r2 {
                    let _ = r.send(Err(e.message()));
                }
            }
        }
    }

    pub fn dev_tools_events(&mut self, name: &str, sink: EventSink) {
        unsafe {
            if let Ok(recv) = self.core.GetDevToolsProtocolEventReceiver(&HSTRING::from(name)) {
                let mut token = 0i64;
                let _ = recv.add_DevToolsProtocolEventReceived(
                    &DevToolsProtocolEventReceivedEventHandler::create(Box::new(move |_wv, args| {
                        if let Some(args) = args {
                            let mut p = PWSTR::null();
                            if args.ParameterObjectAsJson(&mut p).is_ok() {
                                sink(take_pwstr(p));
                            }
                        }
                        Ok(())
                    })),
                    &mut token,
                );
                self.receivers.push(recv);
            }
        }
    }

    pub fn close(&self) {
        unsafe {
            let _ = self.controller.Close();
        }
    }
}

fn starts_with_ic(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.is_char_boundary(prefix.len()) && s[..prefix.len()].eq_ignore_ascii_case(prefix)
}

fn read_stream(stream: &IStream) -> std::result::Result<Vec<u8>, String> {
    unsafe {
        let mut end = 0u64;
        stream.Seek(0, STREAM_SEEK_END, Some(&mut end)).map_err(|e| e.message())?;
        stream.Seek(0, STREAM_SEEK_SET, None).map_err(|e| e.message())?;
        let mut buf = vec![0u8; end as usize];
        let mut got = 0u32;
        stream.Read(buf.as_mut_ptr() as *mut _, buf.len() as u32, Some(&mut got)).ok().map_err(|e| e.message())?;
        buf.truncate(got as usize);
        Ok(buf)
    }
}

/// Splits "https://host/path?query" into (host, path).
fn split_url(uri: &str) -> Option<(String, String)> {
    let rest = uri.strip_prefix("https://").or_else(|| uri.strip_prefix("HTTPS://"))?;
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    Some((host.to_ascii_lowercase(), path.to_string()))
}

fn answer_request(env: &ICoreWebView2Environment, site: &Site, args: &ICoreWebView2WebResourceRequestedEventArgs) {
    unsafe {
        let Ok(request) = args.Request() else { return };
        let mut uri = PWSTR::null();
        let mut method = PWSTR::null();
        if request.Uri(&mut uri).is_err() {
            return;
        }
        let uri = take_pwstr(uri);
        let method = if request.Method(&mut method).is_ok() { take_pwstr(method) } else { "GET".to_string() };
        let Some((host, path)) = split_url(&uri) else { return };
        if host != HOST && host != DATA_HOST {
            return;
        }
        let head = method.eq_ignore_ascii_case("HEAD");
        let (status, reason, body, headers): (i32, &str, std::borrow::Cow<'static, [u8]>, String) = if !(method.eq_ignore_ascii_case("GET") || head) {
            (405, "Method Not Allowed", std::borrow::Cow::Borrowed(&[][..]), "Allow: GET, HEAD".to_string())
        } else {
            match site.serve(&host, &path) {
                Some(s) => {
                    let mut h = format!("Content-Type: {}\r\nCache-Control: no-cache", s.mime);
                    if host == HOST {
                        // SetVirtualHostNameToFolderMapping(.., Allow): the page's own files may be fetched from anywhere
                        h.push_str("\r\nAccess-Control-Allow-Origin: *");
                    }
                    (200, "OK", s.body, h)
                }
                None => (404, "Not Found", std::borrow::Cow::Borrowed(&b"Not found"[..]), "Content-Type: text/plain".to_string()),
            }
        };
        let content: &[u8] = if head { &[] } else { &body };
        let stream = if content.is_empty() { SHCreateMemStream(None) } else { SHCreateMemStream(Some(content)) };
        let reason = HSTRING::from(reason);
        let headers = HSTRING::from(headers);
        if let Ok(response) = env.CreateWebResourceResponse(stream.as_ref(), status, &reason, &headers) {
            let _ = args.SetResponse(&response);
        }
    }
}
