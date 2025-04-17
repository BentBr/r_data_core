-- Create a function to automatically update the updated_at column
CREATE OR REPLACE FUNCTION update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
   NEW.updated_at = NOW();
   RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create triggers for each table
DROP TRIGGER IF EXISTS set_timestamp ON entity_registry;
CREATE TRIGGER set_timestamp
BEFORE UPDATE ON entity_registry
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

DROP TRIGGER IF EXISTS set_timestamp ON class_definitions;
CREATE TRIGGER set_timestamp
BEFORE UPDATE ON class_definitions
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

DROP TRIGGER IF EXISTS set_timestamp ON entities;
CREATE TRIGGER set_timestamp
BEFORE UPDATE ON entities
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

DROP TRIGGER IF EXISTS set_timestamp ON admin_users;
CREATE TRIGGER set_timestamp
BEFORE UPDATE ON admin_users
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

DROP TRIGGER IF EXISTS set_timestamp ON permission_schemes;
CREATE TRIGGER set_timestamp
BEFORE UPDATE ON permission_schemes
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

DROP TRIGGER IF EXISTS set_timestamp ON api_keys;
CREATE TRIGGER set_timestamp
BEFORE UPDATE ON api_keys
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

DROP TRIGGER IF EXISTS set_timestamp ON notifications;
CREATE TRIGGER set_timestamp
BEFORE UPDATE ON notifications
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

DROP TRIGGER IF EXISTS set_timestamp ON workflow_definitions;
CREATE TRIGGER set_timestamp
BEFORE UPDATE ON workflow_definitions
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

DROP TRIGGER IF EXISTS set_timestamp ON workflows;
CREATE TRIGGER set_timestamp
BEFORE UPDATE ON workflows
FOR EACH ROW
EXECUTE FUNCTION update_timestamp(); 