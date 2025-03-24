-- First, let's create ENUM types for subscription and payment method
CREATE TYPE subscription_type AS ENUM ('FREE', 'PRO');
CREATE TYPE payment_method_type AS ENUM ('CREDIT_CARD', 'PAYPAL');

-- Now, create the user_profiles table
CREATE TABLE IF NOT EXISTS user_profiles (
                                             profile_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    country VARCHAR(100),
    subscription_rule subscription_type NOT NULL DEFAULT 'FREE',
    payment_method payment_method_type,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

                             -- Create a foreign key constraint to reference the users table
                             CONSTRAINT fk_user
                             FOREIGN KEY (user_id)
    REFERENCES users (uid)
                         ON DELETE CASCADE
    );

-- Create an index on user_id for faster lookups
CREATE INDEX idx_user_profiles_user_id ON user_profiles(user_id);

-- Create a trigger to automatically update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_user_profiles_updated_at
    BEFORE UPDATE ON user_profiles
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();