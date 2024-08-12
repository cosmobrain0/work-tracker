mod state;

use tokio_postgres::{
    types::{FromSql, Type},
    Error, NoTls,
};

use dotenvy::dotenv;
use std::env;

use crate::state::Payment;

fn get_u32(raw: &[u8]) -> u32 {
    u32::from_be_bytes(raw[0..4].try_into().unwrap())
}

fn pop_u32(raw: &mut Vec<u8>) -> u32 {
    let number = get_u32(&raw[..]);
    *raw = raw.into_iter().skip(4).map(|x| *x).collect::<Vec<u8>>();
    number
}

fn get_data(raw: &[u8]) -> (u32, usize, &[u8]) {
    let oid = u32::from_be_bytes(raw[0..4].try_into().unwrap());
    let length = u32::from_be_bytes(raw[4..8].try_into().unwrap()) as usize;
    (oid, length, &raw[8..8 + length])
}

fn pop_data(raw: &mut Vec<u8>) -> (u32, Vec<u8>) {
    let (oid, length, slice) = get_data(&raw[..]);
    let slice = slice.to_vec();
    *raw = raw.split_at(length + 8).1.to_vec();
    (oid, slice)
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
        let mut raw = raw.to_vec();
        let field_count = pop_u32(&mut raw);
        assert_eq!(field_count, 2);

        let (name_oid, name) = pop_data(&mut raw);
        let (age_oid, age) = pop_data(&mut raw);

        assert_eq!(name_oid, 1043);
        assert_eq!(age_oid, 23);

        let name = String::from_utf8(name)?;
        let age = i32::from_be_bytes(age[0..4].try_into()?);

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
    let result = client
        .query("SELECT job_name, payment FROM example_payment", &[])
        .await?;
    for row in result {
        let job_name: String = row.get(0);
        let payment: Payment = row.get(1);
        dbg!(job_name, payment);
    }
    Ok(())
}
