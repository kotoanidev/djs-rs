//! Links to channels, messages and the App Directory, and e-mail and phone links.

use core::fmt::{self, Write as _};

use djs_model::id::{ApplicationId, ChannelId, GuildId, MessageId, SkuId};

/// Where Discord's web links point.
const CHANNELS: &str = "https://discord.com/channels";

/// The part of a channel or message link naming the server, or `@me` for a DM.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Server(Option<GuildId>);

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(guild) => fmt::Display::fmt(&guild, f),
            None => f.write_str("@me"),
        }
    }
}

/// A link to a channel: `https://discord.com/channels/GUILD_ID/CHANNEL_ID`, or `@me` in place
/// of the server for a DM. Returned by [`channel_link`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct ChannelLink {
    channel: ChannelId,
    guild: Server,
}

/// Links to the channel `channel`, a DM unless the [`guild`](ChannelLink::guild) is set
/// (D-213).
///
/// Unlike a [`channel_mention`](crate::channel_mention), a link works outside Discord too.
///
/// discord.js counterpart: `channelLink`.
///
/// # Examples
///
/// ```
/// use djs_formatters::channel_link;
/// use djs_model::id::{ChannelId, GuildId};
///
/// let channel = ChannelId::new(222_197_033_908_436_994);
/// assert_eq!(channel_link(channel).to_string(), "https://discord.com/channels/@me/222197033908436994");
/// assert_eq!(
///     channel_link(channel).guild(GuildId::new(81_384_788_765_712_384)).to_string(),
///     "https://discord.com/channels/81384788765712384/222197033908436994"
/// );
/// ```
pub const fn channel_link(channel: ChannelId) -> ChannelLink {
    ChannelLink {
        channel,
        guild: Server(None),
    }
}

impl ChannelLink {
    /// Sets the server the channel is in.
    pub const fn guild(mut self, guild: GuildId) -> Self {
        self.guild = Server(Some(guild));
        self
    }
}

impl fmt::Display for ChannelLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{CHANNELS}/{}/{}", self.guild, self.channel)
    }
}

/// A link to a message: `https://discord.com/channels/GUILD_ID/CHANNEL_ID/MESSAGE_ID`.
/// Returned by [`message_link`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct MessageLink {
    channel: ChannelLink,
    message: MessageId,
}

/// Links to the message `message` of the channel `channel`, a DM unless the
/// [`guild`](MessageLink::guild) is set (D-213).
///
/// discord.js counterpart: `messageLink`.
///
/// # Examples
///
/// ```
/// use djs_formatters::message_link;
/// use djs_model::id::{ChannelId, GuildId, MessageId};
///
/// let link = message_link(ChannelId::new(2), MessageId::new(3)).guild(GuildId::new(1));
/// assert_eq!(link.to_string(), "https://discord.com/channels/1/2/3");
/// ```
pub const fn message_link(channel: ChannelId, message: MessageId) -> MessageLink {
    MessageLink {
        channel: channel_link(channel),
        message,
    }
}

impl MessageLink {
    /// Sets the server the message's channel is in.
    pub const fn guild(mut self, guild: GuildId) -> Self {
        self.channel = self.channel.guild(guild);
        self
    }
}

impl fmt::Display for MessageLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.channel, self.message)
    }
}

/// A link to an app's store page in the App Directory, or to one of its SKUs. Returned by
/// [`application_directory`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct ApplicationDirectory {
    application: ApplicationId,
    sku: Option<SkuId>,
}

/// Links to the App Directory store page of the app `application`
/// (`https://discord.com/application-directory/ID/store`), or of one of its SKUs with
/// [`sku`](ApplicationDirectory::sku) (D-213). Discord shows the link as a card.
///
/// discord.js counterpart: `applicationDirectory`.
///
/// # Examples
///
/// ```
/// use djs_formatters::application_directory;
/// use djs_model::id::{ApplicationId, SkuId};
///
/// let app = ApplicationId::new(1);
/// assert_eq!(application_directory(app).to_string(), "https://discord.com/application-directory/1/store");
/// assert_eq!(
///     application_directory(app).sku(SkuId::new(2)).to_string(),
///     "https://discord.com/application-directory/1/store/2"
/// );
/// ```
pub const fn application_directory(application: ApplicationId) -> ApplicationDirectory {
    ApplicationDirectory {
        application,
        sku: None,
    }
}

