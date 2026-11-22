use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

use crate::normalize;

const TOP_HOLDINGS: usize = 10;

#[derive(Debug, Clone)]
pub struct Holding {
    pub name: String,
    pub weight: f64,
    pub country: Option<String>,
    pub sector: Option<String>,
}

/// A fund's allocation with issuer names and weights in any scale.
#[derive(Debug, Default)]
pub struct Breakdown {
    pub countries: Vec<(String, f64)>,
    pub sectors: Vec<(String, f64)>,
    pub top: Vec<(String, f64)>,
}

impl Breakdown {
    pub fn from_holdings(holdings: Vec<Holding>) -> Self {
        let holdings: Vec<Holding> = holdings
            .into_iter()
            .filter(|h| h.weight.is_finite() && h.weight > 0.0)
            .filter(|h| !h.sector.as_deref().is_some_and(normalize::is_non_security))
            .collect();
        let mut countries: BTreeMap<String, f64> = BTreeMap::new();
        let mut sectors: BTreeMap<String, f64> = BTreeMap::new();
        for h in &holdings {
            if let Some(c) = &h.country {
                *countries.entry(c.clone()).or_default() += h.weight;
            }
            if let Some(s) = &h.sector {
                *sectors.entry(s.clone()).or_default() += h.weight;
            }
        }
        let mut top: Vec<(String, f64)> =
            holdings.into_iter().map(|h| (h.name, h.weight)).collect();
        top.sort_by(|a, b| b.1.total_cmp(&a.1));
        top.truncate(TOP_HOLDINGS);
        Self {
            countries: countries.into_iter().collect(),
            sectors: sectors.into_iter().collect(),
            top,
        }
    }

    /// TrackInsight's holdings document with Ghostfolio's country and sector names and fractional weights.
    pub fn render(&self) -> Value {
        let scale = self.scale();
        let countries = merge(&self.countries, scale, normalize::country);
        let sectors = merge(&self.sectors, scale, normalize::sector);
        let coverage = [&countries, &sectors]
            .into_iter()
            .filter(|m| !m.is_empty())
            .map(|m| m.values().sum::<f64>())
            .reduce(f64::min)
            .unwrap_or(0.0);
        let top: Vec<Value> = self
            .top
            .iter()
            .map(|(label, w)| json!({ "label": label, "weight": round(w / scale) }))
            .collect();
        json!({
            "weight": round(coverage),
            "countries": weights(countries),
            "sectors": weights(sectors),
            "topHoldings": top,
        })
    }

    /// Divisor that turns the weights into fractions.
    fn scale(&self) -> f64 {
        let total = |v: &[(String, f64)]| v.iter().map(|(_, w)| w).sum::<f64>();
        if total(&self.countries).max(total(&self.sectors)) > 1.5 {
            100.0
        } else {
            1.0
        }
    }
}

fn merge(
    entries: &[(String, f64)],
    scale: f64,
    canonical: fn(&str) -> Option<&'static str>,
) -> BTreeMap<&'static str, f64> {
    let mut merged = BTreeMap::new();
    for (name, w) in entries {
        if let Some(c) = canonical(name) {
            *merged.entry(c).or_default() += w / scale;
        }
    }
    merged
}

fn weights(map: BTreeMap<&'static str, f64>) -> Value {
    Value::Object(
        map.into_iter()
            .map(|(k, w)| (k.to_owned(), json!({ "weight": round(w) })))
            .collect::<Map<_, _>>(),
    )
}

fn round(w: f64) -> f64 {
    ((w * 1e6).round() / 1e6).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holding(name: &str, weight: f64, country: &str, sector: &str) -> Holding {
        Holding {
            name: name.into(),
            weight,
            country: Some(country.into()),
            sector: Some(sector.into()),
        }
    }

    #[test]
    fn aggregates_holdings_in_percent() {
        let doc = Breakdown::from_holdings(vec![
            holding("TSMC", 60.0, "Taiwan", "Information Technology"),
            holding("Samsung", 38.0, "Korea (South)", "Information Technology"),
            holding("USD CASH", 2.0, "United States", "Cash and/or Derivatives"),
        ])
        .render();
        assert_eq!(doc["weight"], 0.98);
        assert_eq!(doc["countries"]["Taiwan"]["weight"], 0.6);
        assert_eq!(doc["countries"]["South Korea"]["weight"], 0.38);
        assert!(doc["countries"].get("United States").is_none());
        assert_eq!(doc["sectors"]["Technology"]["weight"], 0.98);
        assert_eq!(doc["topHoldings"][0]["label"], "TSMC");
        assert_eq!(doc["topHoldings"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn coverage_is_the_smaller_classified_share() {
        let doc = Breakdown {
            countries: vec![("Germany".into(), 0.5), ("European Union".into(), 0.5)],
            sectors: vec![("Financials".into(), 1.0)],
            top: vec![],
        }
        .render();
        assert_eq!(doc["weight"], 0.5);
    }
}
