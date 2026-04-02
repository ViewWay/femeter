#![allow(unused)]

/* ================================================================== */
/*  build.rs — FeMeter 固件构建脚本                                     */
/*                                                                    */
/*  编译 FreeRTOS C 内核 + hooks                                      */
/* ================================================================== */

fn main() {
    // 只在 freertos feature 启用时编译 FreeRTOS
    if std::env::var("CARGO_FEATURE_FREERTOS").is_ok() {
        build_freertos();
    }
}

fn build_freertos() {
    let freertos_dir = "third_party/freertos";
    let port_dir = format!("{}/portable/GCC/ARM_CM0", freertos_dir);

    // FreeRTOS C 源码
    let freertos_sources = [
        "tasks.c",
        "queue.c",
        "list.c",
        "timers.c",
        "event_groups.c",
        "stream_buffer.c",
    ];

    cc::Build::new()
        .target("thumbv6m-none-eabi")
        .flag("-mcpu=cortex-m0plus")
        .flag("-mthumb")
        .file("freertos_hooks.c")
        .includes(&[
            "third_party/freertos/include",
            &port_dir,
            ".",  // FreeRTOSConfig.h is here
        ])
        .warnings_into_errors(false)
        .opt_level_str("s")  // size optimization
        .compile("freertos_hooks");

    let mut build = cc::Build::new();
    build
        .target("thumbv6m-none-eabi")
        .flag("-mcpu=cortex-m0plus")
        .flag("-mthumb");

    for src in &freertos_sources {
        build.file(format!("{}/{}", freertos_dir, src));
    }

    // Portable layer
    build
        .file(format!("{}/port.c", port_dir))
        .file(format!("{}/portasm.c", port_dir));

    // Heap manager
    build.file(format!("{}/portable/MemMang/heap_4.c", freertos_dir));

    build
        .includes(&[
            "third_party/freertos/include",
            &port_dir,
            ".",  // FreeRTOSConfig.h
        ])
        .warnings_into_errors(false)
        .opt_level_str("s")
        .compile("freertos_kernel");

    // Link with FreeRTOS
    println!("cargo:rustc-link-lib=static=freertos_kernel");
    println!("cargo:rustc-link-lib=static=freertos_hooks");

    // Link libgcc for compiler builtins (__gnu_thumb1_case_uqi etc.)
    // Find arm-none-eabi libgcc path
    let output = std::process::Command::new("arm-none-eabi-gcc")
        .args(["-print-libgcc-file-name"])
        .output()
        .expect("arm-none-eabi-gcc not found");
    let libgcc_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!("cargo:rustc-link-search=native={}", std::path::Path::new(&libgcc_path).parent().unwrap().display());
    println!("cargo:rustc-link-lib=static=gcc");

    // Rebuild if FreeRTOS sources or config change
    println!("cargo:rerun-if-changed=third_party/freertos");
    println!("cargo:rerun-if-changed=FreeRTOSConfig.h");
    println!("cargo:rerun-if-changed=freertos_hooks.c");
}
