use anyhow::Result;
use serde_json::Value;

use super::Issuers;
use crate::breakdown::Breakdown;

const SHARE_CLASSES: &str = "https://dng-api.invesco.com/cache/v1/accounts/en_GB/shareclasses";

pub async fn fetch(issuers: &Issuers, isin: &str) -> Result<Option<Breakdown>> {
    let base = format!("{SHARE_CLASSES}/{isin}");
    let fund_holdings = get(issuers, &format!("{base}/holdings/fund?idType=isin")).await?;
    if pairs(&fund_holdings["holdings"], "name", "weight").is_empty() {
        return Ok(None);
    }
    let details = get(
        issuers,
        &format!("{base}?expand=nav&idType=isin&variationType=fundDetails"),
    )
    .await?;
    let replication = details["replicationMethod"].as_str().unwrap_or("Physical");
    let breakdown = |kind: &'static str, by: &'static str| {
        let url = format!(
            "{base}/weightedHoldings/{kind}?idType=isin&breakdown={by}&audienceType=Financial%20Professional&productCode=ETF&replicationMethod={replication}&assetType=Equity"
        );
        async move {
            Ok::<_, anyhow::Error>(pairs(
                &get(issuers, &url).await?["holdingWeights"],
                "name",
                "value",
            ))
        }
    };
    let synthetic = replication.eq_ignore_ascii_case("synthetic");
    let (countries, sectors, top) = if synthetic {
        let index_holdings = get(issuers, &format!("{base}/holdings/index?idType=isin")).await?;
        (
            breakdown("index", "country").await?,
            breakdown("index", "sector").await?,
            pairs(&index_holdings["holdings"], "name", "weight"),
        )
    } else {
        (
            breakdown("fund", "country").await?,
            breakdown("fund", "sector").await?,
            pairs(&fund_holdings["holdings"], "name", "weight"),
        )
    };
    let mut top = top;
    top.sort_by(|a, b| b.1.total_cmp(&a.1));
    top.truncate(10);
    Ok(Some(Breakdown {
        countries,
        sectors,
        top,
    }))
}

async fn get(issuers: &Issuers, url: &str) -> Result<Value> {
    Ok(issuers
        .http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

fn pairs(list: &Value, name: &str, weight: &str) -> Vec<(String, f64)> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| Some((e[name].as_str()?.to_owned(), e[weight].as_f64()?)))
        .collect()
}
