use std::{collections::HashMap, sync::LazyLock};

/// Ghostfolio's country names, which are those of `countries-list`.
const COUNTRIES: &[(&str, &str)] = &[
    ("AF", "Afghanistan"),
    ("AX", "Aland"),
    ("AL", "Albania"),
    ("DZ", "Algeria"),
    ("AS", "American Samoa"),
    ("AD", "Andorra"),
    ("AO", "Angola"),
    ("AI", "Anguilla"),
    ("AQ", "Antarctica"),
    ("AG", "Antigua and Barbuda"),
    ("AR", "Argentina"),
    ("AM", "Armenia"),
    ("AW", "Aruba"),
    ("AC", "Ascension Island"),
    ("AU", "Australia"),
    ("AT", "Austria"),
    ("AZ", "Azerbaijan"),
    ("BS", "Bahamas"),
    ("BH", "Bahrain"),
    ("BD", "Bangladesh"),
    ("BB", "Barbados"),
    ("BY", "Belarus"),
    ("BE", "Belgium"),
    ("BZ", "Belize"),
    ("BJ", "Benin"),
    ("BM", "Bermuda"),
    ("BT", "Bhutan"),
    ("BO", "Bolivia"),
    ("BQ", "Bonaire"),
    ("BA", "Bosnia and Herzegovina"),
    ("BW", "Botswana"),
    ("BV", "Bouvet Island"),
    ("BR", "Brazil"),
    ("IO", "British Indian Ocean Territory"),
    ("VG", "British Virgin Islands"),
    ("BN", "Brunei"),
    ("BG", "Bulgaria"),
    ("BF", "Burkina Faso"),
    ("BI", "Burundi"),
    ("CV", "Cabo Verde"),
    ("KH", "Cambodia"),
    ("CM", "Cameroon"),
    ("CA", "Canada"),
    ("KY", "Cayman Islands"),
    ("CF", "Central African Republic"),
    ("TD", "Chad"),
    ("CL", "Chile"),
    ("CN", "China"),
    ("CX", "Christmas Island"),
    ("CC", "Cocos (Keeling) Islands"),
    ("CO", "Colombia"),
    ("KM", "Comoros"),
    ("CK", "Cook Islands"),
    ("CR", "Costa Rica"),
    ("HR", "Croatia"),
    ("CU", "Cuba"),
    ("CW", "Curacao"),
    ("CY", "Cyprus"),
    ("CZ", "Czechia"),
    ("CD", "Democratic Republic of the Congo"),
    ("DK", "Denmark"),
    ("DJ", "Djibouti"),
    ("DM", "Dominica"),
    ("DO", "Dominican Republic"),
    ("TL", "East Timor"),
    ("EC", "Ecuador"),
    ("EG", "Egypt"),
    ("SV", "El Salvador"),
    ("GQ", "Equatorial Guinea"),
    ("ER", "Eritrea"),
    ("EE", "Estonia"),
    ("SZ", "Eswatini"),
    ("ET", "Ethiopia"),
    ("FK", "Falkland Islands"),
    ("FO", "Faroe Islands"),
    ("FJ", "Fiji"),
    ("FI", "Finland"),
    ("FR", "France"),
    ("GF", "French Guiana"),
    ("PF", "French Polynesia"),
    ("TF", "French Southern Territories"),
    ("GA", "Gabon"),
    ("GM", "Gambia"),
    ("GE", "Georgia"),
    ("DE", "Germany"),
    ("GH", "Ghana"),
    ("GI", "Gibraltar"),
    ("GR", "Greece"),
    ("GL", "Greenland"),
    ("GD", "Grenada"),
    ("GP", "Guadeloupe"),
    ("GU", "Guam"),
    ("GT", "Guatemala"),
    ("GG", "Guernsey"),
    ("GN", "Guinea"),
    ("GW", "Guinea-Bissau"),
    ("GY", "Guyana"),
    ("HT", "Haiti"),
    ("HM", "Heard Island and McDonald Islands"),
    ("HN", "Honduras"),
    ("HK", "Hong Kong"),
    ("HU", "Hungary"),
    ("IS", "Iceland"),
    ("IN", "India"),
    ("ID", "Indonesia"),
    ("IR", "Iran"),
    ("IQ", "Iraq"),
    ("IE", "Ireland"),
    ("IM", "Isle of Man"),
    ("IL", "Israel"),
    ("IT", "Italy"),
    ("CI", "Ivory Coast"),
    ("JM", "Jamaica"),
    ("JP", "Japan"),
    ("JE", "Jersey"),
    ("JO", "Jordan"),
    ("KZ", "Kazakhstan"),
    ("KE", "Kenya"),
    ("KI", "Kiribati"),
    ("XK", "Kosovo"),
    ("KW", "Kuwait"),
    ("KG", "Kyrgyzstan"),
    ("LA", "Laos"),
    ("LV", "Latvia"),
    ("LB", "Lebanon"),
    ("LS", "Lesotho"),
    ("LR", "Liberia"),
    ("LY", "Libya"),
    ("LI", "Liechtenstein"),
    ("LT", "Lithuania"),
    ("LU", "Luxembourg"),
    ("MO", "Macao"),
    ("MG", "Madagascar"),
    ("MW", "Malawi"),
    ("MY", "Malaysia"),
    ("MV", "Maldives"),
    ("ML", "Mali"),
    ("MT", "Malta"),
    ("MH", "Marshall Islands"),
    ("MQ", "Martinique"),
    ("MR", "Mauritania"),
    ("MU", "Mauritius"),
    ("YT", "Mayotte"),
    ("MX", "Mexico"),
    ("FM", "Micronesia"),
    ("MD", "Moldova"),
    ("MC", "Monaco"),
    ("MN", "Mongolia"),
    ("ME", "Montenegro"),
    ("MS", "Montserrat"),
    ("MA", "Morocco"),
    ("MZ", "Mozambique"),
    ("MM", "Myanmar"),
    ("NA", "Namibia"),
    ("NR", "Nauru"),
    ("NP", "Nepal"),
    ("NL", "Netherlands"),
    ("NC", "New Caledonia"),
    ("NZ", "New Zealand"),
    ("NI", "Nicaragua"),
    ("NE", "Niger"),
    ("NG", "Nigeria"),
    ("NU", "Niue"),
    ("NF", "Norfolk Island"),
    ("MP", "Northern Mariana Islands"),
    ("KP", "North Korea"),
    ("MK", "North Macedonia"),
    ("NO", "Norway"),
    ("OM", "Oman"),
    ("PK", "Pakistan"),
    ("PW", "Palau"),
    ("PS", "Palestine"),
    ("PA", "Panama"),
    ("PG", "Papua New Guinea"),
    ("PY", "Paraguay"),
    ("PE", "Peru"),
    ("PH", "Philippines"),
    ("PN", "Pitcairn Islands"),
    ("PL", "Poland"),
    ("PT", "Portugal"),
    ("PR", "Puerto Rico"),
    ("QA", "Qatar"),
    ("CG", "Republic of the Congo"),
    ("RE", "Reunion"),
    ("RO", "Romania"),
    ("RU", "Russia"),
    ("RW", "Rwanda"),
    ("BL", "Saint Barthelemy"),
    ("SH", "Saint Helena"),
    ("KN", "Saint Kitts and Nevis"),
    ("LC", "Saint Lucia"),
    ("MF", "Saint Martin"),
    ("PM", "Saint Pierre and Miquelon"),
    ("VC", "Saint Vincent and the Grenadines"),
    ("WS", "Samoa"),
    ("SM", "San Marino"),
    ("ST", "Sao Tome and Principe"),
    ("SA", "Saudi Arabia"),
    ("SN", "Senegal"),
    ("RS", "Serbia"),
    ("SC", "Seychelles"),
    ("SL", "Sierra Leone"),
    ("SG", "Singapore"),
    ("SX", "Sint Maarten"),
    ("SK", "Slovakia"),
    ("SI", "Slovenia"),
    ("SB", "Solomon Islands"),
    ("SO", "Somalia"),
    ("ZA", "South Africa"),
    ("GS", "South Georgia and the South Sandwich Islands"),
    ("KR", "South Korea"),
    ("SS", "South Sudan"),
    ("ES", "Spain"),
    ("LK", "Sri Lanka"),
    ("SD", "Sudan"),
    ("SR", "Suriname"),
    ("SJ", "Svalbard and Jan Mayen"),
    ("SE", "Sweden"),
    ("CH", "Switzerland"),
    ("SY", "Syria"),
    ("TW", "Taiwan"),
    ("TJ", "Tajikistan"),
    ("TZ", "Tanzania"),
    ("TH", "Thailand"),
    ("TG", "Togo"),
    ("TK", "Tokelau"),
    ("TO", "Tonga"),
    ("TT", "Trinidad and Tobago"),
    ("TA", "Tristan da Cunha"),
    ("TN", "Tunisia"),
    ("TR", "Türkiye"),
    ("TM", "Turkmenistan"),
    ("TC", "Turks and Caicos Islands"),
    ("TV", "Tuvalu"),
    ("UG", "Uganda"),
    ("UA", "Ukraine"),
    ("AE", "United Arab Emirates"),
    ("GB", "United Kingdom"),
    ("US", "United States"),
    ("UY", "Uruguay"),
    ("UM", "U.S. Minor Outlying Islands"),
    ("VI", "U.S. Virgin Islands"),
    ("UZ", "Uzbekistan"),
    ("VU", "Vanuatu"),
    ("VA", "Vatican City"),
    ("VE", "Venezuela"),
    ("VN", "Vietnam"),
    ("WF", "Wallis and Futuna"),
    ("EH", "Western Sahara"),
    ("YE", "Yemen"),
    ("ZM", "Zambia"),
    ("ZW", "Zimbabwe"),
];

