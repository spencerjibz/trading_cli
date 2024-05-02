mod okex;
pub use okex::*;
mod deribit;
use crate::{exchanges, trading::Instrument};
pub use deribit::*;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ptr::read, sync::Arc};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::http::response;
pub enum ExchangeType {
    Delibris,
    Okex,
}
#[derive(Serialize)]
/// exchange settings to use for connnections
pub enum Exchanges<'a> {
    // (name,url, initial_message)
    Deribit(&'a str, &'a str, DeribitInitMessage),
    Okex(&'a str, &'a str, OkexInitMessage),
}

impl<'a> Exchanges<'a> {
    pub fn init_message(&mut self) -> &mut dyn MessageExtendable {
        match self {
            Exchanges::Deribit(_, _, response) => response as &mut dyn MessageExtendable,
            Exchanges::Okex(_, _, response) => response as &mut dyn MessageExtendable,
        }
    }
    pub fn get_url(&self) -> &'a str {
        match &self {
            Exchanges::Deribit(_, url, _) => url,
            Exchanges::Okex(_, url, _) => url,
        }
    }

    pub fn get_name(&self) -> &'a str {
        match &self {
            Exchanges::Deribit(name, _, _) => name,
            Exchanges::Okex(name, _, _) => name,
        }
    }
}

/// ask-bid pairs in the form of a tuple of (asks, bids)
type AskBidPairs = (Vec<(f32, f32)>, Vec<(f32, f32)>);

/// make return values easier to use, we only care about the bids, asks and instrument name fields
/// This gives us one unified interface for each exchange's response to use
pub trait Returnable {
    fn asks_bids_pair(&self) -> Option<AskBidPairs>;
    fn instrument_name(&self) -> Option<Instrument>;
}

pub trait MessageExtendable {
    fn add_asset(&mut self, asset: &str);
    fn to_json(&self) -> anyhow::Result<String>;
}
/// For global usage;
lazy_static! {
    pub static ref EXCHANGES: Mutex<HashMap<&'static str, Exchanges<'static>>> = {
        let mut map = HashMap::new();
        let deribit_message = DeribitInitMessage::default();
        let deribit_name = "deribit";
        let okex_name = "okex";
        let okex_message = OkexInitMessage::default();
        let okex_url = "wss://ws.okx.com:8443/ws/v5/public";
        let deribit_url = "wss://www.deribit.com/ws/api/v2";
        map.insert(
            deribit_name,
            Exchanges::Deribit(deribit_name, deribit_url, deribit_message),
        );
        map.insert(
            okex_name,
            Exchanges::Okex(okex_name, okex_url, okex_message),
        );

        Mutex::new(map)
    };
}



