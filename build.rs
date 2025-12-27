fn main() {
    // Single-world build: FULL only.
    // Keep build output clean (no cargo:warning spam) while still exposing the
    // build variant to the crate.
    println!("cargo:rustc-env=VISION_BUILD_VARIANT=FULL");
}
