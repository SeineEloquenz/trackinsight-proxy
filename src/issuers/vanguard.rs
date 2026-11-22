use std::collections::{BTreeMap, HashMap};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use super::Issuers;
use crate::breakdown::Breakdown;

const PRODUCTS: &str = "https://global.vanguard.com/de-de/investment-products?product-type=etf";
const GRAPHQL: &str = "https://global.vanguard.com/gpx/graphql";
/// The site's public client id. The API rejects requests without it.
const CONSUMER_ID: &str = "de7";

const ISINS: &str = r#"query ($portIds: [String!]!) {
  funds(portIds: $portIds) { profile { portId identifiers(altIds: ["ISIN"]) { altIdValue } } }
}"#;

const BREAKDOWN: &str = r#"query ($portIds: [String!]!) {
  funds(portIds: $portIds) {
    profile { primaryMarketEquityClassification }
    marketAllocation { countryCode countryName fundMktPercent holdingStatCode }
    sectorDiversification { sectorName fundPercent }
  }
  borHoldings(portIds: $portIds) { holdings(limit: 10) { items { issuerName marketValuePercentage } } }
}"#;

pub async fn fetch(issuers: &Issuers, isin: &str) -> Result<Option<Breakdown>> {
    let funds = issuers.vanguard.get(|| funds(issuers)).await?;
    let Some(port_id) = funds.get(isin) else {
        return Ok(None);
    };
    let data = query(issuers, BREAKDOWN, &[port_id.as_str()]).await?;
    Ok(Some(parse(&data)))
}

async fn funds(issuers: &Issuers) -> Result<HashMap<String, String>> {
    let page = issuers
        .http
        .get(PRODUCTS)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let start = page
        .find(r#""portIds":""#)
        .context("no portIds on the Vanguard product page")?
        + 11;
    let end = start + page[start..].find('"').context("unterminated portIds")?;
    let port_ids: Vec<&str> = page[start..end]
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    let data = query(issuers, ISINS, &port_ids).await?;
    let funds: HashMap<String, String> = data["funds"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|f| {
            let profile = &f["profile"];
            let port_id = profile["portId"].as_str().unwrap_or_default().to_owned();
            profile["identifiers"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(move |id| {
                    Some((id["altIdValue"].as_str()?.to_owned(), port_id.clone()))
                })
        })
        .collect();
    tracing::info!(funds = funds.len(), "loaded Vanguard funds");
    Ok(funds)
}

async fn query(issuers: &Issuers, query: &str, port_ids: &[&str]) -> Result<Value> {
    let mut doc: Value = issuers
        .http
        .post(GRAPHQL)
        .header("x-consumer-id", CONSUMER_ID)
        .json(&json!({ "query": query, "variables": { "portIds": port_ids } }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if let Some(errors) = doc.get("errors").filter(|e| !e.is_null()) {
        bail!("Vanguard GraphQL errors: {errors}");
    }
    Ok(doc["data"].take())
}

fn parse(data: &Value) -> Breakdown {
    let fund = &data["funds"][0];
    let pairs = |list: &Value,
                 name: &dyn Fn(&Value) -> Option<String>,
                 weight: &str|
     -> Vec<(String, f64)> {
        list.as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| Some((name(e)?, e[weight].as_f64()?)))
            .collect()
    };
    Breakdown {
        countries: pairs(
            &countries(fund),
            &|e| {
                e["countryName"]
                    .as_str()
                    .or(e["countryCode"].as_str())
                    .map(str::to_owned)
            },
            "fundMktPercent",
        ),
        sectors: pairs(
            &fund["sectorDiversification"],
            &|e| e["sectorName"].as_str().map(str::to_owned),
            "fundPercent",
        ),
        top: pairs(
            &data["borHoldings"][0]["holdings"]["items"],
            &|e| e["issuerName"].as_str().map(str::to_owned),
            "marketValuePercentage",
        ),
    }
}

/// The allocation under the classification scheme the fund uses.
fn countries(fund: &Value) -> Value {
    let scheme: String = fund["profile"]["primaryMarketEquityClassification"]
        .as_str()
        .unwrap_or_default()
        .chars()
        .take(2)
        .collect::<String>()
        .to_uppercase();
    let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for e in fund["marketAllocation"].as_array().into_iter().flatten() {
        if e["countryCode"].is_string() || e["countryName"].is_string() {
            let code = e["holdingStatCode"].as_str().unwrap_or_default().to_owned();
            groups.entry(code).or_default().push(e.clone());
        }
    }
    let chosen = groups
        .iter()
        .find(|(code, _)| !scheme.is_empty() && code.starts_with(&scheme))
        .or_else(|| groups.iter().max_by_key(|(_, entries)| entries.len()))
        .map(|(_, entries)| entries.clone())
        .unwrap_or_default();
    Value::Array(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_fund_classification() {
        let data = json!({
            "funds": [{
                "profile": { "primaryMarketEquityClassification": "FTSE Country of Risk" },
                "marketAllocation": [
                    { "countryCode": "US", "countryName": "United States", "fundMktPercent": 60.0, "holdingStatCode": "FTCTYATPCS" },
                    { "countryCode": "US", "fundMktPercent": 61.0, "holdingStatCode": "MSCTYATPCS" },
                    { "countryCode": null, "countryName": null, "fundMktPercent": 9.0, "holdingStatCode": "SASTTYPPC" },
                ],
                "sectorDiversification": [{ "sectorName": "Technology", "fundPercent": 30.0 }],
            }],
            "borHoldings": [{ "holdings": { "items": [{ "issuerName": "Apple Inc", "marketValuePercentage": 4.7 }] } }],
        });
        let b = parse(&data);
        assert_eq!(b.countries, vec![("United States".to_owned(), 60.0)]);
        assert_eq!(b.top[0].0, "Apple Inc");
    }
}
