# karga-http
[![crates.io](https://img.shields.io/crates/v/karga_http.svg)](https://crates.io/crates/karga_http)

**karga-http** is the implementation of the main web protocols for the [karga](https://github.com/karga-rs/karga) load testing framework.

It provides a simple and easy way to define and execute HTTP-based scenarios using `reqwest` under the hood.

---

## Installation

Add both `karga` and `karga-http` to your `Cargo.toml`:

```toml
[dependencies]
karga = "*"
karga-http = "*"
```

---

## Features

### `http_action`

A helper function that wraps an HTTP request into a `karga` action. It measures latency, success, and bytes transferred, and feeds them into `HttpAggregate`.

### `Analysis pipeline`
Implements `HttpMetric`, `HttpAggregate` and `HttpReport`, providing the most useful data points for analysing http performance.

---

## License

Licensed under MIT. See [LICENSE](./LICENSE) for details.