const COUNTRY_ALIASES: &[(&str, &str)] = &[
    ("America", "US"),
    ("Bolivia, Plurinational State of", "BO"),
    ("Brunei Darussalam", "BN"),
    ("Burma", "MM"),
    ("Cape Verde", "CV"),
    ("China, Macau", "MO"),
    ("Chinese Taipei", "TW"),
    ("Congo", "CG"),
    ("Congo, the Democratic Republic of the", "CD"),
    ("Cote d'Ivoire", "CI"),
    ("Czech Republic", "CZ"),
    ("Great Britain", "GB"),
    ("Holland", "NL"),
    ("Holy See", "VA"),
    ("Hong Kong SAR", "HK"),
    ("Hong Kong, China", "HK"),
    ("Iran, Islamic Republic of", "IR"),
    ("Korea", "KR"),
    ("Korea (South)", "KR"),
    ("Korea South", "KR"),
    ("Korea, Democratic People's Republic of", "KP"),
    ("Korea, Republic of", "KR"),
    ("Kyrgyz Republic", "KG"),
    ("Lao People's Democratic Republic", "LA"),
    ("Macau", "MO"),
    ("Macao SAR", "MO"),
    ("Macedonia", "MK"),
    ("Micronesia, Federated States of", "FM"),
    ("Moldova, Republic of", "MD"),
    ("Netherlands (the)", "NL"),
    ("Palestine, State of", "PS"),
    ("Republic of Korea", "KR"),
    ("Russian Federation", "RU"),
    ("Saint Martin (French part)", "MF"),
    ("Sint Maarten (Dutch part)", "SX"),
    ("Slovak Republic", "SK"),
    ("Swaziland", "SZ"),
    ("Syrian Arab Republic", "SY"),
    ("Taiwan, Province of China", "TW"),
    ("Tanzania, United Republic of", "TZ"),
    ("Timor-Leste", "TL"),
    ("Turkey", "TR"),
    ("Turkiye", "TR"),
    ("UAE", "AE"),
    ("UK", "GB"),
    ("United States of America", "US"),
    ("USA", "US"),
    ("Venezuela, Bolivarian Republic of", "VE"),
    ("Viet Nam", "VN"),
    ("Virgin Islands (British)", "VG"),
    ("Virgin Islands (U.S.)", "VI"),
    ("Virgin Islands, British", "VG"),
];

