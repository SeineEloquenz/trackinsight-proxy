# trackinsight-proxy

Serves the parts of [TrackInsight](https://www.trackinsight.com)'s API that
[Ghostfolio](https://ghostfol.io) uses to enrich ETFs, built from the holdings
the fund issuers publish themselves. It needs no configuration. Funds are found
by the ISIN in the symbol Ghostfolio asks for.

## Issuers

| Issuer    | Source                                      |
| --------- | ------------------------------------------- |
| iShares   | Product screener and holdings API           |
| Amundi    | Product API, index allocation for swap ETFs |
| Vanguard  | GraphQL API of the Vanguard site            |
| SPDR      | Fund finder and daily holdings workbook     |
| Xtrackers | Constituents workbook                       |
| Invesco   | Holdings API, index allocation for swap ETFs |
| JPMorgan  | Fund data API                               |
| HSBC      | Holdings workbook                           |
| UBS       | Constituents API, iShares fund on the index for swap ETFs |

Each issuer's fund list is loaded once a day and holdings are cached for a day.
Countries and sectors are translated to the names Ghostfolio uses, including
GICS and ICB industries. Holdings published without countries or sectors get
the ones their ISIN has in iShares MSCI World IMI or Core MSCI EM IMI. UBS
swap ETFs and UBS funds without published constituents get the holdings of the
iShares fund named after the same index, ignoring share class, currency hedging
and the index's return type.

## Endpoints

| Route                                    | Response                                                    |
| ---------------------------------------- | ----------------------------------------------------------- |
| `GET /search-api/search_v2/{symbol}/...` | the ISIN in `symbol` as the only result if an issuer has it |
| `GET /data-api/funds/{ISIN}.json`        | `{ "isins": [ISIN] }`                                       |
| `GET /data-api/holdings/{ISIN}.json`     | `weight`, `countries`, `sectors` and `topHoldings`          |
| `GET /healthz`                           | `200 OK`                                                    |

`weight` is the share of the fund with a known country and sector. Ghostfolio
ignores holdings below 0.85.

## Ghostfolio

Route TrackInsight to the proxy with the `PROXY_ROUTES` property.

```json
[{ "domain": "www.trackinsight.com", "url": "http://127.0.0.1:8191" }]
```

The ETF's symbol or its TrackInsight symbol mapping has to contain the ISIN,
such as `IE00B4L5YC18`.

## Configuration (environment)

| Variable      | Default     | Description    |
| ------------- | ----------- | -------------- |
| `LISTEN_HOST` | `127.0.0.1` | Listen address |
| `LISTEN_PORT` | `8191`      | Listen port    |
| `RUST_LOG`    | `info`      | Log filter     |

## Run (NixOS)

```nix
services.trackinsight-proxy.enable = true;
```

## Run (Docker Compose)

```sh
docker compose up -d
curl localhost:8191/data-api/holdings/IE00B4L5YC18.json
```

## Build

```sh
nix build .#default
nix build .#image
```
