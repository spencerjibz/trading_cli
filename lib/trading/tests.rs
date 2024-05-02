#[cfg(test)]
mod order_book {
    use crate::{
        exchanges::ExchangeType,
        trading::{Instrument, Order, OrderBook, TradeRequest},
    };

    #[tokio::test]
    async fn match_trades_across_exchanges() -> anyhow::Result<()> {
        use std::sync::Arc;
        use tokio::sync::Mutex;
        let mut order_book = Arc::new(Mutex::new(OrderBook::new("test")));
        let mut orders = vec![
            Order::new(0.72, 30, TradeRequest::Ask),
            Order::new(0.73, 20, TradeRequest::Ask),
            Order::new(0.90, 50, TradeRequest::Bid),
        ];
        let asset = "BTC-USD-240427-56000-C";
        let instrument = Instrument::from_exchange_string(asset, ExchangeType::Okex)?;

        let mut second_order_book = Arc::new(Mutex::new(OrderBook::new("test2")));
        order_book.lock().await.add_asset(instrument.clone());
        second_order_book.lock().await.add_asset(instrument.clone());
        for order in orders {
            order_book.lock().await.add_order(order, &instrument);
        }

        order_book
            .lock()
            .await
            .match_orders(&instrument, second_order_book.clone())
            .await;
        //order_book.match_orders(&instrument, None);
        // both asks are completed with  higher bid of 90,

        let mut second_orders = [
            Order::new(0.50, 10, TradeRequest::Ask),
            Order::new(0.73, 20, TradeRequest::Bid),
            Order::new(0.90, 50, TradeRequest::Bid),
        ];

        for order in second_orders {
            second_order_book.lock().await.add_order(order, &instrument)
        }
        second_order_book
            .lock()
            .await
            .match_orders(&instrument, order_book.clone())
            .await;

        /*  dbg!(&second_order_book,&order_book);  uncomment to inspect the structures */
        let mut lock = second_order_book.lock().await;

        let cols = lock.asset_order_table.get_mut(&instrument).unwrap();

        // matches are all in here;
        assert_eq!(cols.history.len(), 2);
        assert!(cols.history.iter().all(|o| o.is_completed()));
        // there is should be atleast one partially matched order bin the first_order_book
        let mut od_1_lock = order_book.lock().await;
        let cols = od_1_lock.asset_order_table.get_mut(&instrument).unwrap();

        cols.history
            .iter()
            .any(|order| order.is_partial_completed());

        Ok(())
    }
}
