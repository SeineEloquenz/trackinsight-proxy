use anyhow::{Context, Result};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, USER_AGENT as USER_AGENT_HEADER},
    RequestBuilder,
};
use serde_json::{json, Value};

use super::Issuers;
use crate::breakdown::{Breakdown, Holding};

const API: &str = "https://www.ubs.com/app/HA4/api";
const LOCALE: &str = "en_CH_RETL";
/// UBS's CDN rejects HTTP/1.1, requests without an Accept header and a Chrome user agent without Chrome's client hints.
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0";
const SEGMENT: &str = "etf.emwh";
const CONSTITUENTS: &str = "query GetConstituentsExcelv2($instId: String!, $sgmtKey: String!, $locale: String!) { getConstituentsExcel(instId: $instId, sgmtKey: $sgmtKey, locale: $locale) { etfFundHoldingsLargestConstituents { row { type cell { position data } } } } }";

/// UBS publishes only the collateral basket of swap ETFs, which says nothing about their index, so those are skipped.
pub async fn fetch(issuers: &Issuers, isin: &str) -> Result<Option<Breakdown>> {
    let token = issuers
        .http
        .get(format!("{API}/api/token-service/get-token"))
        .header(USER_AGENT_HEADER, USER_AGENT)
        .header(ACCEPT, "*/*")
        .header("locale", LOCALE)
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?["token"]
        .as_str()
        .context("UBS token service returned no token")?
        .to_owned();
    let auth = format!("Bearer {token}");
    let call = |req: RequestBuilder| async {
        Ok::<Value, anyhow::Error>(
            req.header(USER_AGENT_HEADER, USER_AGENT)
                .header(ACCEPT, "*/*")
                .header(AUTHORIZATION, &auth)
                .header("locale", LOCALE)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?,
        )
    };

    let found = call(
        issuers
            .http
            .post(format!(
                "{API}/api/etf-funddetail-services/etfinstidfromisin"
            ))
            .json(&json!({ "isin": isin.to_lowercase(), "locale": LOCALE, "sgmtKey": SEGMENT })),
    )
    .await?;
    let Some(id) = found["instId"].as_str() else {
        return Ok(None);
    };

    let replication = call(issuers.http.get(format!(
        "{API}/api/etf-funddetail-services/{id}/replication-type?locale={LOCALE}"
    )))
    .await?;
    if replication["replicationType"]
        .as_str()
        .is_some_and(|r| r.eq_ignore_ascii_case("synthetic"))
    {
        tracing::info!(isin, "skipping UBS swap ETF");
        return Ok(None);
    }

    let doc = call(issuers.http.post(format!("{API}/graphql/")).json(&json!({
        "operationName": "GetConstituentsExcelv2",
        "query": CONSTITUENTS,
        "variables": { "instId": id, "sgmtKey": SEGMENT, "locale": LOCALE },
    })))
    .await?;
    let mut holdings = parse(&doc)?;
    issuers.fill_from_reference(&mut holdings).await?;
    Ok(Some(Breakdown::from_holdings(holdings)))
}

fn parse(doc: &Value) -> Result<Vec<Holding>> {
    let rows = doc["data"]["getConstituentsExcel"]["etfFundHoldingsLargestConstituents"]["row"]
        .as_array()
        .context("UBS constituents without rows")?;
    let cells = |row: &Value| -> Vec<(String, String)> {
        row["cell"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| {
                Some((
                    c["position"].as_str()?.to_owned(),
                    c["data"].as_str()?.trim().to_owned(),
                ))
            })
            .collect()
    };
    let header = rows
        .iter()
        .find(|r| {
            r["type"]
                .as_str()
                .is_some_and(|t| t.eq_ignore_ascii_case("header"))
        })
        .map(cells)
        .context("UBS constituents without a header")?;
    let column = |prefix: &str| {
        header
            .iter()
            .find(|(_, label)| label.to_lowercase().starts_with(prefix))
            .map(|(pos, _)| pos.clone())
    };
    let name = column("securit").context("UBS constituents without names")?;
    let weight = column("weight").context("UBS constituents without weights")?;
    let isin = column("isin");
    let holdings = rows
        .iter()
        .filter(|r| {
            r["type"]
                .as_str()
                .is_some_and(|t| t.eq_ignore_ascii_case("data"))
        })
        .filter_map(|r| {
            let cells = cells(r);
            let get = |pos: &str| {
                cells
                    .iter()
                    .find(|(p, _)| p == pos)
                    .map(|(_, d)| d.as_str())
            };
            Some(Holding {
                name: get(&name)?.to_owned(),
                isin: isin.as_deref().and_then(get).map(str::to_owned),
                weight: get(&weight)?.replace(',', "").parse().ok()?,
                country: None,
                sector: None,
            })
        })
        .collect();
    Ok(holdings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_columns_by_header() {
        let row = |kind: &str, cells: &[&str]| {
            json!({ "type": kind, "cell": cells.iter().enumerate()
                .map(|(i, d)| json!({ "position": (i + 1).to_string(), "data": d }))
                .collect::<Vec<_>>() })
        };
        let doc = json!({ "data": { "getConstituentsExcel": { "etfFundHoldingsLargestConstituents": { "row": [
            row("header", &["Securities", "ISIN", "Sedol Code", "Currency", "Price", "Weight %"]),
            row("data", &["APPLE INC", "US0378331005", "2046251", "USD", "349.08", "6.05647"]),
            row("data", &["NVIDIA CORP", "US67066G1040", "2379504", "USD", "165.05", "4.48289"]),
        ] } } } });
        let holdings = parse(&doc).unwrap();
        assert_eq!(holdings.len(), 2);
        assert_eq!(holdings[0].name, "APPLE INC");
        assert_eq!(holdings[0].isin.as_deref(), Some("US0378331005"));
        assert_eq!(holdings[1].weight, 4.48289);
    }
}
