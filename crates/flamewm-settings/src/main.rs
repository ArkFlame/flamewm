fn main() {
    if let Err(error) = flamewm_settings::runtime::run() {
        eprintln!("flamewm-settings: {error}");
        std::process::exit(1);
    }
}
