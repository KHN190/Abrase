# 9. Concurrency Model

Concurrency in `.ect` is built on **coroutines** (green threads), not OS threads. The VM runs a cooperative scheduler — coroutines yield at `await` points, not via preemption. This keeps the VM minimal and makes concurrency fully expressible through the effect system.

## Structured Concurrency

Coroutines must be spawned inside a `scope` block. All coroutines must complete or be cancelled before the scope exits — no coroutine outlives its scope.

```
fn fetch_all(urls: List<String>) -> <async, net, exn> List<Response> {
  scope s {
    let handles = urls.map(|url| s.spawn(async { fetch(url) }));
    handles.map(|h| h.await)
  }
}
```

`s.spawn(expr)` schedules a coroutine on the VM's run queue and returns a handle. The coroutine runs cooperatively — it yields control at every `await`.

## Scope Options

```
scope s {
  // default: any coroutine fails → scope cancels others and propagates error
}

scope s with timeout(5.seconds) {
  // timeout cancels all coroutines in scope
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

`await` is only valid within an `<async>` effect context. It is the only yield point — the scheduler cannot preempt between awaits.

## Region and lifetime

Each coroutine spawned in a scope inherits that scope's region. References cannot escape the scope boundary, which eliminates data races by construction — no locks needed for scoped borrows.

```
scope s {
  let data = load();               // data lives in s's region
  s.spawn(async { process(&data) }); // borrow is valid: coroutine can't outlive s
}
// data freed here, all coroutines already joined
```

## Data Sharing

| Method | Use case |
|--------|----------|
| `Shared<T>` | Immutable data shared across coroutines |
| `Channel<T>` | Message passing between coroutines |
| Scoped borrow | Coroutine borrows outer data within scope lifetime |

## Cancellation Semantics

Scope cancellation:
1. All running coroutines suspend at their next `await` point
2. Resources held by coroutines are dropped in reverse spawn order
3. Cancellation propagates as `<exn<Cancelled>>`
