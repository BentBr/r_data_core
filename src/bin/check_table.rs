use sqlx::{postgres::PgPoolOptions, Row};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up logging
    env_logger::init();

    // Get database URL from environment or use default
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/r_data_core".to_string());

    // Get table name from command line argument
    let args: Vec<String> = env::args().collect();
    let table_name = if args.len() > 1 {
        args[1].clone()
    } else {
        eprintln!("Usage: {} <table_name>", args[0]);
        return Ok(());
    };

    println!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Checking columns for table: {}", table_name);

    // Query to get all columns
    let query = "
        SELECT column_name, data_type, is_nullable 
        FROM information_schema.columns 
        WHERE table_schema = 'public' AND table_name = $1
        ORDER BY ordinal_position
    ";

    let rows = sqlx::query(query)
        .bind(&table_name)
        .fetch_all(&pool)
        .await?;

    if rows.is_empty() {
        println!("Table '{}' not found in database", table_name);
        return Ok(());
    }

    println!("Table: {} (found {} columns)", table_name, rows.len());
    println!("{:-^60}", "");
    println!(
        "{:<20} {:<20} {:<10}",
        "Column Name", "Data Type", "Nullable"
    );
    println!("{:-^60}", "");

    for row in rows {
        let column_name: String = row.get("column_name");
        let data_type: String = row.get("data_type");
        let is_nullable: String = row.get("is_nullable");

        println!(
            "{:<20} {:<20} {:<10}",
            column_name,
            data_type,
            if is_nullable == "YES" { "YES" } else { "NO" }
        );
    }

    println!("{:-^60}", "");

    Ok(())
}
