//! `Console.WriteLine` / `Console.Error.WriteLine` as .NET did it on Windows: UTF-8 text, lines ended by `\r\n`
//! (`Console.OutputEncoding = UTF8` in Program.cs). Write errors (a closed pipe) are ignored.

use std::io::Write;

const NEW_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// `Console.WriteLine(s)`.
pub fn write_line(s: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(s.as_bytes());
    let _ = out.write_all(NEW_LINE.as_bytes());
    let _ = out.flush();
}

/// `Console.WriteLine()`.
pub fn blank_line() {
    write_line("");
}

/// `Console.Write(s)`.
pub fn write(s: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(s.as_bytes());
    let _ = out.flush();
}

/// `Console.Error.WriteLine(s)`.
pub fn error_line(s: &str) {
    let mut err = std::io::stderr().lock();
    let _ = err.write_all(s.as_bytes());
    let _ = err.write_all(NEW_LINE.as_bytes());
    let _ = err.flush();
}
