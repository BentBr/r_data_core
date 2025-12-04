-- Set super_admin flag to true for the default admin user
-- This migration updates the default admin user (username='admin') to have super_admin privileges

DO $$
BEGIN
    UPDATE admin_users
    SET super_admin = TRUE
    WHERE username = 'admin' AND super_admin = FALSE;

    IF FOUND THEN
        RAISE NOTICE 'Default admin user updated to super_admin';
    ELSE
        RAISE NOTICE 'Default admin user not found or already has super_admin flag';
    END IF;
END
$$;

