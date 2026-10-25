# Message Templates

Under **Config → Message templates** you can change the texts Rauthy sends, without a restart and without touching
the config file.

| Template | What it is |
|----------|------------|
| New account: set password | email with the first-password link |
| Password reset | email with the reset link |
| Email registered already | email to an address that has an account |
| Email one-time password | email with the 2FA code |
| Verification code message | plain text with the code, for SMS / WhatsApp / any channel and for email |

The *look* of an email (its HTML skeleton) is built into Rauthy. You change the wording: subject, header, texts, button
texts and footer, per language. The verification message is one text for all languages, with optional texts for single
languages.

## Placeholders

Texts can contain `{{app_name}}`. The verification message additionally has `{{code}}` (required), `{{name}}` and
`{{minutes}}`.

## Check Before Saving

- **Preview** renders your unsaved draft with sample values.
- **Send a test to me** sends the draft to you: emails go to your own address, the verification message goes through
  the channel you pick to the value that channel is tied to, and fails if you have none (no phone number, for example).
- **Save** checks the draft again. A text with an error (an unknown placeholder, a missing `{{code}}`, an empty
  required text, a line break in a one-line text) is refused and the template in use keeps working. SMS texts longer than
  one segment give a warning.

## Performance

The templates are kept in memory. Saving swaps only the template part of the memory, so sending a message never waits
for the database and never sees a half-saved template. Other nodes of a cluster pick up a change within about 15
seconds. **Back to built-in** removes your text and returns to the config file / built-in wording.

Settings in `[[templates]]` of the config file stay the starting point; a saved template overrides them.
