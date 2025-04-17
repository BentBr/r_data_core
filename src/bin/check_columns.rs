use dotenv::dotenv;
use sqlx::{postgres::PgPoolOptions, Row};
use std::{env, process};

#[tokio::main]
async fn main() {
    // Load .env file
    dotenv().ok();

    // Get the table name from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <table_name>", args[0]);
        process::exit(1);
    }

    let table_name = &args[1];

    // Get database URL from environment
    let database_url = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("DATABASE_URL not set in environment or .env file");
            process::exit(1);
        }
    };

    println!("Using database URL: {}", database_url);
    println!("Checking columns for table: {}", table_name);

    // Connect to database
    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Failed to connect to database: {}", e);
            process::exit(1);
        }
    };

    // Get column information
    let query = "
        SELECT column_name, data_type, is_nullable 
        FROM information_schema.columns 
        WHERE table_schema = 'public' AND table_name = $1
        ORDER BY ordinal_position
    ";

    let result = sqlx::query(query).bind(table_name).fetch_all(&pool).await;

    match result {
        Ok(rows) => {
            if rows.is_empty() {
                println!("No columns found for table '{}'", table_name);
                return;
            }

            println!("Found {} columns:", rows.len());
            for row in rows {
                let column_name: &str = row.try_get("column_name").unwrap_or("unknown");
                let data_type: &str = row.try_get("data_type").unwrap_or("unknown");
                let is_nullable: &str = row.try_get("is_nullable").unwrap_or("unknown");

                println!(
                    "  {} ({}) - {}",
                    column_name,
                    data_type,
                    if is_nullable == "YES" {
                        "nullable"
                    } else {
                        "not null"
                    }
                );
            }
        }
        Err(e) => {
            eprintln!("Error querying columns: {}", e);
            process::exit(1);
        }
    }
}
