#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use argon2::{password_hash::rand_core::OsRng, password_hash::SaltString, Argon2, PasswordHasher};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Usage: hash_password <PASSWORD>");
        return;
    }

    let password = &args[1];

    // Generate a random salt and hash the password using Argon2id defaults
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("failed to hash password")
        .to_string();

    println!("Password: {password}");
    println!("Hash: {hash}");
    println!("SQL:");
    println!("UPDATE admin_users SET password_hash = '{hash}' WHERE username = '<USERNAME>';");
}
