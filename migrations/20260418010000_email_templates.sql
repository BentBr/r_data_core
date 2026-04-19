-- Email template types
CREATE TYPE email_template_type AS ENUM ('system', 'workflow');

-- Email templates table
CREATE TABLE email_templates (
    uuid               UUID PRIMARY KEY DEFAULT uuidv7(),
    name               VARCHAR(100) NOT NULL,
    slug               VARCHAR(100) NOT NULL UNIQUE,
    template_type      email_template_type NOT NULL,
    subject_template   TEXT NOT NULL,
    body_html_template TEXT NOT NULL,
    body_text_template TEXT NOT NULL,
    variables          JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by         UUID NOT NULL,
    updated_by         UUID
);

-- Seed the password_reset system template
INSERT INTO email_templates (name, slug, template_type, subject_template, body_html_template, body_text_template, variables, created_by)
VALUES (
    'Password Reset',
    'password_reset',
    'system',
    'Password Reset Request',
    '<html><body><h1>Password Reset</h1><p>Hello {{user_name}},</p><p>You requested a password reset. Click the link below to set a new password:</p><p><a href="{{reset_url}}">Reset Password</a></p><p>This link expires in 1 hour. If you did not request this, please ignore this email.</p></body></html>',
    E'Hello {{user_name}},\n\nYou requested a password reset. Use the link below to set a new password:\n\n{{reset_url}}\n\nThis link expires in 1 hour. If you did not request this, please ignore this email.',
    '[{"key": "reset_url", "description": "Password reset link"}, {"key": "user_name", "description": "User full name (first + last)"}, {"key": "user_email", "description": "User email address"}]'::jsonb,
    '00000000-0000-0000-0000-000000000000'
);
