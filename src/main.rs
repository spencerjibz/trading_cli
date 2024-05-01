use futures_util::future::join;
use lib::{
    exchanges::{DeribitResponse, OkexResponse},
    utils::run_order_book,
};

#[tokio::main]
async fn main() {
    console_subscriber::init();
    let task1 = run_order_book::<OkexResponse>("okex", Some("BTC-USD-240429-56000-C"));
    let task = tokio::spawn(run_order_book::<DeribitResponse>("deribit", None));
    let tasks = tokio::spawn(task1);
}
