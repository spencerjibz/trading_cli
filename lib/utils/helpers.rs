use ordered_float::OrderedFloat;
use serde::de::DeserializeOwned;

use crate::{
    exchanges::{Returnable, EXCHANGES},
    trading::{
        CurrentHoldingPerPrice, Order, OrderBook, OrderStatus, PriceColumns, PriceRow, TradeRequest,
    },
};

use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

pub type ReaderStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
/// a utility function for adding new orders to the OrderBook Columns
pub fn add_each(table: &mut PriceColumns, order: &mut Order) {
    let Order {
        id,
        is_arbitrage: _,
        status: _,
        price,
        request,
        quantity,
        filled_with: _,
        remaining_qty: _,
    } = order;
    let key = *price;

    use TradeRequest::*;
    let existing_holding = if *request == Ask {
        table.asks.get_mut(&key.into())
    } else {
        table.bids.get_mut(&key.into())
    };
    let total_amount = (*quantity as f32) * *price;
    let stored_order = MininalOrder::new(*id, *quantity, *price);
    match (request, existing_holding) {
        (&mut Ask, Some(holding)) => {
            update_existing_holding(holding, *quantity, total_amount, stored_order);
        }
        (&mut Ask, None) => {
            let new_holding = create_new_holding(*quantity, total_amount, stored_order);
            let key = *price;
            table.asks.insert(key.into(), new_holding);
        }

        (&mut Bid, Some(holding)) => {
            update_existing_holding(holding, *quantity, total_amount, stored_order);
        }

        _ => {
            let new_holding = create_new_holding(*quantity, total_amount, stored_order);
            let key = *price;
            table.bids.insert(key.into(), new_holding);
        }
    }
}

pub fn round(x: f32, decimals: u32) -> f32 {
    let y = 10i32.pow(decimals) as f32;
    (x * y).round() / y
}

/// if an order with the same price already exists, just update exisiting properities
fn update_existing_holding(
    holding: &mut CurrentHoldingPerPrice,
    quantity: i32,
    total_amount: f32,
    stored_order: MininalOrder,
) {
    holding.total_quantity += quantity;

    holding.total_amount += total_amount;
    let amount = holding.total_amount;
    holding.total_amount = round(amount, 3);
    holding.orders.push(stored_order)
}

fn create_new_holding(
    quantity: i32,
    total_amount: f32,
    current_order: MininalOrder,
) -> CurrentHoldingPerPrice {
    CurrentHoldingPerPrice {
        total_quantity: quantity,
        total_amount,
        orders: vec![current_order],
    }
}

use crate::trading::MininalOrder;
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
/// Generates a  unique timestamp  to use an id
pub fn get_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

use std::collections::HashMap;
/// updates the quantity required to complete a trade at price level
pub fn match_at_price_level(
    current_holding: &mut CurrentHoldingPerPrice,
    incoming_order_qty: &mut i32,
) -> (i32, VecDeque<u128>) {
    let mut done_qty = 0;

    let mut orders_to_remove = VecDeque::new();

    current_holding.orders.iter_mut().for_each(|order| {
        if order.qty <= *incoming_order_qty {
            *incoming_order_qty -= order.qty;
            done_qty += order.qty;
            order.qty = 0;
            orders_to_remove.push_back(order.id);
        } else {
            order.qty -= *incoming_order_qty;
            done_qty += *incoming_order_qty;

            *incoming_order_qty = 0
        }
    });

    current_holding.orders.retain(|x| x.qty > 0);
    current_holding.update_qty_and_amount();

    (done_qty, orders_to_remove)
}

pub async fn fetch_bids_and_asks<T: Returnable + DeserializeOwned + std::fmt::Debug>(
    reader: Arc<Mutex<ReaderStream>>,
    order_book: Arc<Mutex<OrderBook<'_>>>,
) -> anyhow::Result<()> {
    if let Some(Ok(Message::Text(message))) = reader.lock().await.next().await {
        let json: T = serde_json::from_str(&message)?;

        if let (Some((asks, bids)), Some(instrument_name)) =
            (json.asks_bids_pair(), json.instrument_name())
        {
            let instra = instrument_name.clone();
            order_book.lock().await.add_asset(instra);
            let ask_pairs = asks.iter();
            let bids_pairs = bids.iter();
            let mut bids_count = 0;
            let mut ask_count = 0;
            for (asking_price, quantity) in ask_pairs {
                let ask_order = Order::new(*asking_price, *quantity as i32, TradeRequest::Ask);

                order_book
                    .lock()
                    .await
                    .add_order(ask_order, &instrument_name);
                ask_count += 1;
            }

            for (biding_price, bid_quantity) in bids_pairs {
                let bid_order = Order::new(*biding_price, *bid_quantity as i32, TradeRequest::Bid);
                order_book
                    .lock()
                    .await
                    .add_order(bid_order, &instrument_name);
                bids_count += 1;
            }

            println!(
                "+{ask_count} asks and + {bids_count} from {}",
                order_book.lock().await.exchange
            );
        }
    }

    Ok(())
}

pub async fn create_connection(
    exchange_name: &str,
    asset: Option<&str>,
) -> anyhow::Result<Arc<Mutex<ReaderStream>>> {
    let mut map = EXCHANGES.lock().await;

    if let Some(exchange) = map.get_mut(exchange_name) {
        let mut init_message = exchange.init_message();
        if let Some(asset_name) = asset {
            init_message.add_asset(asset_name)
        }

        let init_message_json = init_message.to_json()?;
        let url = exchange.get_url();
        let name = exchange.get_name();
        println!("connection to {name} exchange");
        let (stream, _) = connect_async(url).await?;
        let (mut writer, mut reader) = stream.split();

        let msg = Message::Text(init_message_json);

        writer.send(msg).await?;
        let clonable_reader = Arc::new(Mutex::new(reader));

        return Ok(clonable_reader);
    }

    Err(anyhow::anyhow!("unknown exchange name"))
}
