//! Transactional emails share a table-based HTML layout and a plain-text alternative.

use lettre::message::MultiPart;

pub struct Email {
    text: String,
    html: String,
}

impl Email {
    pub fn workspace_deletion(name: &str, code: &str) -> Self {
        let text = format!(
            "Your code to permanently delete the workspace {name} is:\n\n{code}\n\nThis code expires in 10 minutes. Deleting this workspace permanently removes its teams, issues, activity, memberships, and invitations. Your account and other workspaces remain available.\n\nIf you did not request this, do not share or use this code."
        );
        let name = escape(name);
        let code = escape(code);
        let heading = format!("Verify {name} deletion");
        let content = format!(
            r#"<p style="margin: 0 0 24px;">Your code to permanently delete the workspace <strong>{name}</strong> is:</p>
<table role="presentation" cellpadding="0" cellspacing="0" border="0" style="margin: 0 0 16px;"><tr><td class="code" style="padding: 12px 18px; background-color: #ededf1; border-radius: 6px; color: #24242b; font-family: 'Courier New', monospace; font-size: 24px; line-height: 32px; font-weight: 700; letter-spacing: 3px;">{code}</td></tr></table>
<p class="muted" style="margin: 0 0 24px; color: #676773; font-size: 14px; line-height: 22px;">This code expires in 10 minutes.</p>
<p style="margin: 0;">Deleting this workspace permanently removes its teams, issues, activity, memberships, and invitations. Your account and other workspaces remain available.</p>"#
        );
        Self::render(
            text,
            &heading,
            &content,
            "If you did not request this, do not share or use this code.",
        )
    }

    pub fn invitation(url: &str) -> Self {
        let text = format!(
            "Join using this single-use invitation within 7 days:\n{url}\nOnly share this link with the intended teammate."
        );
        Self::render(
            text,
            "Join your workspace",
            &link_content(
                "Join using this single-use invitation within 7 days.",
                "Join workspace",
                url,
            ),
            "Only share this link with the intended teammate.",
        )
    }

    pub fn password_reset(url: &str) -> Self {
        let text = format!(
            "Reset your password within 30 minutes:\n{url}\nIf you did not request this, ignore this email."
        );
        Self::render(
            text,
            "Reset your password",
            &link_content(
                "Reset your password within 30 minutes.",
                "Reset password",
                url,
            ),
            "If you did not request this, ignore this email.",
        )
    }

    fn render(text: String, heading: &str, content: &str, notice: &str) -> Self {
        // Only escaped text and our own markup enter the shared layout.
        let content = format!(
            r#"<h1 class="heading foreground" style="margin: 0 0 24px; color: #24242b; font-size: 28px; line-height: 36px; font-weight: 700; letter-spacing: -0.7px;">{heading}</h1>
{content}
<p class="divider muted" style="margin: 32px 0 0; padding-top: 24px; border-top: 1px solid #e5e5e9; color: #676773; font-size: 14px; line-height: 22px;">{notice}</p>"#
        );
        Self {
            text,
            html: include_str!("email.html").replace("{{content}}", &content),
        }
    }

    pub(crate) fn into_multipart(self) -> MultiPart {
        MultiPart::alternative_plain_html(self.text, self.html)
    }
}

fn link_content(intro: &str, label: &str, url: &str) -> String {
    // URLs are constructed by the application from its validated public origin and token.
    let url = escape(url);
    format!(
        r##"<p style="margin: 0 0 24px;">{intro}</p>
<table role="presentation" cellpadding="0" cellspacing="0" border="0" style="margin: 0 0 24px;"><tr><td bgcolor="#5555ca" style="background-color: #5555ca; border-radius: 6px; text-align: center; mso-padding-alt: 12px 24px;"><a href="{url}" style="display: inline-block; padding: 12px 24px; color: #ffffff; font-size: 16px; line-height: 24px; font-weight: 600; text-decoration: none;">{label}</a></td></tr></table>
<p class="muted" style="margin: 0 0 8px; color: #676773; font-size: 14px; line-height: 22px;">You can also copy this link into your browser:</p>
<p style="margin: 0; font-size: 14px; line-height: 22px; overflow-wrap: anywhere; word-break: break-all;"><a class="link" href="{url}" style="color: #5555ca; text-decoration: underline;">{url}</a></p>"##
    )
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_names_are_text_in_html_and_unchanged_in_plain_text() {
        let name = "R&D <script>alert(\"x\")</script> 'team'";
        let email = Email::workspace_deletion(name, "A1B2C3D4");
        assert!(email.text.contains(name));
        assert!(!email.html.contains("<script>"));
        assert!(
            email.html.contains(
                "R&amp;D &lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &#39;team&#39;"
            )
        );
        for body in [&email.text, &email.html] {
            assert!(body.contains("A1B2C3D4"));
            assert!(body.contains("10 minutes"));
            assert!(body.contains("permanently removes"));
            assert!(body.contains("do not share or use this code"));
        }
    }

    #[test]
    fn action_links_preserve_fragments_and_escape_attributes() {
        let url = "https://convex.example/auth/reset#token&\"test";
        for email in [Email::password_reset(url), Email::invitation(url)] {
            assert!(email.text.contains(url));
            assert!(
                email
                    .html
                    .contains("href=\"https://convex.example/auth/reset#token&amp;&quot;test\"")
            );
            assert!(!email.html.contains("{{content}}"));
        }
    }

    #[test]
    fn every_email_has_plain_text_and_html_mime_alternatives() {
        for email in [
            Email::workspace_deletion("Hobbit Hole", "A1B2C3D4"),
            Email::invitation("https://convex.example/invite#token"),
            Email::password_reset("https://convex.example/auth/reset#token"),
        ] {
            let message = lettre::Message::builder()
                .from("convex@example.com".parse().unwrap())
                .to("member@example.com".parse().unwrap())
                .subject("Convex")
                .multipart(email.into_multipart())
                .unwrap();
            let wire = String::from_utf8(message.formatted()).unwrap();
            assert!(wire.contains("multipart/alternative"));
            assert!(wire.contains("text/plain; charset=utf-8"));
            assert!(wire.contains("text/html; charset=utf-8"));
            assert!(wire.find("text/plain").unwrap() < wire.find("text/html").unwrap());
        }
    }
}
