use r_data_core::entity::field::{FieldDefinition, FieldType};
use r_data_core::entity::ClassDefinition;
use uuid::Uuid;

fn main() {
    // Create a sample class definition with some custom fields
    let mut class_def = ClassDefinition::new(
        "customer".to_string(),
        "Customer".to_string(),
        Some("Customer entity".to_string()),
        Some("CRM".to_string()),
        false,
        Some("user".to_string()),
        Vec::new(),
        Uuid::nil(),
    );

    // Add some sample fields
    let first_name = FieldDefinition::new(
        "firstName".to_string(),
        "First Name".to_string(),
        FieldType::String,
    );

    let last_name = FieldDefinition::new(
        "lastName".to_string(),
        "Last Name".to_string(),
        FieldType::String,
    );

    let email = FieldDefinition::new("email".to_string(), "Email".to_string(), FieldType::String);

    class_def.add_field(first_name).unwrap();
    class_def.add_field(last_name).unwrap();
    class_def.add_field(email).unwrap();

    // Generate the SQL schema
    let schema_sql = class_def.generate_schema_sql();

    // Print the generated SQL
    println!(
        "Generated SQL Schema for '{}' entity type:",
        class_def.entity_type
    );
    println!("{}", schema_sql);
    println!("\nTable name: {}", class_def.get_table_name());
}
