# trackinsight-proxy

A reverse proxy that fronts [TrackInsight](https://www.trackinsight.com)'s public data API.

## Endpoints

| Route              | Behaviour                                              |
| ------------------ | ------------------------------------------------------ |
| `GET /healthz`     | `200 OK` once the browser is up                        |
| `GET /data-api/*`  | proxied to `${TRACKINSIGHT_ORIGIN}/data-api/*`         |
| `GET /search-api/*`| proxied to `${TRACKINSIGHT_ORIGIN}/search-api/*`       |
| anything else      | `404`                                                  |

## Configuration (environment)

| Variable                | Default                          | Description                              |
| ----------------------- | -------------------------------- | ---------------------------------------- |
| `SOLVER_PORT`           | `8191`                           | Listen port                              |
| `CHROME_URL`            | `http://127.0.0.1:9222`          | CDP endpoint of the headless-Chrome sidecar |
| `TRACKINSIGHT_ORIGIN`   | `https://www.trackinsight.com`   | Upstream origin                          |
| `WARM_URL`              | `${ORIGIN}/en/etf/US/QQQ/`       | Page loaded to obtain the WAF token      |
| `WARM_DELAY_SECS`       | `6`                              | Wait after navigation before probing     |
| `RUST_LOG`              | `info`                           | Log filter                               |

> The Chrome host in `CHROME_URL` is resolved to an IP before connecting —
> Chrome's DevTools endpoint rejects `Host` headers that aren't an IP literal or
> `localhost` (DNS-rebinding protection), so connecting by service name works
> transparently.

## Run (Docker Compose)

The intended deployment is the proxy plus a headless-Chrome sidecar:

```sh
docker compose up -d
curl localhost:8191/data-api/holdings/QQQ.json
```

See [`docker-compose.yml`](docker-compose.yml).

## Build

```sh
nix build .#default                 # the binary
nix build .#image                   # the OCI image (binary only, ~60 MB)
docker load < result
```

Dev shell + local run (needs a Chrome with remote debugging):

```sh
nix develop
chromium --headless=new --remote-debugging-port=9222 &   # or any headless Chrome
cargo run                                                # CHROME_URL defaults to :9222
curl localhost:8191/healthz
```
