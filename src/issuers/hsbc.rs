use anyhow::Result;
use reqwest::StatusCode;

use super::Issuers;
use crate::{breakdown::Breakdown, sheet};

const DOCUMENTS: &str = "https://www.assetmanagement.hsbc.co.uk/api/v1/download/document";

/// HSBC's holdings export lists no sectors.
pub async fn fetch(issuers: &Issuers, isin: &str) -> Result<Option<Breakdown>> {
    let res = issuers
        .http
        .get(format!(
            "{DOCUMENTS}/{}/gb/en/holdings",
            isin.to_lowercase()
        ))
        .send()
        .await?;
    if res.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let mut holdings = sheet::holdings_from_workbook(&res.error_for_status()?.bytes().await?)?;
    issuers.fill_sectors(&mut holdings).await?;
    Ok(Some(Breakdown::from_holdings(holdings)))
}
