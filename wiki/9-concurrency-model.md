# 9. Concurrency Model

Structured concurrency with coroutines, no preemption, no locks needed for scoped data. Only `await` yields; scheduler is cooperative.

Coroutines allowed only in `scope` blocks. Each scope has a region; references cannot escape. If any coroutine throws an error, the scope cancels all other running coroutines and propagates the error. Optional explicit timeout or external cancellation token. 

Sharing by:

* `Shared<T>`: immutable, read-only, reference-counted, cross coroutines.
* `Channel<T>`: send/receive messages between coroutines. 

```rust
fn fetch_all(urls: List<String>) -> <async, net, exn> List<Response> {
  scope s {
    let handles = urls.map(|url| s.spawn(async { fetch(url) }));
    handles.map(|h| h.await)
  }
}

scope s {
  let data = load();               // in region s
  s.spawn(async { process(&data) }); // borrow safe; scope exits → drop data
}

scope s with timeout(5.seconds) { ... }
scope s with cancellable(token) { ... }

let cfg = Shared.new(config);      // across tasks
let ch = Channel.new();
```

Coroutines join at scope exit. No data races by construction.
