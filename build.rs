// build.rs

#[cfg(windows)]
fn main() {
    extern crate winres;

    let res = winres::WindowsResource::new();
    res.compile().unwrap();
}
