fn main() {
    if std::env::var_os("CARGO_FEATURE_EMBED_WEB").is_some() {
        println!("cargo:rerun-if-changed=../../web/dist");
        assert!(
            std::path::Path::new("../../web/dist/index.html").is_file(),
            "embed-web requires web/dist/index.html; run `mise run build:web` first"
        );
    }
}