/// Ghostfolio's sectors with the GICS and ICB sectors, industry groups and industries that belong to them.
const SECTORS: &[(&str, &[&str])] = &[
    (
        "Basic Materials",
        &[
            "Materials",
            "Non-Energy Materials",
            "Basic Resources",
            "Chemicals",
            "Construction Materials",
            "Containers & Packaging",
            "Metals & Mining",
            "Paper & Forest Products",
        ],
    ),
    (
        "Communication Services",
        &[
            "Communication",
            "Communications",
            "Telecommunications",
            "Telecommunication Services",
            "Diversified Telecommunication Services",
            "Wireless Telecommunication Services",
            "Media",
            "Media & Entertainment",
            "Entertainment",
            "Interactive Media & Services",
        ],
    ),
    (
        "Consumer Cyclical",
        &[
            "Consumer Cyclicals",
            "Consumer Discretionary",
            "Consumer Services",
            "Automobiles & Components",
            "Automobiles & Parts",
            "Automobile Components",
            "Auto Components",
            "Automobiles",
            "Consumer Durables & Apparel",
            "Household Durables",
            "Leisure Products",
            "Textiles, Apparel & Luxury Goods",
            "Hotels, Restaurants & Leisure",
            "Diversified Consumer Services",
            "Consumer Discretionary Distribution & Retail",
            "Consumer Products & Services",
            "Retailing",
            "Retail",
            "Distributors",
            "Broadline Retail",
            "Specialty Retail",
            "Internet & Direct Marketing Retail",
            "Multiline Retail",
            "Travel & Leisure",
        ],
    ),
    (
        "Consumer Defensive",
        &[
            "Consumer Non-Cyclicals",
            "Consumer Staples",
            "Consumer Staples Distribution & Retail",
            "Food & Staples Retailing",
            "Food, Beverage & Tobacco",
            "Food Beverage & Tobacco",
            "Beverages",
            "Food Products",
            "Tobacco",
            "Household & Personal Products",
            "Household Products",
            "Personal Care Products",
            "Personal Products",
            "Personal Care, Drug & Grocery Stores",
        ],
    ),
    (
        "Energy",
        &["Energy Equipment & Services", "Oil, Gas & Consumable Fuels"],
    ),
    (
        "Financial Services",
        &[
            "Finance",
            "Financials",
            "Banks",
            "Diversified Financials",
            "Consumer Finance",
            "Capital Markets",
            "Insurance",
            "Thrifts & Mortgage Finance",
            "Mortgage Real Estate Investment Trusts (REITs)",
        ],
    ),
    (
        "Healthcare",
        &[
            "Health Care",
            "Health Care Equipment & Services",
            "Health Care Equipment & Supplies",
            "Health Care Providers & Services",
            "Health Care Technology",
            "Pharmaceuticals, Biotechnology & Life Sciences",
            "Pharmaceuticals",
            "Biotechnology",
            "Life Sciences Tools & Services",
        ],
    ),
    (
        "Industrials",
        &[
            "Business Services",
            "Capital Goods",
            "Commercial & Professional Services",
            "Transportation",
            "Industrial Goods & Services",
            "Construction & Materials",
            "Aerospace & Defense",
            "Building Products",
            "Construction & Engineering",
            "Electrical Equipment",
            "Industrial Conglomerates",
            "Machinery",
            "Trading Companies & Distributors",
            "Commercial Services & Supplies",
            "Professional Services",
            "Air Freight & Logistics",
            "Airlines",
            "Passenger Airlines",
            "Marine",
            "Marine Transportation",
            "Road & Rail",
            "Ground Transportation",
            "Transportation Infrastructure",
        ],
    ),
    ("Other", &["Others"]),
    (
        "Real Estate",
        &[
            "Equity Real Estate Investment Trusts (REITs)",
            "Real Estate Management & Development",
            "Diversified REITs",
            "Industrial REITs",
            "Hotel & Resort REITs",
            "Office REITs",
            "Health Care REITs",
            "Residential REITs",
            "Retail REITs",
            "Specialized REITs",
        ],
    ),
    (
        "Technology",
        &[
            "Information Technology",
            "Software & Services",
            "Technology Hardware & Equipment",
            "Semiconductors & Semiconductor Equipment",
            "IT Services",
            "Software",
            "Communications Equipment",
            "Technology Hardware, Storage & Peripherals",
            "Electronic Equipment, Instruments & Components",
        ],
    ),
    (
        "Utilities",
        &[
            "Electric Utilities",
            "Gas Utilities",
            "Multi-Utilities",
            "Water Utilities",
            "Independent Power and Renewable Electricity Producers",
            "Independent Power & Renewable Electricity Producers",
        ],
    ),
];

