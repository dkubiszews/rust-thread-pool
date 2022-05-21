![master build](https://github.com/dkubiszews/rust-thread-pool/actions/workflows/rust.yml/badge.svg)

# Overview

A strightforward thread pool implementation.

# Example

```
use thread_pool::ThreadPool;

let mut pool = ThreadPool::new(4);

pool.schedule(|| {
  println!("execute task 1");
});
pool.schedule(|| {
  println!("execute task 2");
});

pool.flush();
```