impl ApplicationDirectory {
    /// Links to the SKU `sku` of the app's store.
    pub const fn sku(mut self, sku: SkuId) -> Self {
        self.sku = Some(sku);
        self
    }
}

impl fmt::Display for ApplicationDirectory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "https://discord.com/application-directory/{}/store",
            self.application
        )?;
        if let Some(sku) = self.sku {
            write!(f, "/{sku}")?;
        }
        Ok(())
    }
}

/// Writes `text` as `application/x-www-form-urlencoded`, like a URL query string: letters,
/// digits and `*-._` as they are, spaces as `+`, every other byte as `%XX`.
fn write_form_encoded(f: &mut fmt::Formatter<'_>, text: &str, lower_case: bool) -> fmt::Result {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &byte in text.as_bytes() {
        match byte {
            b'A'..=b'Z' if lower_case => f.write_char(char::from(byte.to_ascii_lowercase()))?,
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                f.write_char(char::from(byte))?;
            }
            b' ' => f.write_char('+')?,
            _ => {
                f.write_char('%')?;
                f.write_char(char::from(HEX[usize::from(byte >> 4)]))?;
                f.write_char(char::from(HEX[usize::from(byte & 0x0F)]))?;
            }
        }
    }
    Ok(())
}

/// An e-mail link: `<address>` or `<address?header=value&…>`. Returned by [`email`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct Email<'a> {
    address: &'a str,
    headers: &'a [(&'a str, &'a str)],
}

/// Links to the e-mail address `address`; clicking it opens a new e-mail, with the
/// [`headers`](Email::headers) (such as `subject`) filled in if set (D-213).
///
/// discord.js counterpart: `email`.
///
/// # Examples
///
/// ```
/// use djs_formatters::email;
///
/// assert_eq!(email("help@example.com").to_string(), "<help@example.com>");
/// assert_eq!(
///     email("help@example.com").headers(&[("Subject", "Bug report"), ("cc", "a@b.c")]).to_string(),
///     "<help@example.com?subject=Bug+report&cc=a%40b.c>"
/// );
/// ```
pub const fn email(address: &str) -> Email<'_> {
    Email {
        address,
        headers: &[],
    }
}

impl<'a> Email<'a> {
    /// Sets headers of the new e-mail as `(name, value)` pairs, such as `("subject", "Hi")`
    /// or `("body", "…")`. Names are lower-cased, and names and values are encoded as in a
    /// URL query.
    pub const fn headers(mut self, headers: &'a [(&'a str, &'a str)]) -> Self {
        self.headers = headers;
        self
    }
}

impl fmt::Display for Email<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<")?;
        f.write_str(self.address)?;
        for (index, (name, value)) in self.headers.iter().enumerate() {
            f.write_str(if index == 0 { "?" } else { "&" })?;
            write_form_encoded(f, name, true)?;
            f.write_str("=")?;
            write_form_encoded(f, value, false)?;
        }
        f.write_str(">")
    }
}

/// A phone link: `<+number>`. Returned by [`phone_number`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct PhoneNumber<'a>(&'a str);

/// Links to the phone number `number`, which must be international (start with `+`); `None`
/// otherwise (discord.js throws an error).
///
/// discord.js counterpart: `phoneNumber`.
///
/// # Examples
///
/// ```
/// use djs_formatters::phone_number;
///
/// assert_eq!(phone_number("+1 555 0100").unwrap().to_string(), "<+1 555 0100>");
/// assert!(phone_number("555 0100").is_none());
/// ```
#[must_use]
pub fn phone_number(number: &str) -> Option<PhoneNumber<'_>> {
    number.starts_with('+').then_some(PhoneNumber(number))
}

impl fmt::Display for PhoneNumber<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}
