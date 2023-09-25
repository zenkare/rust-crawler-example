pub mod crawler;

use clap::Parser;
use std::{fs, time::Duration};

use anyhow::Result;
use crawler::Crawler;
use futures::StreamExt;
use hyper::{
    client::{connect::dns::GaiResolver, HttpConnector},
    Client,
};
use hyper_timeout::TimeoutConnector;
use hyper_tls::HttpsConnector;

const MAX_CONCURRENT_DEFAULT: u16 = 50;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    /// Seed file to load seeds from.
    #[arg(short, long)]
    seed_file: String,
    /// Sets the limit for concurrent requests. Default is 50.
    #[arg(short, long)]
    concurrent_requests: Option<u16>,
}

fn read_lines(filename: &str) -> Result<Vec<String>> {
    Ok(fs::read_to_string(filename)?
        .lines()
        .map(String::from)
        .collect())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut connector = TimeoutConnector::new(HttpsConnector::new());
    connector.set_connect_timeout(Some(Duration::from_secs(5)));
    connector.set_read_timeout(Some(Duration::from_secs(30)));
    connector.set_write_timeout(Some(Duration::from_secs(30)));
    let client: Client<TimeoutConnector<HttpsConnector<HttpConnector<GaiResolver>>>> =
        Client::builder().build(connector);

    let mut crawler = Crawler::new(
        &client,
        args.concurrent_requests.unwrap_or(MAX_CONCURRENT_DEFAULT),
    );

    for line in read_lines(&args.seed_file)?.iter_mut() {
        line.insert_str(0, "https://");
        crawler.seed(line);
    }
    while let Some(result) = crawler.next().await {
        match result {
            Ok(msg) => println!("{}: {}", msg.url, msg.status.unwrap_or(0)),
            Err(e) => println!("{}", e),
        }
    }
    Ok(())
}
