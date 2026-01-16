use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Quote asset for trading pairs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum QuoteAsset {
    Land,
    Btc,
    Bch,
    Doge,
}

impl QuoteAsset {
    pub fn as_str(&self) -> &'static str {
        match self {
            QuoteAsset::Land => "LAND",
            QuoteAsset::Btc => "BTC",
            QuoteAsset::Bch => "BCH",
            QuoteAsset::Doge => "DOGE",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "LAND" => Some(QuoteAsset::Land),
            "BTC" => Some(QuoteAsset::Btc),
            "BCH" => Some(QuoteAsset::Bch),
            "DOGE" => Some(QuoteAsset::Doge),
            _ => None,
        }
    }
}

/// Trading pair (e.g., "LAND/BTC", "LAND/DOGE")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradingPair {
    pub base: String,      // e.g., "LAND"
    pub quote: QuoteAsset, // e.g., QuoteAsset::Btc
}

impl TradingPair {
    pub fn new(base: impl Into<String>, quote: QuoteAsset) -> Self {
        Self {
            base: base.into(),
            quote,
        }
    }

    pub fn from_symbol(symbol: &str) -> Option<Self> {
        let parts: Vec<&str> = symbol.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        let base = parts[0].to_string();
        let quote = QuoteAsset::from_str(parts[1])?;
        Some(Self { base, quote })
    }

    pub fn symbol(&self) -> String {
        format!("{}/{}", self.base, self.quote.as_str())
    }
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Limit,
    Market,
}

/// Time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum TimeInForce {
    GTC, // Good Till Cancel
    IOC, // Immediate or Cancel
    FOK, // Fill or Kill
    GTT, // Good Till Time (not implemented yet)
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

/// Order structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub owner: String,
    pub pair: TradingPair,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<u64>, // Price in smallest units (satoshis), None for market orders
    pub size: u64,          // Size in smallest units
    pub filled: u64,        // Amount filled so far
    pub status: OrderStatus,
    pub tif: TimeInForce,
    pub post_only: bool, // If true, order will be cancelled if it would immediately match
    pub timestamp: u64,  // Creation time (ms)
}

/// Trade execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub pair: TradingPair,
    pub price: u64,
    pub size: u64,
    pub buyer: String,
    pub seller: String,
    pub buyer_order_id: String,
    pub seller_order_id: String,
    pub timestamp: u64,
    pub taker_side: Side, // Which side was the taker (market order or aggressor)
}

/// Order book level (price and total size at that price)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookLevel {
    pub price: u64,
    pub size: u64,
}

/// Order book for a trading pair
#[derive(Debug, Clone)]
pub struct OrderBook {
    pub pair: TradingPair,
    pub quote: QuoteAsset, // Quick reference to quote asset
    // Price -> List of order IDs at that price
    pub bids: BTreeMap<u64, Vec<String>>, // Buy orders (descending price)
    pub asks: BTreeMap<u64, Vec<String>>, // Sell orders (ascending price)
    // Order ID -> Order
    pub orders: BTreeMap<String, Order>,
}

impl OrderBook {
    pub fn new(pair: TradingPair) -> Self {
        let quote = pair.quote;
        Self {
            pair,
            quote,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: BTreeMap::new(),
        }
    }

    /// Get aggregated bids (highest to lowest)
    pub fn get_bids(&self, depth: usize) -> Vec<BookLevel> {
        self.bids
            .iter()
            .rev() // Reverse to get highest prices first
            .take(depth)
            .map(|(price, order_ids)| {
                let total_size: u64 = order_ids
                    .iter()
                    .filter_map(|id| self.orders.get(id))
                    .map(|o| o.size - o.filled)
                    .sum();
                BookLevel {
                    price: *price,
                    size: total_size,
                }
            })
            .collect()
    }

    /// Get aggregated asks (lowest to highest)
    pub fn get_asks(&self, depth: usize) -> Vec<BookLevel> {
        self.asks
            .iter()
            .take(depth)
            .map(|(price, order_ids)| {
                let total_size: u64 = order_ids
                    .iter()
                    .filter_map(|id| self.orders.get(id))
                    .map(|o| o.size - o.filled)
                    .sum();
                BookLevel {
                    price: *price,
                    size: total_size,
                }
            })
            .collect()
    }

    /// Get best bid (highest buy price)
    pub fn best_bid(&self) -> Option<u64> {
        self.bids.keys().next_back().copied()
    }

