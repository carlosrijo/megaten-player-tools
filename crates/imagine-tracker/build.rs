//! Windows resources of BABEL.exe: the icon (the same app\app.ico the window uses), a version resource and a
//! manifest (per-monitor DPI like the C# app's ApplicationHighDpiMode, visual styles, long paths).

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
        processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
</assembly>
"#;

// the version, git commit and build date the program reports (Settings, the log, crash reports): emit_build_info()
include!("../imagine/build_info.rs");

fn main() {
    emit_build_info();
    println!("cargo:rerun-if-changed=../../app/app.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // tools\publish-portable.ps1 style: IMAGINE_VERSION=1.2.0 stamps that version (default: the crate's version)
    let version = std::env::var("IMAGINE_VERSION").ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap_or_default());
    let mut parts = [0u64; 4];
    for (i, p) in version.split('.').take(4).enumerate() {
        parts[i] = p.trim().parse().unwrap_or(0);
    }
    let packed = (parts[0] << 48) | (parts[1] << 32) | (parts[2] << 16) | parts[3];

    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../app/app.ico");
    res.set_manifest(MANIFEST);
    res.set_version_info(winresource::VersionInfo::FILEVERSION, packed);
    res.set_version_info(winresource::VersionInfo::PRODUCTVERSION, packed);
    res.set("FileVersion", &version);
    res.set("ProductVersion", &version);
    res.set("ProductName", "BABEL");
    res.set("FileDescription", "BABEL");
    res.set("InternalName", "BABEL");
    res.set("OriginalFilename", "BABEL.exe");
    if let Err(e) = res.compile() {
        println!("cargo:warning=could not embed the Windows resources: {e}");
    }
}
