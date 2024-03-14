mod processor;
mod utils;

use clap::Parser;
use tokio::time::Instant;

#[derive(Parser)]
struct Options {
    /// Path where search starts in
    #[arg(short)]
    path: String,

    /// Should save results to file
    #[arg(long)]
    save_to_file: bool,

    /// Concurrency level. Range 1-inf. Default: 3 x num_cpus
    #[arg(short)]
    concurrency: Option<usize>,
}

#[tokio::main]
async fn main() {
    let args = Options::parse();
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
    // console_subscriber::init();
    let cpus = num_cpus::get();
    let concurrency = args.concurrency.unwrap_or_else(|| {
        let concurrency = cpus * 3;
        log::info!("Setting default concurrency to: {}", concurrency);
        concurrency
    });

    log::info!("Starting scan in '{}'", args.path);
    let start = Instant::now();
    let (results, scanned_dirs) = processor::run(args.path, concurrency)
        .await
        .expect("Processor failed to start");

    log::info!(
        "Found {} DICOM files across {} directories. Scan took {:.2?}. Processed {} files.",
        results.dicom_file_count,
        scanned_dirs,
        start.elapsed(),
        results.total_file_count,
    );

    if args.save_to_file {
        utils::save_results(results);
    } else {
        for result in results.dicom_entries {
            println!("{}", result);
        }
    }
}
