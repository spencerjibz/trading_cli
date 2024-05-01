#[cfg(test)]
mod order_book {
    use crate::{
        exchanges::ExchangeType,
        trading::{Instrument, Order, OrderBook, TradeRequest},
    };

    #[test]
    fn match_trades() -> anyhow::Result<()> {
        let mut order_book = OrderBook::new("test");
        let mut orders = vec![
            Order::new(0.72, 30, TradeRequest::Ask),
            Order::new(0.73, 20, TradeRequest::Ask),
            Order::new(0.90, 50, TradeRequest::Bid),
        ];
        let asset = "BTC-USD-240427-56000-C";
        let instrument = Instrument::from_exchange_string(asset, ExchangeType::Okex)?;
        order_book.add_asset(instrument.clone());
        for order in orders {
            order_book.add_order(order, &instrument);
        }
        order_book.match_orders(&instrument, None);
        // both asks are completed with  higher bid of 90,

        let cols = order_book.asset_order_table.get_mut(&instrument).unwrap();
        assert_eq!(cols.history.len(), 2);
        assert!(cols
            .orders
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.is_partial_completed()));

        // the remaining gty of ask is 10
        Ok(())
    }
}
