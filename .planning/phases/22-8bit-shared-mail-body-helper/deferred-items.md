
## From Plan 22-01

- **Pre-existing clippy warning in `genossi_mail/src/worker.rs:105`** (`clippy::unnecessary_sort_by`). Discovered while running `cargo clippy -p genossi_mail --all-targets --all-features -- -D warnings` as part of 22-01 acceptance. Warning is unrelated to Plan 22-01's changes (worker.rs was not modified). Suggested fix: `matches.sort_by_key(|b| std::cmp::Reverse(b.created));`. Recommend a follow-up `gsd-quick` if `-D warnings` is desired workspace-wide.
