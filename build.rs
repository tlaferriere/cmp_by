fn main() {
    // Enable "nightly" cfg if the current compiler is nightly.
    // Spans in nightly are different, so we must be able to check this in UI tests
    if rustc_version::version_meta().unwrap().channel == rustc_version::Channel::Nightly {
        println!("cargo:rustc-cfg=nightly");
    }
}
