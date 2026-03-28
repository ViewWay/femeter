use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // memory.x must be in the same OUT_DIR as cortex-m-rt's link.x
    let memory_x = include_str!("memory.x");
    let mut f = File::create(out_dir.join("memory.x")).unwrap();
    f.write_all(memory_x.as_bytes()).unwrap();

    println!("cargo:rerun-if-changed=memory.x");
}
