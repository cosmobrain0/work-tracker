mod state;

use tokio_postgres::{
    types::{FromSql, Type},
    Error, NoTls,
};

use dotenvy::dotenv;
use std::env;

#[macro_export]
macro_rules! take {
    ($vec:expr, $n:expr) => {{
        let mut x = $vec.split_off($vec.len() - $n);
        x.reverse();
        x
    }};
}

#[macro_export]
macro_rules! take_u32 {
    ($vec: expr, $n: expr) => {
        u32::from_be_bytes(take!($vec, $n).try_into()?)
    };
}

#[derive(Debug, Clone)]
struct Test {
    name: String,
    age: i32,
}
impl<'a> FromSql<'a> for Test {
    fn from_sql(
        ty: &tokio_postgres::types::Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let mut raw = raw.into_iter().map(|x| *x).rev().collect::<Vec<_>>();

        // two fields...
        assert_eq!(take!(raw, 4)[..], [0, 0, 0, 2]);
        // ...starting with a VARCHAR...
        assert_eq!(take!(raw, 4)[..], [0, 0, 4, 19]);
        let length = u32::from_be_bytes(take!(raw, 4)[..].try_into()?) as usize;
        let name = String::from_sql(&Type::from_oid(1043).unwrap(), &take!(raw, length))?;
        // ...followed by an INT4...
        assert_eq!(take!(raw, 4), [0, 0, 0, 23]);
        // ...which is 4 bytes long
        assert_eq!(take!(raw, 4), [0, 0, 0, 4]);
        let age = i32::from_sql(&Type::from_oid(23).unwrap(), &take!(raw, 4))?;
        Ok(Test { name, age })
    }

    fn accepts(ty: &tokio_postgres::types::Type) -> bool {
        ty.name() == "test_type"
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().expect("Couldn't load .env!");
    let password = env::var("PASSWORD").expect("Couldn't get the password from .env!");
    let host = env::var("HOST").expect("Couldn't get the host from .env!");
    let user = env::var("USER").expect("Couldn't get the user from .env!");
    let dbname = env::var("DBNAME").expect("Couldn't get the dbname from .env!");
    let (client, connection) = tokio_postgres::connect(
        format!("host={host} user={user} password={password} dbname={dbname}").as_str(),
        NoTls,
    )
    .await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection nerror: {}", e);
        }
    });
    let result = client.query("SELECT id, person FROM example", &[]).await?;
    for row in result {
        let id: i32 = row.get(0);
        let person: Test = row.get(1);
        dbg!(id, person);
    }
    Ok(())
}
