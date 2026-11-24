//! Stamps the version, git commit and build date into imagine-reader.exe (see crates\imagine\build_info.rs).

include!("../imagine/build_info.rs");

fn main() {
    emit_build_info();
}
