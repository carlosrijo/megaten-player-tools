//! Stamps the version, git commit and build date into newmoon-reader.exe (see crates\newmoon\build_info.rs).

include!("../newmoon/build_info.rs");

fn main() {
    emit_build_info();
}
