mod state;

use tokio_postgres::Error;

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

#[tokio::main]
async fn main() -> Result<(), Error> {
    Ok(())
}
