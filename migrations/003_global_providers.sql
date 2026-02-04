-- Migration: Global Providers with User Group Access Control
-- This migration transforms provider_configs from user-owned to global resources

-- 1. Add user_groups to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS user_groups TEXT[] DEFAULT ARRAY['default']::TEXT[];
CREATE INDEX IF NOT EXISTS idx_users_groups ON users USING GIN(user_groups);

-- 2. Transform provider_configs table
-- 2a. Handle name conflicts by adding user_id prefix (only if user_id column exists)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns
               WHERE table_name = 'provider_configs' AND column_name = 'user_id') THEN
        UPDATE provider_configs SET name = name || '_' || LEFT(user_id::text, 8)
        WHERE user_id IS NOT NULL;
    END IF;
END $$;

-- 2b. Drop user_id related constraints and column
ALTER TABLE provider_configs DROP CONSTRAINT IF EXISTS provider_configs_user_id_fkey;
ALTER TABLE provider_configs DROP CONSTRAINT IF EXISTS provider_configs_user_id_name_key;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns
               WHERE table_name = 'provider_configs' AND column_name = 'user_id') THEN
        ALTER TABLE provider_configs DROP COLUMN user_id;
    END IF;
END $$;

-- 2c. Add allowed_groups column and new unique constraint
ALTER TABLE provider_configs ADD COLUMN IF NOT EXISTS allowed_groups TEXT[] DEFAULT ARRAY[]::TEXT[];
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'provider_configs_name_key') THEN
        ALTER TABLE provider_configs ADD CONSTRAINT provider_configs_name_key UNIQUE(name);
    END IF;
EXCEPTION WHEN duplicate_object THEN
    NULL;
END $$;
CREATE INDEX IF NOT EXISTS idx_provider_configs_groups ON provider_configs USING GIN(allowed_groups);

-- 3. Create models table for explicit model management
CREATE TABLE IF NOT EXISTS models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_config_id UUID NOT NULL REFERENCES provider_configs(id) ON DELETE CASCADE,
    model_id VARCHAR(100) NOT NULL,
    display_name VARCHAR(200),
    input_cost_per_1k DECIMAL(10, 6),
    output_cost_per_1k DECIMAL(10, 6),
    max_context INTEGER,
    supports_tools BOOLEAN DEFAULT false,
    supports_vision BOOLEAN DEFAULT false,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider_config_id, model_id)
);

CREATE INDEX IF NOT EXISTS idx_models_provider ON models(provider_config_id);
CREATE INDEX IF NOT EXISTS idx_models_model_id ON models(model_id);
CREATE INDEX IF NOT EXISTS idx_models_active ON models(is_active);

-- Add updated_at trigger for models table
DROP TRIGGER IF EXISTS update_models_updated_at ON models;
CREATE TRIGGER update_models_updated_at BEFORE UPDATE ON models
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