    /// Get best ask (lowest sell price)
    pub fn best_ask(&self) -> Option<u64> {
        self.asks.keys().next().copied()
    }

    /// Add an order to the book
    pub fn add_order(&mut self, order: Order) {
        let order_id = order.id.clone();
        let price = order.price.expect("Limit orders must have price");

        match order.side {
            Side::Buy => {
                self.bids.entry(price).or_default().push(order_id.clone());
            }
            Side::Sell => {
                self.asks.entry(price).or_default().push(order_id.clone());
            }
        }

        self.orders.insert(order_id, order);
    }

    /// Remove an order from the book
    pub fn remove_order(&mut self, order_id: &str) -> Option<Order> {
        if let Some(order) = self.orders.remove(order_id) {
            if let Some(price) = order.price {
                let book = match order.side {
                    Side::Buy => &mut self.bids,
                    Side::Sell => &mut self.asks,
                };

                if let Some(orders) = book.get_mut(&price) {
                    orders.retain(|id| id != order_id);
                    if orders.is_empty() {
                        book.remove(&price);
                    }
                }
            }
            Some(order)
        } else {
            None
        }
    }

    /// Update order filled amount
    pub fn update_order_filled(&mut self, order_id: &str, filled: u64) {
        if let Some(order) = self.orders.get_mut(order_id) {
            order.filled = filled;
            if order.filled >= order.size {
                order.status = OrderStatus::Filled;
            } else if order.filled > 0 {
                order.status = OrderStatus::PartiallyFilled;
            }
        }
    }
}

