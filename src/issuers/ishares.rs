use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use reqwest::header::COOKIE;
use serde_json::Value;

use super::Issuers;
use crate::breakdown::{Breakdown, Holding};

const FUNDS: &str = "https://www.ishares.com/varnish-api/blk-product-screener-server/api/v1/product-screener/product-data?country=de&language=de&siteName=de-ishares-v2&userType=individual";
const HOLDINGS: &str = "https://www.ishares.com/varnish-api/uk-retail01-product-data/product-data/api/v2/get-product-data?appSubType=ISHARES&appType=PRODUCT_PAGE&component=holdings.all&userType=individual&excludeContent=true";
/// Sites that serve English holdings, tried in order. Each serves only the funds registered there.
const SITES: &[(&str, &str)] = &[("ch-ishares-v2", "en_CH"), ("uk-ishares-v2", "en_GB")];

pub async fn fetch(issuers: &Issuers, isin: &str) -> Result<Option<Breakdown>> {
    Ok(holdings(issuers, isin).await?.map(Breakdown::from_holdings))
}

pub async fn holdings(issuers: &Issuers, isin: &str) -> Result<Option<Vec<Holding>>> {
    let funds = issuers.ishares.get(|| funds(issuers)).await?;
    let Some((id, _)) = funds.get(isin) else {
        return Ok(None);
    };
    let mut status = None;
    for (site, locale) in SITES {
        let res = issuers
            .http
            .get(format!(
                "{HOLDINGS}&portfolioId={id}&targetSite={site}&locale={locale}"
            ))
            .header(COOKIE, "investorAttestationAccepted=true")
            .send()
            .await?;
        if res.status().is_success() {
            return parse(&res.json().await?).map(Some);
        }
        status = Some(res.status());
    }
    bail!("no iShares site serves the holdings of fund {id} ({status:?})")
}

/// ISINs of the funds named after the index with this [`super::index_key`], in a stable order.
pub async fn tracking(issuers: &Issuers, key: &str) -> Result<Vec<String>> {
    let funds = issuers.ishares.get(|| funds(issuers)).await?;
    let mut isins: Vec<String> = funds
        .iter()
        .filter(|(_, (_, name))| super::index_key(name) == key)
        .map(|(isin, _)| isin.clone())
        .collect();
    isins.sort();
    Ok(isins)
}

async fn funds(issuers: &Issuers) -> Result<HashMap<String, (u64, String)>> {
    let doc: Value = issuers
        .http
        .get(FUNDS)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let funds: HashMap<String, (u64, String)> = doc
        .as_object()
        .context("iShares fund list is not an object")?
        .values()
        .filter_map(|f| {
            Some((
                f["isin"].as_str()?.to_owned(),
                (
                    f["portfolioId"].as_u64()?,
                    f["fundName"].as_str()?.to_owned(),
                ),
            ))
        })
        .collect();
    tracing::info!(funds = funds.len(), "loaded iShares funds");
    Ok(funds)
}

fn parse(doc: &Value) -> Result<Vec<Holding>> {
    let points = &doc["componentsByNameMap"]["holdings"]["containersByNameMap"]["all"]
        ["dataPointsByNameMap"];
    let column = |name: &str| points[name]["value"].as_array();
    let names = column("issueName").context("iShares holdings without names")?;
    let weights = column("holdingPercent").context("iShares holdings without weights")?;
    let text = |col: Option<&Vec<Value>>, i: usize| col?.get(i)?.as_str().map(str::to_owned);
    let (isins, countries, sectors) = (
        column("isin"),
        column("countryOfRisk"),
        column("sectorName"),
    );
    let holdings = names
        .iter()
        .enumerate()
        .filter_map(|(i, name)| {
            Some(Holding {
                name: name.as_str()?.to_owned(),
                isin: text(isins, i),
                weight: weights.get(i)?.as_f64()?,
                country: text(countries, i),
                sector: text(sectors, i),
            })
        })
        .collect();
    Ok(holdings)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_columns() {
        let col = |v: Value| json!({ "value": v });
        let doc = json!({"componentsByNameMap": {"holdings": {"containersByNameMap": {"all": {"dataPointsByNameMap": {
            "issueName": col(json!(["TSMC", "USD CASH"])),
            "holdingPercent": col(json!([99.0, 1.0])),
            "countryOfRisk": col(json!(["Taiwan", "United States"])),
            "sectorName": col(json!(["Information Technology", "Cash and/or Derivatives"])),
        }}}}}});
        let rendered = Breakdown::from_holdings(parse(&doc).unwrap()).render();
        assert_eq!(rendered["weight"], 0.99);
        assert_eq!(rendered["countries"]["Taiwan"]["weight"], 0.99);
    }
}
