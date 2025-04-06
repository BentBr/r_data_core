use crate::entity::field::{FieldType, OptionsSource, get_sql_type_for_field};
use super::definition::ClassDefinition;

impl ClassDefinition {
    /// Generate SQL table schema for this class
    pub fn generate_sql_schema(&self) -> String {
        let table_name = self.get_table_name();
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table_name);
        sql.push_str("  id BIGSERIAL PRIMARY KEY,\n");
        sql.push_str("  uuid UUID NOT NULL,\n");
        sql.push_str("  path TEXT NOT NULL,\n");
        sql.push_str("  created_at TIMESTAMP WITH TIME ZONE NOT NULL,\n");
        sql.push_str("  updated_at TIMESTAMP WITH TIME ZONE NOT NULL,\n");
        sql.push_str("  created_by BIGINT,\n");
        sql.push_str("  updated_by BIGINT,\n");
        sql.push_str("  published BOOLEAN NOT NULL DEFAULT FALSE,\n");
        sql.push_str("  version INTEGER NOT NULL DEFAULT 1,\n");
        
        // Add custom fields that should be columns
        for field in &self.fields {
            // Skip relation fields as they'll be in relation tables
            if matches!(field.field_type, FieldType::ManyToOne | FieldType::ManyToMany) {
                // For ManyToOne, we do need a reference column in this table
                if matches!(field.field_type, FieldType::ManyToOne) {
                    if let Some(_) = &field.validation.target_class {
                        sql.push_str(&format!("  {}_id BIGINT,\n", field.name));
                    }
                }
                continue;
            }
            
            let enum_name = if matches!(field.field_type, FieldType::Select) {
                if let Some(OptionsSource::Enum { enum_name }) = &field.validation.options_source {
                    Some(enum_name.as_str())
                } else {
                    None
                }
            } else {
                None
            };
            
            let sql_type = get_sql_type_for_field(&field.field_type, field.validation.max_length, enum_name);
            let null_constraint = if field.required { "NOT NULL" } else { "" };
            
            // Add constraints if applicable
            let mut constraints = String::new();
            
            // Add numeric constraints
            if matches!(field.field_type, FieldType::Integer | FieldType::Float) {
                // Add CHECK constraints for min/max/positive
                let mut checks = Vec::new();
                
                if let Some(min) = &field.validation.min_value {
                    if let Some(min_num) = min.as_i64().or_else(|| min.as_f64().map(|f| f as i64)) {
                        checks.push(format!("{} >= {}", field.name, min_num));
                    }
                }
                
                if let Some(max) = &field.validation.max_value {
                    if let Some(max_num) = max.as_i64().or_else(|| max.as_f64().map(|f| f as i64)) {
                        checks.push(format!("{} <= {}", field.name, max_num));
                    }
                }
                
                if let Some(true) = field.validation.positive_only {
                    checks.push(format!("{} >= 0", field.name));
                }
                
                if !checks.is_empty() {
                    constraints = format!(" CHECK ({})", checks.join(" AND "));
                }
            }
            
            sql.push_str(&format!("  {} {}{}{},\n", field.name, sql_type, null_constraint, constraints));
        }
        
        // Add custom_fields JSONB for any additional fields not in schema
        sql.push_str("  custom_fields JSONB NOT NULL DEFAULT '{}'\n");
        sql.push_str(");\n");
        
        // Add indexes for searchable fields
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_uuid ON {} (uuid);\n", 
            table_name, table_name));
            
        for field in &self.fields {
            if field.indexed && !matches!(field.field_type, FieldType::ManyToMany) {
                // For ManyToOne fields, index the foreign key
                if matches!(field.field_type, FieldType::ManyToOne) {
                    if field.validation.target_class.is_some() {
                        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_{}_id ON {} ({}_id);\n", 
                            table_name, field.name, table_name, field.name));
                    }
                } else if !matches!(field.field_type, FieldType::Object | FieldType::Array) {
                    sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_{} ON {} ({});\n", 
                        table_name, field.name, table_name, field.name));
                }
            }
        }
        
        // Generate relation tables
        sql.push_str(&self.generate_relation_tables());
        
        sql
    }
    
    /// Generate relation tables for this class
    fn generate_relation_tables(&self) -> String {
        let mut sql = String::new();
        let table_name = self.get_table_name();
        
        for field in &self.fields {
            if matches!(field.field_type, FieldType::ManyToOne | FieldType::ManyToMany) {
                if let Some(target_class) = &field.validation.target_class {
                    let target_table = format!("entity_{}", target_class.to_lowercase());
                    
                    // For ManyToOne, add foreign key constraint
                    if matches!(field.field_type, FieldType::ManyToOne) {
                        sql.push_str(&format!(
                            "ALTER TABLE {} ADD CONSTRAINT fk_{}_{} FOREIGN KEY ({}_id) REFERENCES {} (id) ON DELETE SET NULL;\n",
                            table_name, table_name, field.name, field.name, target_table
                        ));
                    }
                    
                    // For ManyToMany, create a join table
                    if matches!(field.field_type, FieldType::ManyToMany) {
                        let relation_table = format!("{}_{}_{}_relation", 
                            self.class_name.to_lowercase(), 
                            field.name,
                            target_class.to_lowercase());
                            
                        sql.push_str(&format!("CREATE TABLE IF NOT EXISTS {} (\n", relation_table));
                        sql.push_str("  id BIGSERIAL PRIMARY KEY,\n");
                        
                        // Reference to this entity
                        sql.push_str(&format!("  {}_id BIGINT NOT NULL REFERENCES {} (id) ON DELETE CASCADE,\n", 
                            self.class_name.to_lowercase(), table_name));
                            
                        // Reference to target entity
                        sql.push_str(&format!("  {}_id BIGINT NOT NULL REFERENCES {} (id) ON DELETE CASCADE,\n", 
                            target_class.to_lowercase(), target_table));
                            
                        // Add position field for ordered relations and metadata
                        sql.push_str("  position INTEGER NOT NULL DEFAULT 0,\n");
                        sql.push_str("  metadata JSONB,\n");
                        
                        // Add unique constraint to prevent duplicates
                        sql.push_str(&format!("  UNIQUE({}_id, {}_id)\n", 
                            self.class_name.to_lowercase(), target_class.to_lowercase()));
                        sql.push_str(");\n");
                        
                        // Add indices for faster lookups
                        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_source ON {} ({}_id);\n", 
                            relation_table, relation_table, self.class_name.to_lowercase()));
                            
                        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_target ON {} ({}_id);\n", 
                            relation_table, relation_table, target_class.to_lowercase()));
                    }
                }
            }
        }
        
        sql
    }
} 