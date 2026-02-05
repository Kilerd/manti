-- Store API keys directly instead of hashing

-- Rename key_hash to key and drop prefix
ALTER TABLE api_keys RENAME COLUMN key_hash TO key;
ALTER TABLE api_keys DROP COLUMN prefix;

-- Update indexes
DROP INDEX IF EXISTS idx_api_keys_key_hash;
DROP INDEX IF EXISTS idx_api_keys_prefix;
CREATE INDEX IF NOT EXISTS idx_api_keys_key ON api_keys(key);
