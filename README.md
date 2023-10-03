# Rust Crawler Example

An example using rust hyper directly to start crawling websites from a seed
file which contains domains. This was mainly an exercise for myself to learn how to limit
the maximum concurrent number of futures of the same type. This is very useful
for not exceeding the os file limit. The crawler struct is an async stream of
completed http messages. The crawler will run indefinitely and has no rate
limiting as this is expected to be as minimal as possible.

```
Usage: rust-crawler-example [OPTIONS] --seed-file <SEED_FILE>

Options:
  -s, --seed-file <SEED_FILE>
          Seed file to load seeds from
  -c, --concurrent-requests <CONCURRENT_REQUESTS>
          Sets the limit for concurrent requests. Default is 50
  -h, --help
          Print help
  -V, --version
          Print version
```