/// Word prefixes for classifications finer than [`SECTORS`] lists, such as GICS sub-industries.
/// The first sector with a matching word wins, so the order resolves names like "Gas Utilities".
const SECTOR_KEYWORDS: &[(&str, &[&str])] = &[
    ("Utilities", &["utilit", "water", "power", "renewable"]),
    ("Real Estate", &["reit", "real"]),
    (
        "Financial Services",
        &[
            "bank", "financ", "insur", "invest", "asset", "capital", "exchange", "credit",
            "mortgage", "reinsur",
        ],
    ),
    (
        "Healthcare",
        &["health", "pharma", "biotech", "medical", "life"],
    ),
    (
        "Technology",
        &[
            "software",
            "semiconductor",
            "technolog",
            "computer",
            "electronic",
            "internet",
            "data",
        ],
    ),
    (
        "Communication Services",
        &[
            "telecom",
            "media",
            "entertain",
            "broadcast",
            "publish",
            "advertis",
            "interactive",
            "cable",
            "wireless",
            "movies",
        ],
    ),
    (
        "Energy",
        &["oil", "gas", "coal", "energy", "drilling", "refin"],
    ),
    (
        "Basic Materials",
        &[
            "chemical",
            "metal",
            "mining",
            "steel",
            "gold",
            "silver",
            "copper",
            "aluminum",
            "paper",
            "forest",
            "packag",
            "fertilizer",
            "material",
        ],
    ),
    (
        "Consumer Defensive",
        &[
            "food",
            "beverage",
            "brewer",
            "distiller",
            "tobacco",
            "household",
            "personal",
            "drug",
            "grocery",
            "agricultur",
        ],
    ),
    (
        "Consumer Cyclical",
        &[
            "retail",
            "apparel",
            "automo",
            "auto",
            "hotel",
            "restaurant",
            "leisure",
            "homebuild",
            "casino",
            "gaming",
            "footwear",
            "luxury",
            "textile",
            "education",
            "consumer",
        ],
    ),
    (
        "Industrials",
        &[
            "aerospace",
            "electrical",
            "defense",
            "machinery",
            "industrial",
            "construction",
            "engineering",
            "transport",
            "airline",
            "railroad",
            "trucking",
            "logistic",
            "freight",
            "marine",
            "building",
            "commercial",
            "professional",
            "security",
            "environmental",
        ],
    ),
];

