//! The two file dialogs the page can ask for (C# `OpenFileDialog` / `SaveFileDialog`), made with the Windows
//! common item dialogs and modal to the app's window. They block the calling (worker) thread, not the window's.

use std::path::{Path, PathBuf};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::*;
use windows::core::{HSTRING, PCWSTR};

struct ComScope(bool);
impl ComScope {
    fn new() -> ComScope {
        // the worker thread may not have COM yet; S_FALSE (already initialized) is fine too
        ComScope(unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() })
    }
}
impl Drop for ComScope {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() }
        }
    }
}

fn result_path(dialog: &IFileDialog) -> Option<PathBuf> {
    unsafe {
        let item = dialog.GetResult().ok()?;
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let s = name.to_string().ok();
        CoTaskMemFree(Some(name.as_ptr() as *const _));
        s.map(PathBuf::from)
    }
}

fn folder_item(dir: &Path) -> Option<IShellItem> {
    unsafe { SHCreateItemFromParsingName(&HSTRING::from(dir.as_os_str()), None::<&IBindCtx>).ok() }
}

/// `OpenFileDialog { Title, Filter = "Images (*.png;*.jpg;*.jpeg;*.bmp;*.gif)|*.png;*.jpg;*.jpeg;*.bmp;*.gif" }`
pub fn open_image(owner: HWND, title: &str) -> Option<PathBuf> {
    let _com = ComScope::new();
    unsafe {
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let base: IFileDialog = windows::core::Interface::cast(&dialog).ok()?;
        let name = HSTRING::from("Images (*.png;*.jpg;*.jpeg;*.bmp;*.gif)");
        let spec = HSTRING::from("*.png;*.jpg;*.jpeg;*.bmp;*.gif");
        let filters = [COMDLG_FILTERSPEC { pszName: PCWSTR(name.as_ptr()), pszSpec: PCWSTR(spec.as_ptr()) }];
        base.SetFileTypes(&filters).ok()?;
        base.SetTitle(&HSTRING::from(title)).ok()?;
        let opts = base.GetOptions().ok()?;
        base.SetOptions(opts | FOS_FORCEFILESYSTEM | FOS_FILEMUSTEXIST | FOS_PATHMUSTEXIST).ok()?;
        base.Show(Some(owner)).ok()?; // cancel is an error (HRESULT_FROM_WIN32(ERROR_CANCELLED))
        result_path(&base)
    }
}

/// `SaveFileDialog { Title, Filter = "CSV files (*.csv)|*.csv", OverwritePrompt = false, FileName, InitialDirectory }`:
/// an existing file is added to, never replaced, so there is no "replace?" question.
pub fn save_csv(owner: HWND, title: &str, file_name: &str, initial_dir: Option<&Path>) -> Option<PathBuf> {
    let _com = ComScope::new();
    unsafe {
        let dialog: IFileSaveDialog = CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let base: IFileDialog = windows::core::Interface::cast(&dialog).ok()?;
        let name = HSTRING::from("CSV files (*.csv)");
        let spec = HSTRING::from("*.csv");
        let filters = [COMDLG_FILTERSPEC { pszName: PCWSTR(name.as_ptr()), pszSpec: PCWSTR(spec.as_ptr()) }];
        base.SetFileTypes(&filters).ok()?;
        base.SetTitle(&HSTRING::from(title)).ok()?;
        base.SetDefaultExtension(&HSTRING::from("csv")).ok()?;
        base.SetFileName(&HSTRING::from(file_name)).ok()?;
        let mut opts = base.GetOptions().ok()?;
        opts &= !FOS_OVERWRITEPROMPT;
        base.SetOptions(opts | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST).ok()?;
        if let Some(dir) = initial_dir {
            if let Some(item) = folder_item(dir) {
                let _ = base.SetFolder(&item);
            }
        }
        base.Show(Some(owner)).ok()?;
        result_path(&base)
    }
}
