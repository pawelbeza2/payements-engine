use std::{env};

mod engine;
use engine::Engine;

fn main() -> anyhow::Result<()>  {
    let file_path = match env::args().nth(1) {
        None => Err(anyhow::anyhow!("Expecting one argument")),
        Some(file_path) => Ok(file_path),
    }?;

    let reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(file_path)?;

    let mut engine = Engine::new();
    if let Err(e) = engine.process_transactions(reader.into_deserialize()) {
        return Err(anyhow::anyhow!("Error processing transactions: {}", e));
    }

    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .delimiter(b',')
        .flexible(false)
        .from_writer(std::io::stdout());

    for account in engine.accounts()? {
        writer.serialize(account)?;
    }

    Ok(())
}