/// Markers issuers use for cash, derivatives and unclassified positions.
const UNCLASSIFIED: &[&str] = &[
    "",
    "-",
    "n/a",
    "none",
    "null",
    "unassigned",
    "unknown",
    "cash",
    "cash and or derivatives",
    "cash and derivatives",
    "derivatives",
    "european union",
    "supranational",
];

fn key(name: &str) -> String {
    let folded: String = name
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'å' | 'ã' => 'a',
            'ç' => 'c',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            '&' => ' ',
            c => c.to_ascii_lowercase(),
        })
        .collect();
    let words: Vec<&str> = folded
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty() && *w != "and" && *w != "the")
        .collect();
    words.join(" ")
}

fn is_unclassified(k: &str) -> bool {
    UNCLASSIFIED.iter().any(|u| key(u) == k)
}

static COUNTRY_INDEX: LazyLock<HashMap<String, &'static str>> = LazyLock::new(|| {
    let by_code: HashMap<&str, &'static str> = COUNTRIES.iter().copied().collect();
    let mut index = HashMap::new();
    for (code, name) in COUNTRIES {
        index.insert(key(name), *name);
        index.insert(key(code), *name);
    }
    for (alias, code) in COUNTRY_ALIASES {
        index.insert(key(alias), by_code[code]);
    }
    index
});

