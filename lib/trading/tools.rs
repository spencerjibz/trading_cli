use crate::exchanges::{
    self, string_to_instrument_deribit, string_to_instrument_okex, ExchangeType,
};

use crate::utils::{add_each, get_timestamp_ms, round};
use ordered_float::OrderedFloat;
use std::default;
use std::fmt::format;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    sync::{Arc, Mutex},
    thread::sleep,
};
use tokio_tungstenite::tungstenite::http::{request, Version};

#[derive(PartialEq, PartialOrd, Debug, Default, Clone)]
pub enum TradeRequest {
    #[default]
    Ask,
    Bid,
}

impl TradeRequest {
    pub fn is_ask(&self) -> bool {
        *self == TradeRequest::Ask
    }
}

#[derive(PartialEq, PartialOrd, Debug, Default, Clone)]
pub struct MininalOrder {
    pub price: f32,
    pub id: u128,
    pub qty: i32,
}

impl MininalOrder {
    pub fn new(id: u128, qty: i32, price: f32) -> Self {
        Self { id, qty, price }
    }
}
#[derive(PartialEq, PartialOrd, Debug, Default, Clone)]
/// Current quantity and total amount of assets at price, we have left
pub struct CurrentHoldingPerPrice {
    pub total_quantity: i32,       // quantity of  assets left at certain price
    pub total_amount: f32,         // total amount in price left after trading
    pub orders: Vec<MininalOrder>, //
}

impl CurrentHoldingPerPrice {
    pub fn update_qty_and_amount(&mut self) {
        self.total_quantity = self.orders.iter().fold(0, |c, n| c + n.qty);

        if let Some(first) = self.orders.first() {
            self.total_amount = self.total_quantity as f32 * first.price;
        }
    }
}
#[derive(PartialEq, PartialOrd, Debug, Default, Clone)]
pub enum OrderStatus {
    Completed,
    Partial,
    #[default]
    Pending,
}
impl OrderStatus {
    pub fn is_completed(&self) -> bool {
        *self == OrderStatus::Completed
    }

    pub fn is_partial(&self) -> bool {
        *self == OrderStatus::Partial
    }
}
#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub struct MatchedOrders {
    pub price: f32,
    pub quantity: i32,
    pub exchange: String,
}
impl MatchedOrders {
    pub fn new(price: f32, quantity: i32, exchange: &str) -> Self {
        Self {
            price,
            quantity,
            exchange: exchange.to_owned(),
        }
    }
}
#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub struct Order {
    pub id: u128, // timestamp in ms
    pub is_arbitrage: bool,
    pub status: OrderStatus,
    pub price: f32,
    pub request: TradeRequest,
    pub quantity: i32,
    pub remaining_qty: i32, // the qty required to completed order after a partial trade,
    pub filled_with: VecDeque<MatchedOrders>,
}
// custom because we want to add a custom  id (timestamp)
impl Default for Order {
    fn default() -> Self {
        Self {
            id: get_timestamp_ms(),
            is_arbitrage: false,
            price: 0.0,
            quantity: 0,
            status: OrderStatus::default(),
            request: TradeRequest::default(),
            remaining_qty: 0,
            filled_with: VecDeque::new(),
        }
    }
}

impl Order {
    pub fn new(price: f32, quantity: i32, request: TradeRequest) -> Self {
        Self {
            price,
            quantity,
            request,
            id: get_timestamp_ms(),
            ..Default::default()
        }
    }
    pub fn is_completed(&self) -> bool {
        self.status.is_completed()
    }

    pub fn is_partial_completed(&self) -> bool {
        self.status.is_partial()
    }
}
pub type PriceRow = BTreeMap<OrderedFloat<f32>, CurrentHoldingPerPrice>;

#[derive(Debug, Default, Clone)]
pub struct PriceColumns {
    pub bids: PriceRow,
    pub asks: PriceRow,
    pub spread: f32,
    pub midprice: f32, //
    pub orders: Arc<Mutex<VecDeque<Order>>>,
    pub history: VecDeque<Order>, // all completed trades
    pub exchange_name: String,
}

impl PriceColumns {
    pub fn update_spread_and_mid_price(&mut self) {
        let max_bid = self.bids.last_key_value();
        let min_ask = self.asks.first_key_value();
        if let (Some((min_price, _)), Some((max_price, _))) = (min_ask, max_bid) {
            let diff = *max_price - *min_price;
            let mid_point = (*max_price + *min_price) / 2.0;

            self.spread = round(diff.into_inner(), 5);
            self.midprice = round(mid_point.into_inner(), 5);
        }
    }

    pub fn extend(&mut self, other: &mut PriceColumns) {
        self.bids
            .extend(other.bids.iter().map(|(k, v)| (*k, v.clone())));
        self.asks
            .extend(other.asks.iter().map(|(k, v)| (*k, v.clone())));

        self.update_spread_and_mid_price();
    }
}

#[derive(PartialEq, Eq, PartialOrd, Debug, Default, Clone, Hash)]
pub enum InstrumentType {
    #[default]
    Call,
    Pull,
}

impl InstrumentType {
    pub fn from_given_str(input_str: &str) -> Option<Self> {
        match input_str {
            "C" => Some(InstrumentType::Call),
            "P" => Some(InstrumentType::Pull),
            _ => None,
        }
    }
    pub fn to_char(&self) -> char {
        match self {
            Self::Call => 'C',
            _ => 'P',
        }
    }
}

use chrono::NaiveDate;
#[derive(PartialEq, Eq, Debug, Default, Clone, Hash)]
pub struct Instrument {
    pub asset: String,
    pub strike_price: i64,
    pub expiration_date: NaiveDate,
    pub instrument_type: InstrumentType,
}

impl Instrument {
    pub fn to_singular_asset(&self) -> Instrument {
        let mut instrument = self.clone();
        if let Some(asset) = instrument.asset.split('-').nth(0) {
            instrument.asset = asset.to_owned();
        }
        instrument
    }
    pub fn from_exchange_string(
        input_str: &str,
        exchange_type: ExchangeType,
    ) -> anyhow::Result<Instrument> {
        use ExchangeType::*;
        match exchange_type {
            Delibris => string_to_instrument_deribit(input_str),

            Okex => string_to_instrument_okex(input_str),
        }
    }

    pub fn to_exchange_asset_str(&self, exchange_type: ExchangeType) -> String {
        use ExchangeType::*;
        match exchange_type {
            Delibris => format!(
                "{}-{}-{}-{}",
                self.asset,
                self.expiration_date.format("%d%h%y"),
                self.strike_price,
                self.instrument_type.to_char()
            )
            .to_uppercase(),

            Okex => format!(
                "{}-{}-{}-{}",
                self.asset,
                self.expiration_date.format("%y%m%d"),
                self.strike_price,
                self.instrument_type.to_char()
            ),
        }
    }
}
