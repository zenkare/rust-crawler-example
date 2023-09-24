use anyhow::Result;
use futures::{FutureExt, Stream, StreamExt};
use hyper::{
    client::{connect::dns::GaiResolver, HttpConnector, ResponseFuture},
    http, Body, Client, Request,
};
use hyper_timeout::TimeoutConnector;
use hyper_tls::HttpsConnector;
use lazy_static::lazy_static;
use scraper::{Html, Selector};
use std::task::{Context, Poll};
use std::{collections::HashSet, pin::Pin};
use std::{collections::VecDeque, future::Future};

lazy_static! {
    static ref A_SELECTOR: Selector = Selector::parse("a").unwrap();
}

pub struct Message {
    pub url: String,
    pub method: String,
    pub status: Option<u16>,
    pub response_body: Option<Vec<u8>>,
}

type PendingMessage = Pin<Box<dyn Future<Output = Result<Message>>>>;

type HyperClient = Client<TimeoutConnector<HttpsConnector<HttpConnector<GaiResolver>>>>;

const MAX_CONCURRENT: usize = 50;

pub struct Crawler<'c> {
    client: &'c HyperClient,
    queue: VecDeque<String>,
    pending: Vec<PendingMessage>,
    unique: HashSet<String>, // Replace with bloom filter.
}

impl<'c> Crawler<'c> {
    pub fn new(client: &'c HyperClient) -> Crawler {
        Crawler {
            client,
            queue: VecDeque::new(),
            pending: Vec::new(),
            unique: HashSet::new(),
        }
    }

    pub fn seed(&mut self, url: &str) {
        self.queue.push_back(url.into());
        self.unique.insert(url.into());
    }

    fn new_message(uri: &str, method: &str) -> Message {
        Message {
            url: uri.to_owned(),
            method: method.to_owned(),
            status: None,
            response_body: None,
        }
    }

    fn create_request(msg: &Message) -> Result<hyper::Request<Body>, http::Error> {
        Request::builder()
            .uri(&msg.url)
            .method(msg.method.as_str())
            .body(Body::empty())
    }

    async fn process_response(mut msg: Message, fut: ResponseFuture) -> Result<Message> {
        let res = fut.await?;
        let (head, mut body) = res.into_parts();
        if let Some(ct) = head.headers.get("content-type") {
            let content_type = ct.to_str()?;
            if content_type.contains("text/html") {
                let mut bytes = Vec::new();
                while let Some(chunk) = body.next().await {
                    bytes.extend(chunk?);
                }
                msg.response_body = Some(bytes);
            }
        }
        msg.status = Some(head.status.into());
        return Ok(msg);
    }

    fn queue_link(&mut self, link: &str) {
        if !self.unique.contains(link) {
            self.unique.insert(link.into());
            self.queue.push_back(link.into());
        }
    }

    fn extract_and_queue(&mut self, msg: &Message) {
        let doc = Html::parse_document(&String::from_utf8_lossy(
            msg.response_body.as_ref().unwrap_or(&Vec::<u8>::new()),
        ));
        // Ignores errors in doc.errors. Best case dom is built.
        for element in doc.select(&A_SELECTOR) {
            if let Some(link) = element.value().attr("href") {
                self.queue_link(link);
            }
        }
    }
}

impl<'c> Stream for Crawler<'c> {
    type Item = Result<Message>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let slf = self.get_mut();
        // Exit condition.
        if slf.pending.is_empty() && slf.queue.is_empty() {
            return Poll::Ready(None);
        }
        // Transfer more to pending if not full.
        for _ in 0..MAX_CONCURRENT - slf.pending.len() {
            if let Some(url) = slf.queue.pop_front() {
                let msg = Crawler::new_message(&url, "GET");
                let req = Crawler::create_request(&msg)?;
                let fut = slf.client.request(req);
                slf.pending
                    .push(Box::pin(Crawler::process_response(msg, fut)));
            }
        }
        // Poll all pending http requests and remove the ones that are done.
        for (i, fut) in slf.pending.iter_mut().enumerate() {
            if let Poll::Ready(result) = fut.poll_unpin(cx) {
                slf.pending.swap_remove(i); // Future is complete.
                match result {
                    Ok(msg) => {
                        slf.extract_and_queue(&msg);
                        return Poll::Ready(Some(Ok(msg)));
                    }
                    Err(e) => return Poll::Ready(Some(Err(e))),
                }
            }
        }
        return Poll::Pending;
    }
}
