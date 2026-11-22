use anyhow::{anyhow, Context, Result};
use calamine::{open_workbook_auto_from_rs, Data, Reader};

use crate::breakdown::Holding;

const NAME: &[&str] = &[
    "name",
    "security name",
    "issuer name",
    "issuer",
    "holding",
    "security",
    "description",
];
const WEIGHT: &[&str] = &[
    "weighting",
    "weight",
    "percent of fund",
    "% of fund",
    "% of net assets",
    "weight (%)",
    "% weight",
];
const COUNTRY: &[&str] = &[
    "country of risk",
    "country",
    "trade country name",
    "location",
    "country name",
    "country of issue",
];
const ISIN: &[&str] = &["isin", "isin code"];
const SECTOR: &[&str] = &[
    "sector",
    "sector classification",
    "gics sector",
    "industry classification",
    "industry",
];

/// Reads the holdings of the first worksheet of an issuer's xls or xlsx export by recognising its header row.
pub fn holdings_from_workbook(bytes: &[u8]) -> Result<Vec<Holding>> {
    let mut book =
        open_workbook_auto_from_rs(std::io::Cursor::new(bytes)).context("opening workbook")?;
    let range = book
        .worksheet_range_at(0)
        .ok_or_else(|| anyhow!("workbook has no sheets"))??;
    let rows: Vec<Vec<Cell>> = range
        .rows()
        .map(|r| r.iter().map(Cell::from).collect())
        .collect();
    holdings_from_rows(&rows)
}

pub enum Cell {
    Text(String),
    Number(f64),
    Empty,
}

impl From<&Data> for Cell {
    fn from(d: &Data) -> Self {
        match d {
            Data::Float(f) => Cell::Number(*f),
            Data::Int(i) => Cell::Number(*i as f64),
            Data::String(s) => Cell::Text(s.trim().to_owned()),
            _ => Cell::Empty,
        }
    }
}

impl Cell {
    fn text(&self) -> Option<&str> {
        match self {
            Cell::Text(s) if !s.is_empty() => Some(s),
            _ => None,
        }
    }

    fn number(&self) -> Option<f64> {
        match self {
            Cell::Number(n) => Some(*n),
            Cell::Text(s) => {
                let s = s.trim_end_matches('%').trim();
                let s = if s.contains('.') {
                    s.to_owned()
                } else {
                    s.replace(',', ".")
                };
                s.parse().ok()
            }
            Cell::Empty => None,
        }
    }
}

pub fn holdings_from_rows(rows: &[Vec<Cell>]) -> Result<Vec<Holding>> {
    let (header_at, cols) = rows
        .iter()
        .enumerate()
        .find_map(|(i, row)| Columns::find(row).map(|c| (i, c)))
        .ok_or_else(|| anyhow!("no header row with a name and weight column"))?;
    let at = |row: &Vec<Cell>, col: Option<usize>| {
        col.and_then(|c| row.get(c))
            .and_then(Cell::text)
            .map(str::to_owned)
    };
    Ok(rows[header_at + 1..]
        .iter()
        .filter_map(|row| {
            Some(Holding {
                name: at(row, Some(cols.name))?,
                isin: at(row, cols.isin),
                weight: row.get(cols.weight)?.number()?,
                country: at(row, cols.country),
                sector: at(row, cols.sector),
            })
        })
        .collect())
}

struct Columns {
    name: usize,
    weight: usize,
    isin: Option<usize>,
    country: Option<usize>,
    sector: Option<usize>,
}

impl Columns {
    fn find(row: &[Cell]) -> Option<Self> {
        let headers: Vec<String> = row
            .iter()
            .map(|c| compact(c.text().unwrap_or_default()))
            .collect();
        let pick = |names: &[&str]| {
            names
                .iter()
                .find_map(|n| headers.iter().position(|h| *h == compact(n)))
        };
        Some(Self {
            name: pick(NAME)?,
            weight: pick(WEIGHT)?,
            country: pick(COUNTRY),
            isin: pick(ISIN),
            sector: pick(SECTOR),
        })
    }
}

fn compact(header: &str) -> String {
    header
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '%')
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Cell {
        Cell::Text(s.into())
    }

    #[test]
    fn finds_the_header_row_and_skips_footer() {
        let rows = vec![
            vec![t("Fund Name:"), t("Some ETF")],
            vec![Cell::Empty],
            vec![
                t("ISIN"),
                t("SecurityName"),
                t("Percent of Fund"),
                t("Trade Country Name"),
                t("Sector Classification"),
            ],
            vec![
                t("US0378331005"),
                t("Apple"),
                Cell::Number(5.5),
                t("United States"),
                t("Information Technology"),
            ],
            vec![
                t("Unassigned"),
                t("U.S. Dollar"),
                t("-"),
                t("United States"),
                t("Unassigned"),
            ],
            vec![t("Marketing communication")],
        ];
        let h = holdings_from_rows(&rows).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].name, "Apple");
        assert_eq!(h[0].isin.as_deref(), Some("US0378331005"));
        assert_eq!(h[0].weight, 5.5);
        assert_eq!(h[0].country.as_deref(), Some("United States"));
    }
}
