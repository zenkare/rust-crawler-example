extern crate crawler_structure_example;

use clap::Parser;
use std::{fs, time::Duration};

use anyhow::Result;
use crawler_structure_example::crawler::Crawler;
use futures::StreamExt;
use hyper::{
    client::{connect::dns::GaiResolver, HttpConnector},
    Client,
};
use hyper_timeout::TimeoutConnector;
use hyper_tls::HttpsConnector;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    /// Seed file to load seeds from.
    #[arg(short, long)]
    seed_file: Option<String>,
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

    if let Some(filename) = args.seed_file {
        let mut connector = TimeoutConnector::new(HttpsConnector::new());
        connector.set_connect_timeout(Some(Duration::from_secs(5)));
        connector.set_read_timeout(Some(Duration::from_secs(30)));
        connector.set_write_timeout(Some(Duration::from_secs(30)));
        let client: Client<TimeoutConnector<HttpsConnector<HttpConnector<GaiResolver>>>> =
            Client::builder().build(connector);

        let mut crawler = Crawler::new(&client);
        for line in read_lines(&filename)?.iter_mut() {
            line.insert_str(0, "https://");
            crawler.seed(&line);
        }
        while let Some(result) = crawler.next().await {
            match result {
                Ok(msg) => println!("{}: {}", msg.url, msg.status.unwrap_or(0)),
                Err(e) => println!("{}", e),
            }
        }
    } else {
        println!("nothing to do");
    }
    Ok(())
}
