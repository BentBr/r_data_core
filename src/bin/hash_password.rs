use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, Params,
};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <password>", args[0]);
        return;
    }

    let password = &args[1];

    // Use standardized Argon2id parameters:
    // - m=19456 (19 MB memory cost - good balance for modern systems)
    // - t=2 (2 iterations - recommended minimum)
    // - p=1 (1 parallelism - good for most systems)
    let params = match Params::new(19456, 2, 1, None) {
        Ok(p) => p,
        Err(e) => {
            println!("Error creating parameters: {}", e);
            return;
        }
    };

    // Generate salt
    let salt = SaltString::generate(&mut OsRng);

    // Create Argon2id hasher
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    // Hash password
    let hash = match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(e) => {
            println!("Error hashing password: {}", e);
            return;
        }
    };

    println!("Password: {}", password);
    println!("Hash: {}", hash);

    // SQL Insert statement
    println!("\nSQL:");
    println!(
        "UPDATE admin_users SET password_hash = '{}' WHERE username = 'admin';",
        hash.replace("'", "''")
    );
}
