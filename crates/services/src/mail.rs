#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use lettre::message::{header::ContentType, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use r_data_core_core::config::SmtpConfig;
use r_data_core_core::error::{Error, Result};

pub struct MailService {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from_mailbox: Mailbox,
    template_engine: handlebars::Handlebars<'static>,
}

impl MailService {
    /// Create a new `MailService` from SMTP configuration.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] if the from address is invalid or TLS relay setup fails.
    pub fn new(config: &SmtpConfig) -> Result<Self> {
        let from_mailbox: Mailbox = config
            .from_name
            .as_ref()
            .map_or_else(
                || config.from_address.clone(),
                |name| format!("{name} <{}>", config.from_address),
            )
            .parse()
            .map_err(|e| Error::Config(format!("Invalid from address: {e}")))?;

        let mut builder = if config.tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                .map_err(|e| Error::Config(format!("SMTP TLS relay error: {e}")))?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
        };

        builder = builder.port(config.port);

        if let (Some(ref user), Some(ref pass)) = (&config.username, &config.password) {
            builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
        }

        let transport = builder.build();
        let template_engine = handlebars::Handlebars::new();

        Ok(Self {
            transport,
            from_mailbox,
            template_engine,
        })
    }

    /// Render a Handlebars template string with the given JSON context.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] if template rendering fails.
    pub fn render_template(&self, template: &str, context: &serde_json::Value) -> Result<String> {
        self.template_engine
            .render_template(template, context)
            .map_err(|e| Error::Config(format!("Template rendering failed: {e}")))
    }

    /// Send an email with Handlebars templates for subject and body.
    ///
    /// `from_name_override` replaces the default `from_name` if provided.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] if template rendering or SMTP send fails,
    /// or [`Error::Validation`] if recipients are invalid or absent.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_email(
        &self,
        to: &[String],
        cc: &[String],
        subject_template: &str,
        body_text_template: &str,
        body_html_template: Option<&str>,
        context: &serde_json::Value,
        from_name_override: Option<&str>,
    ) -> Result<()> {
        let subject = self.render_template(subject_template, context)?;
        let body_text = self.render_template(body_text_template, context)?;
        let body_html = body_html_template
            .map(|t| self.render_template(t, context))
            .transpose()?;

        let from = from_name_override.map_or_else(
            || self.from_mailbox.clone(),
            |name| {
                format!("{name} <{}>", self.from_mailbox.email)
                    .parse()
                    .unwrap_or_else(|_| self.from_mailbox.clone())
            },
        );

        self.send_raw_email_with_from(&from, to, cc, &subject, &body_text, body_html.as_deref())
            .await
    }

    /// Send a pre-rendered email (no template rendering).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] if SMTP send fails,
    /// or [`Error::Validation`] if recipients are invalid or absent.
    pub async fn send_raw_email(
        &self,
        to: &[String],
        subject: &str,
        body_text: &str,
        body_html: Option<&str>,
    ) -> Result<()> {
        self.send_raw_email_with_from(&self.from_mailbox, to, &[], subject, body_text, body_html)
            .await
    }

    async fn send_raw_email_with_from(
        &self,
        from: &Mailbox,
        to: &[String],
        cc: &[String],
        subject: &str,
        body_text: &str,
        body_html: Option<&str>,
    ) -> Result<()> {
        if to.is_empty() {
            return Err(Error::Validation("No recipients specified".to_string()));
        }

        let mut builder = Message::builder().from(from.clone()).subject(subject);

        for addr in to {
            let mailbox: Mailbox = addr
                .parse()
                .map_err(|e| Error::Validation(format!("Invalid recipient '{addr}': {e}")))?;
            builder = builder.to(mailbox);
        }

        for addr in cc {
            let mailbox: Mailbox = addr
                .parse()
                .map_err(|e| Error::Validation(format!("Invalid CC recipient '{addr}': {e}")))?;
            builder = builder.cc(mailbox);
        }

        let message = if let Some(html) = body_html {
            builder
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .content_type(ContentType::TEXT_PLAIN)
                                .body(body_text.to_string()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .content_type(ContentType::TEXT_HTML)
                                .body(html.to_string()),
                        ),
                )
                .map_err(|e| Error::Config(format!("Failed to build email: {e}")))?
        } else {
            builder
                .body(body_text.to_string())
                .map_err(|e| Error::Config(format!("Failed to build email: {e}")))?
        };

        self.transport
            .send(message)
            .await
            .map_err(|e| Error::Config(format!("Failed to send email: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn render_simple_template() {
        let hbs = handlebars::Handlebars::new();
        let ctx = serde_json::json!({"name": "Alice", "order_id": "12345"});
        let result = hbs
            .render_template("Hello {{name}}, order {{order_id}}", &ctx)
            .unwrap();
        assert_eq!(result, "Hello Alice, order 12345");
    }

    #[test]
    fn render_conditional_template() {
        let hbs = handlebars::Handlebars::new();
        let ctx = serde_json::json!({"name": "Bob", "errors": [{"row": 1, "message": "bad"}]});
        let tmpl =
            "Hi {{name}}{{#if errors}}, errors: {{#each errors}}row {{this.row}}{{/each}}{{/if}}";
        let result = hbs.render_template(tmpl, &ctx).unwrap();
        assert_eq!(result, "Hi Bob, errors: row 1");
    }

    #[test]
    fn render_missing_variable_is_empty() {
        let hbs = handlebars::Handlebars::new();
        let ctx = serde_json::json!({"name": "Carol"});
        let result = hbs
            .render_template("Hello {{name}}, id: {{id}}", &ctx)
            .unwrap();
        assert_eq!(result, "Hello Carol, id: ");
    }

    #[test]
    fn render_nested_variable() {
        let hbs = handlebars::Handlebars::new();
        let ctx = serde_json::json!({"user": {"name": "Dave"}});
        let result = hbs.render_template("Hello {{user.name}}", &ctx).unwrap();
        assert_eq!(result, "Hello Dave");
    }

    #[test]
    fn render_each_loop() {
        let hbs = handlebars::Handlebars::new();
        let ctx = serde_json::json!({"items": ["apple", "banana", "cherry"]});
        let result = hbs
            .render_template("Items: {{#each items}}{{this}}, {{/each}}", &ctx)
            .unwrap();
        assert_eq!(result, "Items: apple, banana, cherry, ");
    }

    #[test]
    fn render_if_else() {
        let hbs = handlebars::Handlebars::new();
        let ctx_true = serde_json::json!({"active": true});
        let ctx_false = serde_json::json!({"active": false});
        let tmpl = "{{#if active}}yes{{else}}no{{/if}}";
        assert_eq!(hbs.render_template(tmpl, &ctx_true).unwrap(), "yes");
        assert_eq!(hbs.render_template(tmpl, &ctx_false).unwrap(), "no");
    }

    #[test]
    fn render_html_in_template() {
        let hbs = handlebars::Handlebars::new();
        let ctx = serde_json::json!({"url": "https://example.com", "name": "Alice"});
        let tmpl = "<a href=\"{{url}}\">Hello {{name}}</a>";
        let result = hbs.render_template(tmpl, &ctx).unwrap();
        assert!(result.contains("https://example.com"));
        assert!(result.contains("Alice"));
    }
}
