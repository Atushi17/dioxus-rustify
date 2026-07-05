fn main() {
    let _ = dotenvy::from_filename(".env");

    emit_env("API_BASE_URL");
    emit_env("COMPILE_ID");
    emit_env("PROJECT_ID");
    emit_env("HOSTED_BASE_PATH");
    emit_env("WEBSITE_TITLE");

    println!("cargo:rerun-if-changed=.env");
}

fn emit_env(key: &str) {
    if let Ok(value) = std::env::var(key) {
        println!("cargo:rustc-env={}={}", key, value);
    }
}
