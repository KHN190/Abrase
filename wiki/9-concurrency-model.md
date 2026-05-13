# 9. Concurrency Model

## Structured Concurrency

Concurrent tasks must be within `scope` blocks. All tasks must join or be cancelled before scope ends.

```
fn fetch_all(urls: List<String>) -> <async, net, exn> List<Response> {
  scope s {
    let handles = urls.map(|url| s.thread(async { fetch(url) }));
    handles.map(|h| h.await)
  }
}
```

## Scope Options

```
scope s {
  // default: any child task fails, scope cancels others and propagates error
}

scope s with timeout(5.seconds) {
  // timeout cancels all tasks
}

scope s with cancellable(token) {
  // external token can trigger cancellation
}
```

## await

```
async fn fetch(url: String) -> <net, exn> Response { ... }

fn use_fetch() -> <async, net, exn> Unit {
  let resp = fetch("https://example.com").await;
}
```

`await` is only valid within `<async>` effect context.

## Data Sharing

Valid ways to share data across tasks:

| Method | Use case |
|--------|----------|
| `Shared<T>` | Immutable shared data |
| Channel (`channel<T>()`) | Message passing between tasks |
| Scoped borrow | Tasks borrow outer data within scope |

## Cancellation Semantics

Scope cancellation causes:
1. All running Futures terminate at next await point
2. All Future-held resources are dropped in chain
3. Cancellation signal propagates via `<exn<Cancelled>>`
