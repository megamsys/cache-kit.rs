use std::fs;

fn main() {
    // Read VERSION file
    let version_file = fs::read_to_string("VERSION")
        .expect("VERSION file not found - run: echo '0.9.0' > VERSION");

    let version = version_file.trim();

    // Get Cargo.toml version
    let cargo_version = env!("CARGO_PKG_VERSION");

    // Validate they match
    if version != cargo_version {
        panic!(
            "\n\n\
            ❌ VERSION MISMATCH!\n\
            VERSION file: {}\n\
            Cargo.toml:   {}\n\n\
            Run: make version-bump VERSION={}\n\n",
            version, cargo_version, version
        );
    }

    println!("cargo:rerun-if-changed=VERSION");
}
