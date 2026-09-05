use flamewm_wm::{WmConfig, run};

fn main() {
    if let Err(error) = run(WmConfig::from_env()) {
        eprintln!("flamewm: {error}");
        std::process::exit(1);
    }
}
