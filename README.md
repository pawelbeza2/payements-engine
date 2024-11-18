# Payements engine

## Assumptions
*Additional assumptions which weren't explicily stated in problem statement*
- transactions:
    - dispute can make available funds negative
    - resolve and chargeback can make held funds negative
    - dispute (+ resolve and chargeback) is available only for deposits
    - if we fail to process transaction we always just log and proceed to the next one
- state
    - assuming that I store all the state in memory (instead of DB)
## Design
My main goals for the solution were:  
(1) make solution is thread safe and easily pluggable to any multithreaded env  
(2) make calculations precise as we operate on financial data  
  


### (1) multithread design
What we want to have is some kind of mapping between account id and
account info including account balance and transactions (here called `AccountManager`)
```
pub struct Engine {
    accounts: Arc<DashMap<u16, AccountManager>>,
}

pub struct AccountManager {
    pub account: Account,
    pub transactions: HashMap<u32, TransactionDetails>,
}
```

I decided to keep it dead simple and use concurrent hashmap called [DashMap](https://github.com/xacrimon/dashmap) instead of reinventing
the most optimize way to do locking with `RwLock` and `Mutex`. 
  
I chose to embed transactions within `AccountManager` rather than using a global transactions map in `Engine`.
This approach ensures that operations on transactions for one account do not interfere with those for another
account (we just avoid unnecessary locking).

An important question might be whether transactions remain in order after parallel processing, assuming the input
stream is ordered. This locking mechanism ensures that transactions for the same account are executed in the same
order. This is because a lock on account X prevents us from processing transaction *i + 1* until transaction *i* is completed.
  
I experimented a little bit with tokio by modifying function `process_transactions` in `engine.rs`:
```
let mut handles = Vec::new();
let mut stream = tokio_stream::iter(transacations_iter);
while let Some(record) = stream.next().await {
if let Ok(record) = record {
    let accounts = Arc::clone(&self.accounts);
    let handle = tokio::spawn(async move {
        Self::process_transaction( accounts, record).await
    });
    handles.push(handle);
}
}
for handle in handles {
    if let Err(e) = handle.await {
        eprintln!("Error: {:?}", e);
    }
}
```  
I haven't observed much improvement so decided to drop it as we still need to read input one by one so that we keep the order.  

But in general we could observe better performance if we would have multiple input streams. What we would have to have in mind are these 2:  
(1) each transactions stream is sorted (similarly as for original problem statement)  
(2) each transaction stream operates on different set of accounts otherwise we loose original order
of the transactions

### (2) precise calculations
Since we operate on financial data we need to make calculations precise hence we need type which requires significant integral and
fractional digits with no round-off errors. 
In this case I used type Decimal from [rust-decimal](https://github.com/paupino/rust-decimal) which is exactly for this purpose.

## Implementation structure
```
src /
    engine /
        engine.rs - brain coordinating transaction execution
        account_manager.rs - implements all transactions  
        account.rs, transaction.rs - types
    scripts /
        generate.py - script for generating example data
```
## Generate example data
```
python3 scripts/generate.py <num_records>
```

## Run
```
RUST_LOG=<log_level> cargo run -- <file.csv>
```

## Test
```
cargo test
```
