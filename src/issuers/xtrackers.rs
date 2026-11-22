use anyhow::Result;
use reqwest::header::CONTENT_TYPE;

use super::Issuers;
use crate::{breakdown::Breakdown, sheet};

const CONSTITUENTS: &str =
    "https://etf.deutsche-am.com/etfdata/export/GBR/ENG/excel/product/constituent";

pub async fn fetch(issuers: &Issuers, isin: &str) -> Result<Option<Breakdown>> {
    let res = issuers
        .http
        .get(format!("{CONSTITUENTS}/{isin}/"))
        .send()
        .await?;
    let is_sheet = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("spreadsheet"));
    if !res.status().is_success() || !is_sheet {
        return Ok(None);
    }
    let holdings = sheet::holdings_from_xlsx(&res.bytes().await?)?;
    Ok(Some(Breakdown::from_holdings(holdings)))
}
