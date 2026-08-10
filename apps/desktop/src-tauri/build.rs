fn main() {
    // ggml-metal (via transcribe-cpp) uses Objective-C `@available` checks,
    // which clang lowers to `__isPlatformVersionAtLeast` from compiler-rt.
    // Rust links with -nodefaultlibs, so that builtin never gets pulled in
    // and the release link dies with "Undefined symbols: ___isPlatformVersionAtLeast"
    // (first hit on Xcode 17 / clang 17). Link clang's builtins archive
    // explicitly.
    #[cfg(target_os = "macos")]
    {
        let resource_dir = std::process::Command::new("xcrun")
            .args(["clang", "--print-resource-dir"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
        if let Some(dir) = resource_dir {
            println!("cargo:rustc-link-search=native={dir}/lib/darwin");
            println!("cargo:rustc-link-lib=static=clang_rt.osx");
        } else {
            println!("cargo:warning=xcrun clang --print-resource-dir failed; ggml-metal link may miss __isPlatformVersionAtLeast");
        }
    }

    tauri_build::build()
}
