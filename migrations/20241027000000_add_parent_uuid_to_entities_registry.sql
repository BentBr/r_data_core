-- Add parent_uuid column to entities_registry for parent-child relationships
ALTER TABLE entities_registry 
ADD COLUMN IF NOT EXISTS parent_uuid UUID REFERENCES entities_registry(uuid) ON DELETE SET NULL;

-- Create index on parent_uuid for faster lookups
CREATE INDEX IF NOT EXISTS idx_entities_registry_parent_uuid ON entities_registry(parent_uuid);

