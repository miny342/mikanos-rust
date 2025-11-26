use std::process::Command;
use std::path::Path;
use std::env;

fn main() {
    // bootloaderとkernelは別のtargetである必要があり、一方でworkspaceとper-package-targetとbuild-stdを合わせるとcargoが落ちる
    // そのため、現状はbuild.rsでbootloaderをビルドすることで回避する

    let current_dir = env::current_dir().expect("Failed to get current directory");
    let bootloader_path = Path::new("../bootloader");
    env::set_current_dir(&bootloader_path).expect("Failed to change directory");
    let profile = std::env::var("PROFILE").unwrap();

    let bootloader_image_path = match profile.as_str() {
        "debug" => {
            let status = Command::new("cargo")
                .args(&["build"])
                .status()
                .expect("Failed to execute cargo build");
            assert!(status.success(), "Bootloader build failed");
            bootloader_path.join("target/x86_64-unknown-uefi/debug/mikanos-rust.efi")
        }
        "release" => {
            let status = Command::new("cargo")
                .args(&["build", "--release"])
                .status()
                .expect("Failed to execute cargo build");
            assert!(status.success(), "Bootloader build failed");
            bootloader_path.join("target/x86_64-unknown-uefi/release/mikanos-rust.efi")
        }
        _ => {
            panic!("Building with an unknown profile: {}", profile);
        }
    };
    
    env::set_current_dir(&current_dir).expect("Failed to change back to original directory");

    let out_dir = Path::new("target");
    std::fs::copy(&bootloader_image_path, out_dir.join("mikanos-rust.efi")).expect("Failed to copy bootloader image");
}
