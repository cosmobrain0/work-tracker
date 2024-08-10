mod state;

use tokio_postgres::{Error, NoTls};

use dotenvy::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().expect("Couldn't load .env!");
    let password = env::var("PASSWORD").expect("Couldn't get the password from .env!");
    let (client, connection) = tokio_postgres::connect(
        format!("host=localhost user=cosmo password={password} dbname=mydb").as_str(),
        NoTls,
    )
    .await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection nerror: {}", e);
        }
    });
    let result = client.query("SELECT name, age FROM example", &[]).await?;
    let name: String = result[0].get("name");
    let age: i32 = result[0].get("age");
    dbg!(name, age);
    Ok(())
}
