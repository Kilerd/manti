-- Store provider API keys directly instead of encrypting
-- Note: Existing encrypted keys will become invalid, providers need to be re-created
ALTER TABLE provider_configs RENAME COLUMN api_key_encrypted TO api_key;
