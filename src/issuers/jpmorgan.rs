use std::collections::HashMap;

use anyhow::Result;
use serde_json::Value;

use super::Issuers;
use crate::breakdown::{Breakdown, Holding};

const HANDLER: &str = "https://am.jpmorgan.com/FundsMarketingHandler";
const COUNTRIES: &[&str] = &["gb", "de"];

pub async fn fetch(issuers: &Issuers, isin: &str) -> Result<Option<Breakdown>> {
    let funds = issuers.jpmorgan.get(|| funds(issuers)).await?;
    let Some(country) = funds.get(isin) else {
        return Ok(None);
    };
    let doc: Value = issuers
        .http
        .get(format!("{HANDLER}/product-data?cusip={isin}&country={country}&role=per&language=en&userLoggedIn=false"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(Some(parse(&doc["fundData"])))
}

/// Maps each ISIN to the first country whose site lists it. Data is only served for listed funds.
async fn funds(issuers: &Issuers) -> Result<HashMap<String, &'static str>> {
    let mut funds = HashMap::new();
    for country in COUNTRIES {
        let list: Value = issuers
            .http
            .get(format!(
                "{HANDLER}/autocomplete?country={country}&role=per&language=en&userLoggedIn=false"
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        for isin in list
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["cusip"].as_str())
        {
            funds.entry(isin.to_owned()).or_insert(*country);
        }
    }
    tracing::info!(funds = funds.len(), "loaded JPMorgan funds");
    Ok(funds)
}

/// Countries come from the daily holdings and sectors from the sector breakdown, which the holdings lack.
fn parse(fund: &Value) -> Breakdown {
    let holdings = fund["dailyHoldingsAll"]["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|h| {
            let kind = h["securityType"].as_str().unwrap_or_default();
            Some(Holding {
                name: h["securityDescription"].as_str()?.to_owned(),
                weight: h["marketValuePercent"].as_f64()?,
                country: h["country"].as_str().map(str::to_owned),
                sector: kind.eq_ignore_ascii_case("cash").then(|| kind.to_owned()),
            })
        })
        .collect();
    let sectors = fund["emeaSectorBreakdown"]["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| Some((s["name"].as_str()?.to_owned(), s["value"].as_f64()?)))
        .collect();
    Breakdown {
        sectors,
        ..Breakdown::from_holdings(holdings)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn takes_countries_from_holdings_and_sectors_from_the_breakdown() {
        let fund = json!({
            "dailyHoldingsAll": { "data": [
                { "securityDescription": "APPLE INC", "marketValuePercent": 99.0, "country": "United States", "securityType": "Common Stock" },
                { "securityDescription": "Cash", "marketValuePercent": 1.0, "country": "United States", "securityType": "Cash" },
            ] },
            "emeaSectorBreakdown": { "data": [
                { "name": "Technology - Semi & Hardware", "value": 99.0 },
                { "name": "Total", "value": 100.0 },
            ] },
        });
        let doc = parse(&fund).render();
        assert_eq!(doc["countries"]["United States"]["weight"], 0.99);
        assert_eq!(doc["sectors"]["Technology"]["weight"], 0.99);
        assert_eq!(doc["weight"], 0.99);
    }
}
