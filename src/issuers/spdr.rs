use std::collections::HashMap;

use anyhow::Result;
use serde_json::Value;

use super::Issuers;
use crate::{breakdown::Breakdown, isin, sheet};

const FUND_FINDER: &str = "https://www.ssga.com/bin/v1/ssmp/fund/fundfinder?country=de&language=en_gb&role=intermediary&product=&ui=fund-finder";
const HOLDINGS: &str = "https://www.ssga.com/library-content/products/fund-data/etfs/emea";

pub async fn fetch(issuers: &Issuers, isin: &str) -> Result<Option<Breakdown>> {
    let funds = issuers.spdr.get(|| funds(issuers)).await?;
    let Some(ticker) = funds.get(isin) else {
        return Ok(None);
    };
    let bytes = issuers
        .http
        .get(format!(
            "{HOLDINGS}/holdings-daily-emea-en-{}.xlsx",
            slug(ticker)
        ))
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let holdings = sheet::holdings_from_workbook(&bytes)?;
    Ok(Some(Breakdown::from_holdings(holdings)))
}

/// Maps every ISIN in a fund's entry, including its document links, to the fund's ticker.
async fn funds(issuers: &Issuers) -> Result<HashMap<String, String>> {
    let doc: Value = issuers
        .http
        .get(FUND_FINDER)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut entries = Vec::new();
    collect(&doc, &mut entries);
    let mut funds = HashMap::new();
    for entry in entries {
        let Some(ticker) = entry["fundTicker"].as_str() else {
            continue;
        };
        for isin in isin::find_all(&entry.to_string()) {
            funds
                .entry(isin.to_owned())
                .or_insert_with(|| ticker.to_owned());
        }
    }
    tracing::info!(funds = funds.len(), "loaded SPDR funds");
    Ok(funds)
}

fn collect<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
    match v {
        Value::Object(o) if o.contains_key("fundTicker") => out.push(v),
        Value::Object(o) => o.values().for_each(|c| collect(c, out)),
        Value::Array(a) => a.iter().for_each(|c| collect(c, out)),
        _ => {}
    }
}

fn slug(ticker: &str) -> String {
    ticker
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_tickers() {
        assert_eq!(slug("SPYW GY"), "spyw-gy");
        assert_eq!(slug("SAWE IM^"), "sawe-im");
    }
}
