-- Create default admin user with username 'admin' and password 'admin'
-- Note: In production, you should use a secure password and change it immediately after setup

-- Check if admin user already exists
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM admin_users WHERE username = 'admin') THEN
        -- Insert admin user with hashed password
        -- Password 'admin' is hashed using secure Argon2id parameters (m=19456,t=2,p=1)
        INSERT INTO admin_users (
            uuid, 
            path,
            username, 
            email, 
            password_hash, 
            first_name,
            last_name,
            is_active,
            created_at,
            updated_at,
            published,
            version,
            created_by
        ) VALUES (
            uuid_generate_v7(),
            '/users', 
            'admin', 
            'admin@example.com',
            -- 'adminadmin' password with secure Argon2id parameters
            '$argon2id$v=19$m=19456,t=2,p=1$AyU4SymrYGzpmYfqDSbugg$AhzMvJ1bOxrv2WQ1ks3PRFXGezp966kjJwkoUdJbFY4',
            'System',
            'Administrator',
            TRUE,
            NOW(),
            NOW(),
            TRUE,
            1,
            '00000000-0000-0000-0000-000000000000'
        );
        
        RAISE NOTICE 'Default admin user created';
    ELSE
        RAISE NOTICE 'Admin user already exists, skipping creation';
    END IF;
END
$$; 