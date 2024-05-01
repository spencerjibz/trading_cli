
use super::{MessageExtendable, Returnable};
use crate::trading::{Instrument, InstrumentType};
use anyhow::{ensure, Ok};
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Debug)]
/// expected Response from subscribing to Deribit Exchange
pub struct DeribitResponse {
    pub params: Option<ResponseParams>,
}

#[derive(Deserialize, Debug)]
pub struct ResponseParams {
    pub data: DeribitResponseData,
}

#[derive(Deserialize, Debug)]
pub struct DeribitResponseData {
    pub bids: Vec<(f32, f32)>,
    pub asks: Vec<(f32, f32)>,
    pub instrument_name: String,
}

#[derive(Serialize)]
pub struct DeribitInitMessage {
    method: String,
    params: DeribitInitMessageParams,
}

impl Default for DeribitInitMessage {
    fn default() -> Self {
        Self {
            method: String::from("public/subscribe"),
            params: DeribitInitMessageParams::default(),
        }
    }
}

impl MessageExtendable for DeribitInitMessage {
    fn add_asset(&mut self, asset: &str) {
        self.params.add_asset(asset)
    }

    fn to_json(&self) -> anyhow::Result<String> {
        let json = serde_json::to_string_pretty(&self)?;
        Ok(json)
    }
}
#[derive(Serialize)]
struct DeribitInitMessageParams {
    channels: Vec<String>,
    jsonrpc: String,
    id: i32,
}

impl MessageExtendable for DeribitInitMessageParams {
    fn add_asset(&mut self, asset: &str) {
        if let Some(value) = self.channels.get_mut(0) {
            *value = format!("book.{}.none.20.100ms", asset);
        }
    }

    fn to_json(&self) -> anyhow::Result<String> {
        let json = serde_json::to_string_pretty(&self)?;
        Ok(json)
    }
}

impl Default for DeribitInitMessageParams {
    fn default() -> Self {
        Self {
            channels: vec!["book.BTC-10MAY24-66000-C.none.20.100ms".to_owned()],
            jsonrpc: String::from("2.0"),
            id: 0,
        }
    }
}

impl DeribitInitMessageParams {
    fn new(asset: &String) -> Self {
        let channel = format!("book.{asset}.none.20.100ms");
        Self {
            channels: vec![channel],
            ..Default::default()
        }
    }
}

/// Implements the `Returnable` trait for the `DeribitResponse` struct.
///
/// The `asks_bids_pair` method returns an `Option` containing the asks and bids
/// from the `DeribitResponseData` struct, if present.
///
/// The `instrument_name` method returns an `Option` containing the instrument
/// name from the `DeribitResponseData` struct, if present.
impl Returnable for DeribitResponse {
    fn asks_bids_pair(&self) -> Option<super::AskBidPairs> {
        if let Some(ResponseParams {
            data:
                DeribitResponseData {
                    bids,
                    asks,
                    instrument_name,
                },
        }) = &self.params
        {
            return Some((asks.to_owned(), bids.to_owned()));
        }
        None
    }

    fn instrument_name(&self) -> Option<Instrument> {
        if let Some(ResponseParams {
            data:
                DeribitResponseData {
                    bids,
                    asks,
                    instrument_name,
                },
        }) = &self.params
        {
            let name = instrument_name;
            let instrument =
                Instrument::from_exchange_string(name, super::ExchangeType::Delibris).unwrap();
            return Some(instrument);
        }
        None
    }
}

pub fn string_to_instrument_deribit(asset: &str) -> anyhow::Result<crate::trading::Instrument> {
    use chrono::{NaiveDate, Utc};
    let parts: Vec<&str> = asset.split('-').collect(); // expected format {asset}-{date}-{strike price} - {C/P}
    ensure!(parts.len() == 4, anyhow::anyhow!("Invalid asset format"));
    let asset_name = parts[0];
    let date_str = parts[1];
    let price_str = parts[2];
    let instrument_type_str = parts[3];
    let mut datetime = NaiveDate::parse_from_str(date_str, "%d%h%y")?;
    let price_int: i64 = price_str.parse()?;
    let instrument_type = InstrumentType::from_given_str(instrument_type_str);
    if let Some(value) = instrument_type {
        let instrument = Instrument {
            asset: asset_name.to_owned(),
            strike_price: price_int,
            expiration_date: datetime,
            instrument_type: value,
        };
        return Ok(instrument);
    }
    Err(anyhow::anyhow!(
        "unsupported instrument type {instrument_type_str}"
    ))
}

#[test]
fn parsing_between_string_and_instrument_deribit_works() -> anyhow::Result<()> {
    use chrono::NaiveDate;
    let asset = "BTC-27APR24-56000-C";
    let date = NaiveDate::parse_from_str("27APR24", "%d%h%y")?;
    let expected = Instrument {
        asset: "BTC".to_owned(),
        strike_price: 56000,
        expiration_date: date,
        instrument_type: InstrumentType::Call,
    };
    let inst = string_to_instrument_deribit(asset)?;
    assert_eq!(inst, expected);

    let instr_to_str = inst.to_exchange_asset_str(crate::exchanges::ExchangeType::Delibris);
    assert_eq!(instr_to_str, asset);

    Ok(())
}
