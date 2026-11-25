use std::{collections::HashMap, time::Duration};

use anyhow::{anyhow, Result};
use reqwest::Client;

use crate::{
    breakdown::{Breakdown, Holding},
    cache::Cached,
};

mod amundi;
mod hsbc;
mod invesco;
mod ishares;
mod jpmorgan;
mod spdr;
mod ubs;
mod vanguard;
mod xtrackers;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36";

/// iShares MSCI World IMI and Core MSCI EM IMI, which together hold the MSCI ACWI IMI universe.
const REFERENCE_FUNDS: &[&str] = &["IE000AOK2013", "IE00BKM4GZ66"];

/// Words in index and fund names that do not change which securities the index holds.
const NOT_INDEX: &[&str] = &[
    "ishares", "core", "ubs", "ucits", "etf", "acc", "dist", "index", "total", "return", "net",
    "gross", "div", "reinv", "nr", "tr", "usd", "eur", "gbp", "chf", "jpy",
];

/// Fund issuers whose public sites publish holdings, found by ISIN.
pub struct Issuers {
    http: Client,
    ishares: Cached<HashMap<String, (u64, String)>>,
    vanguard: Cached<HashMap<String, String>>,
    spdr: Cached<HashMap<String, String>>,
    jpmorgan: Cached<HashMap<String, &'static str>>,
    reference: Cached<HashMap<String, Classification>>,
}

#[derive(Clone)]
struct Classification {
    country: Option<String>,
    sector: Option<String>,
}

impl Issuers {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .user_agent(USER_AGENT)
                .timeout(Duration::from_secs(60))
                .build()?,
            ishares: Cached::default(),
            vanguard: Cached::default(),
            spdr: Cached::default(),
            jpmorgan: Cached::default(),
            reference: Cached::default(),
        })
    }

    /// The breakdown of the fund with this ISIN and its issuer's name, or `None` if no issuer offers it.
    pub async fn breakdown(&self, isin: &str) -> Result<Option<(&'static str, Breakdown)>> {
        let (ishares, amundi, vanguard, spdr, xtrackers, invesco, jpmorgan, hsbc, ubs) = tokio::join!(
            ishares::fetch(self, isin),
            amundi::fetch(self, isin),
            vanguard::fetch(self, isin),
            spdr::fetch(self, isin),
            xtrackers::fetch(self, isin),
            invesco::fetch(self, isin),
            jpmorgan::fetch(self, isin),
            hsbc::fetch(self, isin),
            ubs::fetch(self, isin),
        );
        let mut failure = None;
        for (issuer, result) in [
            ("iShares", ishares),
            ("Amundi", amundi),
            ("Vanguard", vanguard),
            ("SPDR", spdr),
            ("Xtrackers", xtrackers),
            ("Invesco", invesco),
            ("JPMorgan", jpmorgan),
            ("HSBC", hsbc),
            ("UBS", ubs),
        ] {
            match result {
                Ok(Some(b)) => return Ok(Some((issuer, b))),
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(isin, issuer, "lookup failed: {e:#}");
                    failure = Some(e);
                }
            }
        }
        failure.map_or(Ok(None), Err)
    }

    /// The breakdown of an iShares fund on the index with this name.
    async fn index_breakdown(&self, index: &str) -> Result<Option<Breakdown>> {
        for isin in ishares::tracking(self, &index_key(index)).await? {
            if let Ok(Some(holdings)) = ishares::holdings(self, &isin).await {
                return Ok(Some(Breakdown::from_holdings(holdings)));
            }
        }
        Ok(None)
    }

    /// Gives holdings the country and sector their ISIN has in the reference funds when none of them have one.
    async fn fill_from_reference(&self, holdings: &mut [Holding]) -> Result<()> {
        let countries = holdings.iter().all(|h| h.country.is_none());
        let sectors = holdings.iter().all(|h| h.sector.is_none());
        if !countries && !sectors {
            return Ok(());
        }
        let reference = self.reference.get(|| self.load_reference()).await?;
        for h in holdings {
            let Some(r) = h.isin.as_ref().and_then(|isin| reference.get(isin)) else {
                continue;
            };
            if countries {
                h.country = r.country.clone();
            }
            if sectors {
                h.sector = r.sector.clone();
            }
        }
        Ok(())
    }

    async fn load_reference(&self) -> Result<HashMap<String, Classification>> {
        let mut reference = HashMap::new();
        for fund in REFERENCE_FUNDS {
            let holdings = ishares::holdings(self, fund)
                .await?
                .ok_or_else(|| anyhow!("iShares no longer lists reference fund {fund}"))?;
            for h in holdings {
                if let Some(isin) = h.isin {
                    reference.entry(isin).or_insert(Classification {
                        country: h.country,
                        sector: h.sector,
                    });
                }
            }
        }
        tracing::info!(
            securities = reference.len(),
            "loaded reference classifications"
        );
        Ok(reference)
    }
}

/// The words of an index or fund name that identify the index, so that both name it the same way.
fn index_key(name: &str) -> String {
    let lower = name.to_lowercase();
    let mut words: Vec<&str> = lower
        .split(|c: char| !(c.is_alphanumeric() || c == '&'))
        .filter(|w| !w.is_empty())
        .collect();
    if let Some(hedged) = words.iter().position(|w| *w == "hedged") {
        let start = words[..hedged]
            .iter()
            .rposition(|w| *w == "with")
            .unwrap_or(hedged);
        let end = if words.get(hedged + 1) == Some(&"to") {
            hedged + 3
        } else {
            hedged + 1
        };
        words.drain(start..end.min(words.len()));
    }
    words
        .into_iter()
        .filter(|w| !NOT_INDEX.contains(w))
        .map(|w| if w == "us" { "usa" } else { w })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::index_key;

    #[test]
    fn index_and_fund_names_share_a_key() {
        let world_ex_usa = index_key("iShares MSCI World ex-USA UCITS ETF");
        assert_eq!(world_ex_usa, "msci world ex usa");
        assert_eq!(
            index_key("MSCI World (ex US) (net div. reinv.)"),
            world_ex_usa
        );
        assert_eq!(
            index_key("MSCI ACWI with Developed Markets 100% hedged to JPY Index"),
            index_key("iShares MSCI ACWI UCITS ETF")
        );
        assert_eq!(
            index_key("MSCI World Total Return Net"),
            index_key("iShares Core MSCI World UCITS ETF USD (Acc)")
        );
        assert_eq!(
            index_key("iShares S&P 500 CHF Hedged UCITS ETF (Acc)"),
            index_key("S&P 500")
        );
        assert_ne!(
            index_key("MSCI World ESG Screened"),
            index_key("MSCI World")
        );
    }
}
