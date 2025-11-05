fn main() {
    #[cfg(feature = "web")]
    {
        use std::{env, fs, path::Path, process::Command};

        let root = format!("{}/../../..", env::var("CARGO_MANIFEST_DIR").unwrap());
        let audio_worklet_path =
            format!("{root}/crates/audio_stream/src/wasm/audio_stream_worklet.js");

        println!("cargo:rerun-if-changed={root}/crates/audio_module/");
        println!("cargo:rerun-if-changed={root}/crates/freeverb_module/");
        println!("cargo:rerun-if-changed={audio_worklet_path}");

        fs::copy(audio_worklet_path, "public/audio_stream_worklet.js")
            .expect("Failed to copy audio_stream_worklet.js");

        if !Command::new("cargo")
            .args(&[
                "build",
                "-p",
                "freeverb_module",
                "--target",
                "wasm32-unknown-unknown",
                "--lib",
            ])
            .status()
            .expect("Failed to call cargo build")
            .success()
        {
            panic!("cargo build failed for freeverb_module");
        }

        let wasm_path = format!("{root}/target/wasm32-unknown-unknown/debug/freeverb_module.wasm",);
        if !Command::new("wasm-bindgen")
            .args(&[
                "--out-dir",
                "public",
                "--out-name",
                "freeverb",
                "--no-modules",
                "--no-typescript",
                &wasm_path,
            ])
            .status()
            .expect("Failed to call wasm-bindgen")
            .success()
        {
            panic!("cargo build failed for freeverb_module");
        }
    }
}
