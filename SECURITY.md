# Security Policy

## Supported Versions

Security updates are provided for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| latest  | ✅ |
| < 1.0   | ✅                |

We recommend always running the latest version of RDataCore.

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

If you discover a security vulnerability, please report it privately by emailing:

**security@rdatacore.eu**

Include as much of the following information as possible:

- Type of vulnerability (e.g., SQL injection, authentication bypass, XSS)
- Full paths of affected source files (if known)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if available)
- Potential impact of the vulnerability

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your report within 48 hours.
- **Assessment**: We will investigate and validate the reported vulnerability.
- **Updates**: We will keep you informed of our progress.
- **Resolution**: We aim to address critical vulnerabilities within 30 days.
- **Credit**: We will credit you in release notes (unless you prefer to remain anonymous).

## Security Considerations

RDataCore handles potentially sensitive business data. When deploying:

- **Never expose the admin API** to the public internet without proper access controls
- **Use strong, unique values** for `JWT_SECRET` and other secrets
- **Change the default admin password** immediately after installation
- **Enable HTTPS** in production environments
- **Restrict `CORS_ORIGINS`** to trusted domains only
- **Review API key permissions** and rotate keys periodically
- **Keep dependencies updated** by regularly pulling new releases

## Security Features

- Argon2 password hashing for admin users
- JWT-based authentication with configurable expiration
- Role-based access control for API endpoints
- API key authentication with scoped permissions
- Compile-time SQL query verification (SQLx)

## Responsible Disclosure

We kindly ask that you:

- Give us reasonable time to address the issue before public disclosure
- Avoid accessing or modifying data that does not belong to you
- Act in good faith to avoid privacy violations and service disruption

We appreciate your help in keeping RDataCore and its users secure.
