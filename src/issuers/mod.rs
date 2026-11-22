use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use reqwest::Client;

use crate::{breakdown::Breakdown, cache::Cached};

mod amundi;
mod hsbc;
mod invesco;
mod ishares;
mod jpmorgan;
mod spdr;
mod vanguard;
mod xtrackers;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36";

/// Fund issuers whose public sites publish holdings, found by ISIN.
pub struct Issuers {
    http: Client,
    ishares: Cached<HashMap<String, u64>>,
    vanguard: Cached<HashMap<String, String>>,
    spdr: Cached<HashMap<String, String>>,
    jpmorgan: Cached<HashMap<String, &'static str>>,
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
        })
    }

    /// The breakdown of the fund with this ISIN and its issuer's name, or `None` if no issuer offers it.
    pub async fn breakdown(&self, isin: &str) -> Result<Option<(&'static str, Breakdown)>> {
        let (ishares, amundi, vanguard, spdr, xtrackers, invesco, jpmorgan, hsbc) = tokio::join!(
            ishares::fetch(self, isin),
            amundi::fetch(self, isin),
            vanguard::fetch(self, isin),
            spdr::fetch(self, isin),
            xtrackers::fetch(self, isin),
            invesco::fetch(self, isin),
            jpmorgan::fetch(self, isin),
            hsbc::fetch(self, isin),
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
}
