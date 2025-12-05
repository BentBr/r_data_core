#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

/// Test validation using JSON examples from `.example_files/json_examples` directory
#[cfg(test)]
#[allow(clippy::module_inception)]
mod validation_tests {
    use super::*;

    /// Load a JSON example file
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed
    fn load_json_example(filename: &str) -> Result<Value> {
        let path = format!(".example_files/json_examples/{filename}");
        let content = fs::read_to_string(&path).map_err(|e| {
            r_data_core_core::error::Error::Validation(format!("Failed to read {path}: {e}"))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            r_data_core_core::error::Error::Validation(format!("Failed to parse {path}: {e}"))
        })
    }

    /// Load a JSON example file from the `trigger_validation` subfolder
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed
    fn load_trigger_validation_example(filename: &str) -> Result<Value> {
        let path = format!(".example_files/json_examples/trigger_validation/{filename}");
        let content = fs::read_to_string(&path).map_err(|e| {
            r_data_core_core::error::Error::Validation(format!("Failed to read {path}: {e}"))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            r_data_core_core::error::Error::Validation(format!("Failed to parse {path}: {e}"))
        })
    }

    /// Create an entity definition from JSON
    ///
    /// # Errors
    /// Returns an error if the JSON cannot be deserialized into an `EntityDefinition`
    fn create_entity_definition_from_json(json_data: Value) -> Result<EntityDefinition> {
        serde_json::from_value(json_data).map_err(|e| {
            r_data_core_core::error::Error::Validation(format!(
                "Failed to deserialize entity definition: {e}"
            ))
        })
    }

    /// Create a dynamic entity for testing
    ///
    /// # Errors
    /// Returns an error if field validation fails
    fn create_test_entity(
        entity_type: &str,
        data: HashMap<String, Value>,
        entity_def: EntityDefinition,
    ) -> DynamicEntity {
        DynamicEntity::from_data(entity_type.to_string(), data, Arc::new(entity_def))
    }

    #[test]
    fn test_pattern_validation_email() -> Result<()> {
        // Load user entity definition which has email pattern validation
        let json_data = load_json_example("user_entity_definition.json")?;
        let entity_def = create_entity_definition_from_json(json_data)?;

        // Test valid email
        let mut valid_data = HashMap::new();
        valid_data.insert("email".to_string(), json!("test@example.com"));
        valid_data.insert("username".to_string(), json!("testuser"));
        valid_data.insert("first_name".to_string(), json!("John"));
        valid_data.insert("last_name".to_string(), json!("Doe"));
        valid_data.insert("role".to_string(), json!("customer"));
        valid_data.insert("status".to_string(), json!("active"));
        valid_data.insert("newsletter_opt_in".to_string(), json!(true));

        let valid_entity = create_test_entity("user", valid_data, entity_def.clone());
        let result = valid_entity.validate();
        assert!(
            result.is_ok(),
            "Valid email should pass validation: {result:?}"
        );

        // Test invalid email patterns
        let invalid_emails = vec![
            "invalid-email",
            "@example.com",
            "test@",
            "test@.com",
            "test@example",
            "",
        ];

        for invalid_email in invalid_emails {
            let mut invalid_data = HashMap::new();
            invalid_data.insert("email".to_string(), json!(invalid_email));
            invalid_data.insert("username".to_string(), json!("testuser"));
            invalid_data.insert("first_name".to_string(), json!("John"));
            invalid_data.insert("last_name".to_string(), json!("Doe"));
            invalid_data.insert("role".to_string(), json!("customer"));
            invalid_data.insert("status".to_string(), json!("active"));
            invalid_data.insert("newsletter_opt_in".to_string(), json!(true));

            let invalid_entity = create_test_entity("user", invalid_data, entity_def.clone());
            let result = invalid_entity.validate();
            assert!(
                result.is_err(),
                "Invalid email '{invalid_email}' should fail validation"
            );
        }

        Ok(())
    }

