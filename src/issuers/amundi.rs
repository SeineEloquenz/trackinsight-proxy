use anyhow::Result;
use serde_json::{json, Value};

use super::Issuers;
use crate::breakdown::Breakdown;

const PRODUCTS: &str = "https://www.amundietf.de/mapi/ProductAPI/getProductsData";

pub async fn fetch(issuers: &Issuers, isin: &str) -> Result<Option<Breakdown>> {
    let body = json!({
        "context": { "countryCode": "DEU", "languageCode": "en", "userProfileName": "RETAIL" },
        "productIds": [isin],
        "productType": "PRODUCT",
        "characteristics": ["FUND_REPLICATION_METHODOLOGY"],
        "breakDown": { "aggregationFields": [
            "FUND_COUNTRIES", "FUND_SECTORS", "FUND_TOP10",
            "INDEX_COUNTRIES", "INDEX_SECTORS", "INDEX_TOP10",
        ] },
    });
    let doc: Value = issuers
        .http
        .post(PRODUCTS)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(doc["products"]
        .as_array()
        .and_then(|products| products.iter().find(|p| p["productId"] == isin))
        .map(parse))
}

/// Uses the index allocation for swap-based funds.
fn parse(product: &Value) -> Breakdown {
    let replication = product["characteristics"]["FUND_REPLICATION_METHODOLOGY"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();
    let synthetic = ["indirect", "swap", "synthetic"]
        .iter()
        .any(|s| replication.contains(s));
    let order = if synthetic {
        ["INDEX", "FUND"]
    } else {
        ["FUND", "INDEX"]
    };
    let field = |kind: &str| -> Vec<(String, f64)> {
        order
            .iter()
            .map(|source| aggregation(product, &format!("{source}_{kind}")))
            .find(|entries| !entries.is_empty())
            .unwrap_or_default()
    };
    Breakdown {
        countries: field("COUNTRIES"),
        sectors: field("SECTORS"),
        top: field("TOP10"),
    }
}

fn aggregation(product: &Value, field: &str) -> Vec<(String, f64)> {
    product["breakDowns"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| b["aggregationField"] == field)
        .flat_map(|b| b["breakDownData"].as_array().into_iter().flatten())
        .filter_map(|d| {
            let weight = d["adjustedWeight"]
                .as_f64()
                .or_else(|| d["weight"].as_f64())?;
            Some((d["aggregationName"].as_str()?.to_owned(), weight))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product(replication: &str) -> Value {
        let data =
            |name: &str| json!([{ "aggregationName": name, "weight": 1.0, "adjustedWeight": 1.0 }]);
        json!({
            "characteristics": { "FUND_REPLICATION_METHODOLOGY": replication },
            "breakDowns": [
                { "aggregationField": "FUND_COUNTRIES", "breakDownData": data("Germany") },
                { "aggregationField": "INDEX_COUNTRIES", "breakDownData": data("Taiwan") },
            ],
        })
    }

    #[test]
    fn uses_the_index_for_swap_funds() {
        assert_eq!(
            parse(&product("Indirect(Swap Based)")).countries[0].0,
            "Taiwan"
        );
        assert_eq!(
            parse(&product("Direct(Physical)")).countries[0].0,
            "Germany"
        );
    }
}
