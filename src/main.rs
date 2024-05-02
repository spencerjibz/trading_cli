use lib::{
    exchanges::{DeribitResponse, OkexResponse},
    trading::{Instrument, OrderBook},
    utils::{create_connection, fetch_bids_and_asks},
};
use std::sync::Arc;
use tokio::{select, sync::Mutex};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    console_subscriber::init();
    let deribit_order_book = Arc::new(Mutex::new(OrderBook::new("deribit")));
    let okex_order_book = Arc::new(Mutex::new(OrderBook::new("okex"))); // default instrument here is same as okex_one ()
    let deribit_reader = create_connection("deribit", None).await?;
    let okex_reader = create_connection("okex", Some("BTC-USD-240510-66000-C")).await?;
    loop {
        let (deribit_reader_clone, okex_reader_clone) =
            (deribit_reader.clone(), okex_reader.clone());
        let (deribit_order_book_clone, okex_order_book_clone) =
            (deribit_order_book.clone(), okex_order_book.clone());
        let task = fetch_bids_and_asks::<OkexResponse>(okex_reader_clone, okex_order_book_clone);

        let task_o =
            fetch_bids_and_asks::<DeribitResponse>(deribit_reader_clone, deribit_order_book_clone); // on we add

        // fetch data simultaneously also order match accross whenever a fetch in completed
        select! {
             _ = task_o => {
                 let mut deribit_ob = deribit_order_book.lock().await;
                 let mut instrument = Instrument::from_exchange_string("BTC-10MAY24-66000-C",lib::exchanges::ExchangeType::Delibris)?;
                  instrument.asset.push_str("-USD");

                 deribit_ob.match_orders(&instrument,okex_order_book.clone()).await

             }

              _ =  task => {

                let mut okex_ob = okex_order_book.lock().await;
                   let instrument = Instrument::from_exchange_string("BTC-USD-240510-66000-C",lib::exchanges::ExchangeType::Okex)?;
                  okex_ob.match_orders(&instrument.to_singular_asset(),deribit_order_book.clone()).await
             }

        }
    }
}
