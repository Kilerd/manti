-- Enable pgcrypto extension for gen_random_uuid()
-- (gen_random_uuid is built-in since PostgreSQL 13, but pgcrypto ensures compatibility)
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Add gen_random_uuid() as default for all UUID primary keys
ALTER TABLE users ALTER COLUMN id SET DEFAULT gen_random_uuid();
ALTER TABLE api_keys ALTER COLUMN id SET DEFAULT gen_random_uuid();
ALTER TABLE provider_configs ALTER COLUMN id SET DEFAULT gen_random_uuid();
ALTER TABLE usage ALTER COLUMN id SET DEFAULT gen_random_uuid();
