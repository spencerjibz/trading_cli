use std::any;

use crate::trading::{Instrument, InstrumentType};

use super::{MessageExtendable, Returnable};
use anyhow::{ensure, Ok};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
#[derive(Serialize)]
pub struct OkexInitMessage {
    op: String,
    args: Vec<OkexInitMessageArg>,
}
impl Default for OkexInitMessage {
    fn default() -> Self {
        Self {
            op: String::from("subscribe"),
            args: vec![OkexInitMessageArg::default()],
        }
    }
}

impl MessageExtendable for OkexInitMessage {
    fn add_asset(&mut self, asset: &str) {
        if let Some(arg) = self.args.get_mut(0) {
            arg.add_asset(asset)
        }
    }

    fn to_json(&self) -> anyhow::Result<String> {
        let json = serde_json::to_string_pretty(&self)?;
        Ok(json)
    }
}

impl OkexInitMessage {
    pub fn new<T: AsRef<str>>(inst_id: T) -> Self {
        let stri = inst_id.as_ref();
        Self {
            op: String::from("subscribe"),
            args: vec![OkexInitMessageArg::new(stri)],
        }
    }
}

impl Default for OkexInitMessageArg {
    fn default() -> Self {
        Self {
            channel: "books".to_owned(),
            inst_id: "BTC-USD-240427-56000-C".to_owned(),
        }
    }
}

impl MessageExtendable for OkexInitMessageArg {
    fn add_asset(&mut self, asset: &str) {
        self.inst_id = asset.to_owned();
    }
    fn to_json(&self) -> anyhow::Result<String> {
        let json = serde_json::to_string_pretty(&self)?;
        Ok(json)
    }
}
impl OkexInitMessageArg {
    pub fn new(inst_id: &str) -> Self {
        Self {
            channel: "books".to_owned(),
            inst_id: inst_id.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OkexInitMessageArg {
    pub channel: String,
    pub inst_id: String,
}

#[derive(Deserialize, Debug)]
pub struct OkexResponse {
    pub data: Option<Vec<OkexResponseData>>,
    pub arg: Option<OkexInitMessageArg>,
}

#[derive(Deserialize, Debug)]
pub struct OkexResponseData {
    pub bids: Vec<Vec<String>>,
    pub asks: Vec<Vec<String>>,
}

impl Returnable for OkexResponse {
    fn asks_bids_pair(&self) -> Option<super::AskBidPairs> {
        if let Some(ref data) = &self.data {
            if let Some(OkexResponseData { bids, asks }) = data.first() {
                let bid_floats: Vec<_> = bids
                    .iter()
                    .map(|v| {
                        let first: f32 = v[0].parse().unwrap();
                        let second: f32 = v[1].parse().unwrap();
                        (first, second)
                    })
                    .collect();

                let ask_floats: Vec<_> = asks
                    .iter()
                    .map(|v| {
                        let first: f32 = v[0].parse().unwrap();
                        let second: f32 = v[1].parse().unwrap();
                        (first, second)
                    })
                    .collect();

                return Some((ask_floats, bid_floats));
            }
        }
        None
    }

    fn instrument_name(&self) -> Option<Instrument> {
        if let Some(ref data) = &self.arg {
            let asset_name = data.inst_id.clone();
            let instrument =
                Instrument::from_exchange_string(&asset_name, super::ExchangeType::Okex).unwrap();
            return Some(instrument);
        }
        None
    }
}

pub fn string_to_instrument_okex(asset: &str) -> Result<crate::trading::Instrument, anyhow::Error> {
    use chrono::{DateTime, Utc};
    let parts: Vec<&str> = asset.split('-').collect(); // expected format {asset}-{date}-{strike price} - {C/P}

    ensure!(parts.len() == 5, anyhow::anyhow!("Invalid asset format"));

    let asset_name = parts[0].to_string();
    let date_str = parts[2];
    let price_str = parts[3];
    let instrument_type_str = parts[4];

    let mut datetime = NaiveDate::parse_from_str(date_str, "%y%m%d")?;
    //let date_utc = chrono::DateTime::from_naive_utc_and_offset(datetime.and_hms_opt(0, 0, 0).unwrap(), Utc);

    let price_int: i64 = price_str.parse()?;

    let instrument_type = InstrumentType::from_given_str(instrument_type_str);
    if let Some(value) = instrument_type {
        let instrument = Instrument {
            asset: asset_name,
            strike_price: price_int,
            expiration_date: datetime,
            instrument_type: value,
        };

        Ok(instrument)
    } else {
        Err(anyhow::anyhow!(
            "unsupported instrument type {instrument_type_str}"
        ))
    }
}

#[test]
fn parsing_between_string_and_instrument_okex_works() -> anyhow::Result<()> {
    let asset = "BTC-USD-240427-56000-C";
    let date = NaiveDate::from_ymd_opt(2024, 4, 27).unwrap();
    let inst = string_to_instrument_okex(asset)?;
    let expected = Instrument {
        asset: "BTC-USD".to_owned(),
        strike_price: 56000,
        expiration_date: date,
        instrument_type: InstrumentType::Call,
    };

    assert_eq!(inst, expected);

    let instr_to_str = inst.to_exchange_asset_str(crate::exchanges::ExchangeType::Okex);
    assert_eq!(instr_to_str, asset);
    Ok(())
}