    #[test]
    fn test_pattern_validation_sku() -> Result<()> {
        // Load product entity definition which has SKU pattern validation
        let json_data = load_json_example("product_entity_definition.json")?;
        let entity_def = create_entity_definition_from_json(json_data)?;

        // Test valid SKU patterns
        let valid_skus = vec![
            "ABC123",
            "PROD001",
            "ITEM2024",
            "TEST",
            "LONGPRODUCTNAME123",
        ];

        for valid_sku in valid_skus {
            let mut valid_data = HashMap::new();
            valid_data.insert("sku".to_string(), json!(valid_sku));
            valid_data.insert("name".to_string(), json!("Test Product"));
            valid_data.insert("description".to_string(), json!("Test description"));
            valid_data.insert("category".to_string(), json!("Test"));
            valid_data.insert("brand".to_string(), json!("Test Brand"));
            valid_data.insert("price".to_string(), json!(10.99));
            valid_data.insert("cost".to_string(), json!(5.99));
            valid_data.insert("tax_category".to_string(), json!("standard"));
            valid_data.insert("quantity_in_stock".to_string(), json!(100));
            valid_data.insert("min_stock_level".to_string(), json!(10));
            valid_data.insert("status".to_string(), json!("active"));
            valid_data.insert("weight".to_string(), json!(1.5));
            valid_data.insert("weight_unit".to_string(), json!("kg"));
            valid_data.insert("seo_title".to_string(), json!("Test Product SEO"));
            valid_data.insert(
                "seo_description".to_string(),
                json!("Test product description"),
            );

            let valid_entity = create_test_entity("product", valid_data, entity_def.clone());
            let result = valid_entity.validate();
            assert!(
                result.is_ok(),
                "Valid SKU '{valid_sku}' should pass validation: {result:?}"
            );
        }

        // Test invalid SKU patterns
        let invalid_skus = vec![
            "abc123",                                         // lowercase not allowed
            "ABC-123",                                        // hyphens not allowed
            "ABC_123",                                        // underscores not allowed
            "A",                                              // too short
            "VERYLONGPRODUCTNAMETHATEXCEEDSTWENTYCHARACTERS", // too long
            "ABC 123",                                        // spaces not allowed
            "",
        ];

        for invalid_sku in invalid_skus {
            let mut invalid_data = HashMap::new();
            invalid_data.insert("sku".to_string(), json!(invalid_sku));
            invalid_data.insert("name".to_string(), json!("Test Product"));
            invalid_data.insert("description".to_string(), json!("Test description"));
            invalid_data.insert("category".to_string(), json!("Test"));
            invalid_data.insert("brand".to_string(), json!("Test Brand"));
            invalid_data.insert("price".to_string(), json!(10.99));
            invalid_data.insert("cost".to_string(), json!(5.99));
            invalid_data.insert("tax_category".to_string(), json!("standard"));
            invalid_data.insert("quantity_in_stock".to_string(), json!(100));
            invalid_data.insert("min_stock_level".to_string(), json!(10));
            invalid_data.insert("status".to_string(), json!("active"));
            invalid_data.insert("weight".to_string(), json!(1.5));
            invalid_data.insert("weight_unit".to_string(), json!("kg"));
            invalid_data.insert("seo_title".to_string(), json!("Test Product SEO"));
            invalid_data.insert(
                "seo_description".to_string(),
                json!("Test product description"),
            );

            let invalid_entity = create_test_entity("product", invalid_data, entity_def.clone());
            let result = invalid_entity.validate();
            assert!(
                result.is_err(),
                "Invalid SKU '{invalid_sku}' should fail validation"
            );
        }

        Ok(())
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Test function with comprehensive validation scenarios
    fn test_range_validation_numeric() -> Result<()> {
        // Load product entity definition which has numeric range validation
        let json_data = load_json_example("product_entity_definition.json")?;
        let entity_def = create_entity_definition_from_json(json_data)?;

        // Test valid numeric ranges
        let valid_prices = vec![0.0, 10.99, 100.50, 999.99, 1000.0];
        let valid_quantities = vec![0, 1, 100, 1000, 9999];

        for price in valid_prices {
            let mut valid_data = HashMap::new();
            valid_data.insert("sku".to_string(), json!("TEST123"));
            valid_data.insert("name".to_string(), json!("Test Product"));
            valid_data.insert("description".to_string(), json!("Test description"));
            valid_data.insert("category".to_string(), json!("Test"));
            valid_data.insert("brand".to_string(), json!("Test Brand"));
            valid_data.insert("price".to_string(), json!(price));
            valid_data.insert("cost".to_string(), json!(5.99));
            valid_data.insert("tax_category".to_string(), json!("standard"));
            valid_data.insert("quantity_in_stock".to_string(), json!(100));
            valid_data.insert("min_stock_level".to_string(), json!(10));
            valid_data.insert("status".to_string(), json!("active"));
            valid_data.insert("weight".to_string(), json!(1.5));
            valid_data.insert("weight_unit".to_string(), json!("kg"));
            valid_data.insert("seo_title".to_string(), json!("Test Product SEO"));
            valid_data.insert(
                "seo_description".to_string(),
                json!("Test product description"),
            );

            let valid_entity = create_test_entity("product", valid_data, entity_def.clone());
            let result = valid_entity.validate();
            assert!(
                result.is_ok(),
                "Valid price {price} should pass validation: {result:?}"
            );
        }

        for quantity in valid_quantities {
            let mut valid_data = HashMap::new();
            valid_data.insert("sku".to_string(), json!("TEST123"));
            valid_data.insert("name".to_string(), json!("Test Product"));
            valid_data.insert("description".to_string(), json!("Test description"));
            valid_data.insert("category".to_string(), json!("Test"));
            valid_data.insert("brand".to_string(), json!("Test Brand"));
            valid_data.insert("price".to_string(), json!(10.99));
            valid_data.insert("cost".to_string(), json!(5.99));
            valid_data.insert("tax_category".to_string(), json!("standard"));
            valid_data.insert("quantity_in_stock".to_string(), json!(quantity));
            valid_data.insert("min_stock_level".to_string(), json!(10));
            valid_data.insert("status".to_string(), json!("active"));
            valid_data.insert("weight".to_string(), json!(1.5));
            valid_data.insert("weight_unit".to_string(), json!("kg"));
            valid_data.insert("seo_title".to_string(), json!("Test Product SEO"));
            valid_data.insert(
                "seo_description".to_string(),
                json!("Test product description"),
            );

            let valid_entity = create_test_entity("product", valid_data, entity_def.clone());
            let result = valid_entity.validate();
            assert!(
                result.is_ok(),
                "Valid quantity {quantity} should pass validation: {result:?}"
            );
        }

        // Test invalid numeric ranges
        let invalid_prices = vec![-1.0, -10.0, -100.0];
        let invalid_quantities = vec![-1, -10, -100];

        for price in invalid_prices {
            let mut invalid_data = HashMap::new();
            invalid_data.insert("sku".to_string(), json!("TEST123"));
            invalid_data.insert("name".to_string(), json!("Test Product"));
            invalid_data.insert("description".to_string(), json!("Test description"));
            invalid_data.insert("category".to_string(), json!("Test"));
            invalid_data.insert("brand".to_string(), json!("Test Brand"));
            invalid_data.insert("price".to_string(), json!(price));
            invalid_data.insert("cost".to_string(), json!(5.99));
            invalid_data.insert("tax_category".to_string(), json!("standard"));
            invalid_data.insert("quantity_in_stock".to_string(), json!(100));
            invalid_data.insert("min_stock_level".to_string(), json!(10));
            invalid_data.insert("status".to_string(), json!("active"));
            invalid_data.insert("weight".to_string(), json!(1.5));
            invalid_data.insert("weight_unit".to_string(), json!("kg"));
            invalid_data.insert("seo_title".to_string(), json!("Test Product SEO"));
            invalid_data.insert(
                "seo_description".to_string(),
                json!("Test product description"),
            );

            let invalid_entity = create_test_entity("product", invalid_data, entity_def.clone());
            let result = invalid_entity.validate();
            assert!(
                result.is_err(),
                "Invalid price {price} should fail validation"
            );
        }

        for quantity in invalid_quantities {
            let mut invalid_data = HashMap::new();
            invalid_data.insert("sku".to_string(), json!("TEST123"));
            invalid_data.insert("name".to_string(), json!("Test Product"));
            invalid_data.insert("description".to_string(), json!("Test description"));
            invalid_data.insert("category".to_string(), json!("Test"));
            invalid_data.insert("brand".to_string(), json!("Test Brand"));
            invalid_data.insert("price".to_string(), json!(10.99));
            invalid_data.insert("cost".to_string(), json!(5.99));
            invalid_data.insert("tax_category".to_string(), json!("standard"));
            invalid_data.insert("quantity_in_stock".to_string(), json!(quantity));
            invalid_data.insert("min_stock_level".to_string(), json!(10));
            invalid_data.insert("status".to_string(), json!("active"));
            invalid_data.insert("weight".to_string(), json!(1.5));
            invalid_data.insert("weight_unit".to_string(), json!("kg"));
            invalid_data.insert("seo_title".to_string(), json!("Test Product SEO"));
            invalid_data.insert(
                "seo_description".to_string(),
                json!("Test product description"),
            );

            let invalid_entity = create_test_entity("product", invalid_data, entity_def.clone());
            let result = invalid_entity.validate();
            assert!(
                result.is_err(),
                "Invalid quantity {quantity} should fail validation"
            );
        }

        Ok(())
    }

    #[test]
    fn test_enum_validation() -> Result<()> {
        // Load product entity definition which has enum validation
        let json_data = load_json_example("product_entity_definition.json")?;
        let entity_def = create_entity_definition_from_json(json_data)?;

        // Test valid enum values
        let valid_tax_categories = vec!["standard", "reduced", "zero", "exempt"];

        for tax_category in valid_tax_categories {
            let mut valid_data = HashMap::new();
            valid_data.insert("sku".to_string(), json!("TEST123"));
            valid_data.insert("name".to_string(), json!("Test Product"));
            valid_data.insert("description".to_string(), json!("Test description"));
            valid_data.insert("category".to_string(), json!("Test"));
            valid_data.insert("brand".to_string(), json!("Test Brand"));
            valid_data.insert("price".to_string(), json!(10.99));
            valid_data.insert("cost".to_string(), json!(5.99));
            valid_data.insert("tax_category".to_string(), json!(tax_category));
            valid_data.insert("quantity_in_stock".to_string(), json!(100));
            valid_data.insert("min_stock_level".to_string(), json!(10));
            valid_data.insert("status".to_string(), json!("active"));
            valid_data.insert("weight".to_string(), json!(1.5));
            valid_data.insert("weight_unit".to_string(), json!("kg"));
            valid_data.insert("seo_title".to_string(), json!("Test Product SEO"));
            valid_data.insert(
                "seo_description".to_string(),
                json!("Test product description"),
            );

            let valid_entity = create_test_entity("product", valid_data, entity_def.clone());
            let result = valid_entity.validate();
            assert!(
                result.is_ok(),
                "Valid tax category '{tax_category}' should pass validation: {result:?}"
            );
        }

        // Test invalid enum values
        let invalid_tax_categories = vec!["invalid", "unknown", "wrong"];

        for tax_category in invalid_tax_categories {
            let mut invalid_data = HashMap::new();
            invalid_data.insert("sku".to_string(), json!("TEST123"));
            invalid_data.insert("name".to_string(), json!("Test Product"));
            invalid_data.insert("description".to_string(), json!("Test description"));
            invalid_data.insert("category".to_string(), json!("Test"));
            invalid_data.insert("brand".to_string(), json!("Test Brand"));
            invalid_data.insert("price".to_string(), json!(10.99));
            invalid_data.insert("cost".to_string(), json!(5.99));
            invalid_data.insert("tax_category".to_string(), json!(tax_category));
            invalid_data.insert("quantity_in_stock".to_string(), json!(100));
            invalid_data.insert("min_stock_level".to_string(), json!(10));
            invalid_data.insert("status".to_string(), json!("active"));
            invalid_data.insert("weight".to_string(), json!(1.5));
            invalid_data.insert("weight_unit".to_string(), json!("kg"));
            invalid_data.insert("seo_title".to_string(), json!("Test Product SEO"));
            invalid_data.insert(
                "seo_description".to_string(),
                json!("Test product description"),
            );

            let invalid_entity = create_test_entity("product", invalid_data, entity_def.clone());
            let result = invalid_entity.validate();
            assert!(
                result.is_err(),
                "Invalid tax category '{tax_category}' should fail validation"
            );
        }

        Ok(())
    }

    #[test]
    fn test_edge_cases_empty_strings() -> Result<()> {
        // Load user entity definition
        let json_data = load_json_example("user_entity_definition.json")?;
        let entity_def = create_entity_definition_from_json(json_data)?;

        // Test empty strings for required fields
        let mut invalid_data = HashMap::new();
        invalid_data.insert("email".to_string(), json!(""));
        invalid_data.insert("username".to_string(), json!("testuser"));
        invalid_data.insert("first_name".to_string(), json!("John"));
        invalid_data.insert("last_name".to_string(), json!("Doe"));
        invalid_data.insert("role".to_string(), json!("customer"));
        invalid_data.insert("status".to_string(), json!("active"));
        invalid_data.insert("newsletter_opt_in".to_string(), json!(true));

        let invalid_entity = create_test_entity("user", invalid_data, entity_def.clone());
        let result = invalid_entity.validate();
        assert!(result.is_err(), "Empty email should fail validation");

        // Test empty strings for optional fields (should pass)
        let mut valid_data = HashMap::new();
        valid_data.insert("email".to_string(), json!("test@example.com"));
        valid_data.insert("username".to_string(), json!("testuser"));
        valid_data.insert("first_name".to_string(), json!("John"));
        valid_data.insert("last_name".to_string(), json!("Doe"));
        valid_data.insert("role".to_string(), json!("customer"));
        valid_data.insert("status".to_string(), json!("active"));
        valid_data.insert("newsletter_opt_in".to_string(), json!(true));
        valid_data.insert("phone".to_string(), json!("")); // Optional field

        let valid_entity = create_test_entity("user", valid_data, entity_def);
        let result = valid_entity.validate();
        assert!(
            result.is_ok(),
            "Empty optional field should pass validation"
        );

        Ok(())
    }

    #[test]
    fn test_edge_cases_null_values() -> Result<()> {
        // Load user entity definition
        let json_data = load_json_example("user_entity_definition.json")?;
        let entity_def = create_entity_definition_from_json(json_data)?;

        // Test null values for required fields
        let mut invalid_data = HashMap::new();
        invalid_data.insert("email".to_string(), Value::Null);
        invalid_data.insert("username".to_string(), json!("testuser"));
        invalid_data.insert("first_name".to_string(), json!("John"));
        invalid_data.insert("last_name".to_string(), json!("Doe"));
        invalid_data.insert("role".to_string(), json!("customer"));
        invalid_data.insert("status".to_string(), json!("active"));
        invalid_data.insert("newsletter_opt_in".to_string(), json!(true));

        let invalid_entity = create_test_entity("user", invalid_data, entity_def.clone());
        let result = invalid_entity.validate();
        assert!(
            result.is_err(),
            "Null required field should fail validation"
        );

        // Test null values for optional fields (should pass)
        let mut valid_data = HashMap::new();
        valid_data.insert("email".to_string(), json!("test@example.com"));
        valid_data.insert("username".to_string(), json!("testuser"));
        valid_data.insert("first_name".to_string(), json!("John"));
        valid_data.insert("last_name".to_string(), json!("Doe"));
        valid_data.insert("role".to_string(), json!("customer"));
        valid_data.insert("status".to_string(), json!("active"));
        valid_data.insert("newsletter_opt_in".to_string(), json!(true));
        valid_data.insert("phone".to_string(), Value::Null); // Optional field

        let valid_entity = create_test_entity("user", valid_data, entity_def);
        let result = valid_entity.validate();
        assert!(result.is_ok(), "Null optional field should pass validation");

        Ok(())
    }

    #[test]
    fn test_edge_cases_missing_required_fields() -> Result<()> {
        // Load user entity definition
        let json_data = load_json_example("user_entity_definition.json")?;
        let entity_def = create_entity_definition_from_json(json_data)?;

        // Test missing required fields
        let mut invalid_data = HashMap::new();
        // Missing email, username, first_name, last_name, role, status
        invalid_data.insert("phone".to_string(), json!("1234567890"));

        let invalid_entity = create_test_entity("user", invalid_data, entity_def.clone());
        let result = invalid_entity.validate();
        assert!(
            result.is_err(),
            "Missing required fields should fail validation"
        );

        // Test with some required fields missing
        let mut partial_data = HashMap::new();
        partial_data.insert("email".to_string(), json!("test@example.com"));
        partial_data.insert("username".to_string(), json!("testuser"));
        partial_data.insert("first_name".to_string(), json!("John"));
        // Missing last_name, role, status, newsletter_opt_in (all required)

        let partial_entity = create_test_entity("user", partial_data, entity_def);
        let result = partial_entity.validate();
        assert!(
            result.is_err(),
            "Partially missing required fields should fail validation"
        );

        Ok(())
    }

    #[test]
    fn test_comprehensive_validation_scenarios() -> Result<()> {
        // Load product entity definition for comprehensive testing
        let json_data = load_json_example("product_entity_definition.json")?;
        let entity_def = create_entity_definition_from_json(json_data)?;

        // Test comprehensive valid product
        let mut valid_data = HashMap::new();
        valid_data.insert("sku".to_string(), json!("COMPREHENSIVE123"));
        valid_data.insert("name".to_string(), json!("Comprehensive Test Product"));
        valid_data.insert(
            "description".to_string(),
            json!("A comprehensive test product"),
        );
        valid_data.insert("category".to_string(), json!("Test Category"));
        valid_data.insert("brand".to_string(), json!("Test Brand"));
        valid_data.insert("price".to_string(), json!(99.99));
        valid_data.insert("cost".to_string(), json!(50.00));
        valid_data.insert("tax_category".to_string(), json!("standard"));
        valid_data.insert("quantity_in_stock".to_string(), json!(100));
        valid_data.insert("min_stock_level".to_string(), json!(10));
        valid_data.insert("status".to_string(), json!("active"));
        valid_data.insert("weight".to_string(), json!(1.5));
        valid_data.insert("weight_unit".to_string(), json!("kg"));
        valid_data.insert("seo_title".to_string(), json!("Valid Product SEO"));
        valid_data.insert(
            "seo_description".to_string(),
            json!("Valid product description"),
        );

        let valid_entity = create_test_entity("product", valid_data, entity_def.clone());
        let result = valid_entity.validate();
        assert!(
            result.is_ok(),
            "Comprehensive valid product should pass validation: {result:?}"
        );

        // Test product with multiple validation errors
        let mut invalid_data = HashMap::new();
        invalid_data.insert("sku".to_string(), json!("invalid-sku")); // Invalid pattern
        invalid_data.insert("name".to_string(), json!("AB")); // Too short
        invalid_data.insert("description".to_string(), json!("Test description"));
        invalid_data.insert("category".to_string(), json!("Test"));
        invalid_data.insert("brand".to_string(), json!("Test Brand"));
        invalid_data.insert("price".to_string(), json!(-10.99)); // Negative value
        invalid_data.insert("cost".to_string(), json!(5.99));
        invalid_data.insert("tax_category".to_string(), json!("invalid_category")); // Invalid enum
        invalid_data.insert("quantity_in_stock".to_string(), json!(-5)); // Negative value
        invalid_data.insert("min_stock_level".to_string(), json!(10));
        invalid_data.insert("status".to_string(), json!("active"));
        invalid_data.insert("weight".to_string(), json!(1.5));
        invalid_data.insert("weight_unit".to_string(), json!("kg"));
        invalid_data.insert("seo_title".to_string(), json!("Invalid Product SEO"));
        invalid_data.insert(
            "seo_description".to_string(),
            json!("Invalid product description"),
        );

        let invalid_entity = create_test_entity("product", invalid_data, entity_def);
        let result = invalid_entity.validate();
        assert!(
            result.is_err(),
            "Product with multiple validation errors should fail"
        );

        Ok(())
    }

    #[test]
    fn test_load_trigger_validation_examples() -> Result<()> {
        // Test loading trigger validation example files
        let email_patterns = load_trigger_validation_example("invalid_email_patterns.json")?;
        let entity_def = create_entity_definition_from_json(email_patterns)?;
        assert_eq!(entity_def.entity_type, "user");
        assert_eq!(entity_def.fields.len(), 4);

        let enum_values = load_trigger_validation_example("invalid_enum_values.json")?;
        let entity_def = create_entity_definition_from_json(enum_values)?;
        assert_eq!(entity_def.entity_type, "product");
        assert!(entity_def.fields.iter().any(|f| f.name == "tax_category"));

        let numeric_ranges = load_trigger_validation_example("invalid_numeric_ranges.json")?;
        let entity_def = create_entity_definition_from_json(numeric_ranges)?;
        assert_eq!(entity_def.entity_type, "product");
        assert!(entity_def.fields.iter().any(|f| f.name == "price"));

        Ok(())
    }
}
