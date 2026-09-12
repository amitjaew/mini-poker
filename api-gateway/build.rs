fn main() {
    println!("cargo:rerun-if-changed=.env");

    if let Ok(iter) = dotenvy::dotenv_iter() {
        for item in iter {
            if let Ok((key, value)) = item {
                println!("cargo:rustc-env={key}={value}");
            }
        }
    }
}