static SECTOR_INDEX: LazyLock<HashMap<String, &'static str>> = LazyLock::new(|| {
    let mut index = HashMap::new();
    for (sector, names) in SECTORS {
        index.insert(key(sector), *sector);
        for name in *names {
            index.insert(key(name), *sector);
        }
    }
    index
});

/// Ghostfolio's name for a country given by name or ISO 3166 alpha-2 code.
pub fn country(name: &str) -> Option<&'static str> {
    let k = key(name);
    let found = COUNTRY_INDEX.get(&k).copied();
    if found.is_none() && !is_unclassified(&k) && !k.starts_with("other") {
        tracing::warn!(country = name, "unknown country");
    }
    found
}

/// Ghostfolio's name for a sector, `Other` for unknown sectors and `None` for cash and similar markers.
pub fn sector(name: &str) -> Option<&'static str> {
    let k = key(name);
    if is_unclassified(&k) {
        return None;
    }
    let found = SECTOR_INDEX.get(&k).copied().or_else(|| {
        SECTOR_KEYWORDS
            .iter()
            .find(|(_, words)| words.iter().any(|w| k.split(' ').any(|t| t.starts_with(w))))
            .map(|(sector, _)| *sector)
    });
    Some(found.unwrap_or_else(|| {
        tracing::warn!(sector = name, "unknown sector");
        "Other"
    }))
}

/// Whether a holding's sector marks it as cash, a derivative or otherwise not a security.
pub fn is_non_security(sector: &str) -> bool {
    let k = key(sector);
    !k.is_empty() && is_unclassified(&k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_issuer_countries() {
        for (input, expected) in [
            ("Korea (South)", "South Korea"),
            ("Korea, Republic of", "South Korea"),
            ("Korea", "South Korea"),
            ("Czech Republic", "Czechia"),
            ("Russian Federation", "Russia"),
            ("Turkey", "Türkiye"),
            ("UAE", "United Arab Emirates"),
            ("United States of America", "United States"),
            ("Macao", "Macao"),
            ("Curaçao", "Curacao"),
            ("US", "United States"),
            ("Taiwan", "Taiwan"),
        ] {
            assert_eq!(country(input), Some(expected), "{input}");
        }
        for input in ["European Union", "None", "null", "-", "Other"] {
            assert_eq!(country(input), None, "{input}");
        }
    }

    #[test]
    fn maps_issuer_sectors() {
        for (input, expected) in [
            ("Information Technology", "Technology"),
            ("Communication", "Communication Services"),
            ("Telecommunications", "Communication Services"),
            ("Consumer Discretionary", "Consumer Cyclical"),
            ("Consumer Staples", "Consumer Defensive"),
            ("Financials", "Financial Services"),
            ("Health Care", "Healthcare"),
            ("Materials", "Basic Materials"),
            ("Basic Materials", "Basic Materials"),
            ("Real Estate", "Real Estate"),
            ("Pharmaceuticals", "Healthcare"),
            ("Retail Reits", "Real Estate"),
            ("Oil, Gas and Consumable Fuels", "Energy"),
            ("Something New", "Other"),
            ("Managed Health Care", "Healthcare"),
            ("Gas Utilities", "Utilities"),
            ("Diversified Banks", "Financial Services"),
            ("Application Software", "Technology"),
            ("Integrated Oil & Gas", "Energy"),
            ("Apparel Retail", "Consumer Cyclical"),
            ("Electrical Components & Equipment", "Industrials"),
            ("Life & Health Insurance", "Financial Services"),
            ("Others", "Other"),
        ] {
            assert_eq!(sector(input), Some(expected), "{input}");
        }
        for input in ["Cash and/or Derivatives", "Unassigned", "unknown", "None"] {
            assert_eq!(sector(input), None, "{input}");
        }
    }

    #[test]
    fn every_ghostfolio_sector_is_reachable() {
        for sector_name in SECTORS.iter().map(|(s, _)| *s) {
            assert_eq!(sector(sector_name), Some(sector_name));
        }
    }
}