/// Matching engine
pub struct MatchingEngine {
    books: Arc<Mutex<BTreeMap<String, OrderBook>>>,
    trades: Arc<Mutex<Vec<Trade>>>,
    next_trade_id: Arc<Mutex<u64>>,
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self {
            books: Arc::new(Mutex::new(BTreeMap::new())),
            trades: Arc::new(Mutex::new(Vec::new())),
            next_trade_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Get or create order book for a pair
    fn get_or_create_book(&self, pair: &TradingPair) -> OrderBook {
        let mut books = self.books.lock().unwrap();
        books
            .entry(pair.symbol())
            .or_insert_with(|| OrderBook::new(pair.clone()))
            .clone()
    }

    /// Update order book
    fn update_book(&self, book: OrderBook) {
        let mut books = self.books.lock().unwrap();
        books.insert(book.pair.symbol(), book);
    }

    /// Generate next trade ID
    fn next_trade_id(&self) -> String {
        let mut next_id = self.next_trade_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        format!("trade-{}", id)
    }

    /// Place a limit order
    pub fn place_limit_order(&self, mut order: Order) -> Result<Vec<Trade>, String> {
        if order.order_type != OrderType::Limit {
            return Err("Not a limit order".into());
        }
        if order.price.is_none() {
            return Err("Limit order must have price".into());
        }

        let mut book = self.get_or_create_book(&order.pair);
        let mut trades = Vec::new();

        // Check post_only flag
        if order.post_only {
            let would_match = match order.side {
                Side::Buy => {
                    if let Some(best_ask) = book.best_ask() {
                        order.price.unwrap() >= best_ask
                    } else {
                        false
                    }
                }
                Side::Sell => {
                    if let Some(best_bid) = book.best_bid() {
                        order.price.unwrap() <= best_bid
                    } else {
                        false
                    }
                }
            };

            if would_match {
                order.status = OrderStatus::Rejected;
                return Err("Post-only order would immediately match".into());
            }
        }

        // Try to match against existing orders
        let executed_trades = self.match_order(&mut book, &mut order)?;
        trades.extend(executed_trades);

        // If order has remaining size and isn't IOC/FOK, add to book
        let remaining = order.size - order.filled;
        if remaining > 0 {
            match order.tif {
                TimeInForce::IOC => {
                    order.status = OrderStatus::Cancelled;
                }
                TimeInForce::FOK => {
                    if order.filled == 0 {
                        order.status = OrderStatus::Cancelled;
                        return Err("Fill or Kill order could not be completely filled".into());
                    }
                }
                TimeInForce::GTC | TimeInForce::GTT => {
                    book.add_order(order.clone());
                }
            }
        }

        self.update_book(book);

        // Store trades
        if !trades.is_empty() {
            let mut all_trades = self.trades.lock().unwrap();
            all_trades.extend(trades.clone());
        }

        Ok(trades)
    }

    /// Place a market order
    pub fn place_market_order(&self, mut order: Order) -> Result<Vec<Trade>, String> {
        if order.order_type != OrderType::Market {
            return Err("Not a market order".into());
        }

        let mut book = self.get_or_create_book(&order.pair);

        // Market orders must match immediately
        let trades = self.match_order(&mut book, &mut order)?;

        if trades.is_empty() {
            return Err("No liquidity available for market order".into());
        }

        // Market orders never go on the book
        if order.filled < order.size {
            order.status = OrderStatus::PartiallyFilled;
        }

        self.update_book(book);

        // Store trades
        let mut all_trades = self.trades.lock().unwrap();
        all_trades.extend(trades.clone());

        Ok(trades)
    }

    /// Match an order against the book
    fn match_order(&self, book: &mut OrderBook, order: &mut Order) -> Result<Vec<Trade>, String> {
        let mut trades = Vec::new();
        let mut remaining = order.size - order.filled;

        // Get matching side of the book
        let prices: Vec<u64> = match order.side {
            Side::Buy => {
                // Match against asks (lowest first)
                book.asks.keys().copied().collect()
            }
            Side::Sell => {
                // Match against bids (highest first)
                book.bids.keys().rev().copied().collect()
            }
        };

        for price in prices {
            if remaining == 0 {
                break;
            }

            // Check if price matches
            let can_match = match order.order_type {
                OrderType::Market => true, // Market orders match at any price
                OrderType::Limit => match order.side {
                    Side::Buy => price <= order.price.unwrap(),
                    Side::Sell => price >= order.price.unwrap(),
                },
            };

            if !can_match {
                break;
            }

            // Get orders at this price level
            let order_ids: Vec<String> = match order.side {
                Side::Buy => book.asks.get(&price).cloned().unwrap_or_default(),
                Side::Sell => book.bids.get(&price).cloned().unwrap_or_default(),
            };

            for maker_id in order_ids {
                if remaining == 0 {
                    break;
                }

                if let Some(maker) = book.orders.get(&maker_id).cloned() {
                    let maker_remaining = maker.size - maker.filled;
                    let trade_size = remaining.min(maker_remaining);

                    // Create trade
                    let trade = Trade {
                        id: self.next_trade_id(),
                        pair: order.pair.clone(),
                        price,
                        size: trade_size,
                        buyer: if order.side == Side::Buy {
                            order.owner.clone()
                        } else {
                            maker.owner.clone()
                        },
                        seller: if order.side == Side::Sell {
                            order.owner.clone()
                        } else {
                            maker.owner.clone()
                        },
                        buyer_order_id: if order.side == Side::Buy {
                            order.id.clone()
                        } else {
                            maker.id.clone()
                        },
                        seller_order_id: if order.side == Side::Sell {
                            order.id.clone()
                        } else {
                            maker.id.clone()
                        },
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                        taker_side: order.side,
                    };

                    trades.push(trade.clone());

                    // Charge fee on the trade (0.1% to taker)
                    let quote_value = (trade.size as f64 * trade.price as f64) / 1e16; // Convert to actual units
                    let fee_amount = quote_value * 0.001; // 0.1% fee

                    // Deduct fee from taker in quote currency
                    let taker_id = if trade.taker_side == Side::Buy {
                        &trade.buyer
                    } else {
                        &trade.seller
                    };
                    if let Err(e) =
                        crate::market::wallet::deduct_quote(taker_id, book.quote, fee_amount)
                    {
                        tracing::warn!("Failed to deduct fee from {}: {}", taker_id, e);
                    } else {
                        // Route fee to vault and trigger auto-buy
                        if let Err(e) =
                            crate::market::settlement::route_exchange_fee(book.quote, fee_amount)
                        {
                            tracing::warn!("Failed to route exchange fee: {}", e);
                        }
                    }

                    // Update filled amounts
                    order.filled += trade_size;
                    book.update_order_filled(&maker_id, maker.filled + trade_size);

                    // Remove maker if fully filled
                    if maker.filled + trade_size >= maker.size {
                        book.remove_order(&maker_id);
                    }

                    remaining -= trade_size;
                }
            }
        }

        // Update order status
        if order.filled >= order.size {
            order.status = OrderStatus::Filled;
        } else if order.filled > 0 {
            order.status = OrderStatus::PartiallyFilled;
        }

        Ok(trades)
    }

    /// Cancel an order
    pub fn cancel_order(
        &self,
        pair: &TradingPair,
        order_id: &str,
        owner: &str,
    ) -> Result<Order, String> {
        let mut book = self.get_or_create_book(pair);

        if let Some(mut order) = book.remove_order(order_id) {
            if order.owner != owner {
                return Err("Not authorized to cancel this order".into());
            }

            // Unlock funds that were locked for this order
            let remaining = order.size - order.filled;
            if remaining > 0 {
                if order.side == Side::Buy {
                    // For buy orders, unlock the quote currency (LAND)
                    if let Some(price) = order.price {
                        let locked_amount = (remaining as f64 * price as f64) / 1e16;
                        if let Err(e) = crate::market::wallet::unlock_quote_balance(
                            owner,
                            book.quote,
                            locked_amount,
                        ) {
                            tracing::warn!(
                                "Failed to unlock quote balance for cancelled order {}: {}",
                                order_id,
                                e
                            );
                        } else {
                            tracing::info!(
                                "🔓 Unlocked {} {} for cancelled buy order {}",
                                locked_amount,
                                book.quote.as_str(),
                                order_id
                            );
                        }
                    }
                } else {
                    // For sell orders, unlock the base currency (BTC/BCH/DOGE)
                    let base_amount = (remaining as f64) / 1e8;
                    // Base currency is determined from the pair's base (chain)
                    let base_asset = QuoteAsset::from_str(&pair.base).unwrap_or(QuoteAsset::Btc);
                    if let Err(e) =
                        crate::market::wallet::unlock_base_balance(owner, base_asset, base_amount)
                    {
                        tracing::warn!(
                            "Failed to unlock base balance for cancelled order {}: {}",
                            order_id,
                            e
                        );
                    } else {
                        tracing::info!(
                            "🔓 Unlocked {} {} for cancelled sell order {}",
                            base_amount,
                            base_asset.as_str(),
                            order_id
                        );
                    }
                }
            }

            order.status = OrderStatus::Cancelled;
            self.update_book(book);
            Ok(order)
        } else {
            Err("Order not found".into())
        }
    }

    /// Get order book
    pub fn get_book(&self, pair: &TradingPair, depth: usize) -> (Vec<BookLevel>, Vec<BookLevel>) {
        let book = self.get_or_create_book(pair);
        (book.get_bids(depth), book.get_asks(depth))
    }

    /// Get recent trades
    pub fn get_trades(&self, pair: &TradingPair, limit: usize) -> Vec<Trade> {
        let trades = self.trades.lock().unwrap();
        trades
            .iter()
            .filter(|t| &t.pair == pair)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get user's open orders
    pub fn get_user_orders(&self, pair: &TradingPair, owner: &str) -> Vec<Order> {
        let book = self.get_or_create_book(pair);
        book.orders
            .values()
            .filter(|o| {
                o.owner == owner && o.status == OrderStatus::Open
                    || o.status == OrderStatus::PartiallyFilled
            })
            .cloned()
            .collect()
    }

    /// Get ticker data
    pub fn get_ticker(&self, pair: &TradingPair) -> Option<TickerData> {
        let trades = self.trades.lock().unwrap();
        let pair_trades: Vec<&Trade> = trades.iter().filter(|t| &t.pair == pair).collect();

        if pair_trades.is_empty() {
            return None;
        }

        let last_price = pair_trades.last()?.price;
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let day_ago = now.saturating_sub(86400 * 1000);

        let trades_24h: Vec<&Trade> = pair_trades
            .iter()
            .filter(|t| t.timestamp >= day_ago)
            .copied()
            .collect();

        let volume_24h: u64 = trades_24h.iter().map(|t| t.size).sum();
        let high_24h = trades_24h
            .iter()
            .map(|t| t.price)
            .max()
            .unwrap_or(last_price);
        let low_24h = trades_24h
            .iter()
            .map(|t| t.price)
            .min()
            .unwrap_or(last_price);

        let open_price = trades_24h.first().map(|t| t.price).unwrap_or(last_price);
        let change_24h = if open_price > 0 {
            ((last_price as f64 - open_price as f64) / open_price as f64) * 100.0
        } else {
            0.0
        };

        Some(TickerData {
            last: last_price,
            change_24h,
            volume_24h,
            high_24h,
            low_24h,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerData {
    pub last: u64,
    pub change_24h: f64,
    pub volume_24h: u64,
    pub high_24h: u64,
    pub low_24h: u64,
}

impl Default for MatchingEngine {
    fn default() -> Self {
        Self::new()
    }
}
