//! The formatters: output of every kind, nesting, and the optional parts (D-213).

use std::{
    fmt::Write as _,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use djs_formatters::{
    Face, GuildNavigation, HeadingLevel, ListItem, TimestampStyle, application_directory,
    block_quote, bold, channel_link, channel_mention, code_block, email, emoji, escape_markdown,
    heading, hide_link_embed, hyperlink, inline_code, italic, linked_role_mention, message_link,
    ordered_list, phone_number, quote, role_mention, slash_command_mention, spoiler, strikethrough,
    subtext, time, underline, unordered_list, user_mention,
};
use djs_model::{
    id::{ApplicationId, ChannelId, CommandId, EmojiId, GuildId, MessageId, RoleId, SkuId, UserId},
    util::Timestamp,
};

#[test]
fn mentions() {
    assert_eq!(user_mention(UserId::new(1)).to_string(), "<@1>");
    assert_eq!(channel_mention(ChannelId::new(2)).to_string(), "<#2>");
    assert_eq!(role_mention(RoleId::new(3)).to_string(), "<@&3>");
    assert_eq!(
        linked_role_mention(RoleId::new(4)).to_string(),
        "<id:linked-roles:4>"
    );
    let id = CommandId::new(5);
    assert_eq!(slash_command_mention("ping", id).to_string(), "</ping:5>");
    assert_eq!(
        slash_command_mention("tag", id)
            .subcommand("show")
            .to_string(),
        "</tag show:5>"
    );
    assert_eq!(
        slash_command_mention("admin", id)
            .group("users", "ban")
            .to_string(),
        "</admin users ban:5>"
    );
    // The last call wins.
    assert_eq!(
        slash_command_mention("admin", id)
            .group("users", "ban")
            .subcommand("info")
            .to_string(),
        "</admin info:5>"
    );
    let custom = EmojiId::new(u64::MAX);
    assert_eq!(emoji(custom).to_string(), "<:emoji:18446744073709551615>");
    assert_eq!(
        emoji(custom).name("pepe").to_string(),
        "<:pepe:18446744073709551615>"
    );
    assert_eq!(
        emoji(custom).animated(true).to_string(),
        "<a:emoji:18446744073709551615>"
    );
    assert_eq!(GuildNavigation::Browse.to_string(), "<id:browse>");
    assert_eq!(GuildNavigation::Guide.to_string(), "<id:guide>");
    assert_eq!(GuildNavigation::LinkedRoles.as_str(), "<id:linked-roles>");
}

#[test]
fn markdown_nests_without_escaping() {
    assert_eq!(bold(italic(underline("x"))).to_string(), "**___x___**");
    assert_eq!(strikethrough(spoiler(42)).to_string(), "~~||42||~~");
    assert_eq!(
        inline_code(user_mention(UserId::new(1))).to_string(),
        "`<@1>`"
    );
    assert_eq!(quote("a").to_string(), "> a");
    assert_eq!(block_quote("a\nb").to_string(), ">>> a\nb");
    assert_eq!(subtext("small").to_string(), "-# small");
    assert_eq!(heading("H").to_string(), "# H");
    assert_eq!(heading("H").level(HeadingLevel::Two).to_string(), "## H");
    assert_eq!(heading("H").level(HeadingLevel::Three).to_string(), "### H");
    // Content from users is escaped first.
    let name = "*_sneaky_*";
    assert_eq!(
        bold(escape_markdown(name)).to_string(),
        r"**\*\_sneaky\_\***"
    );
}

#[test]
fn code_blocks() {
    assert_eq!(code_block("x").to_string(), "```\nx\n```");
    assert_eq!(
        code_block("x").language("rust").to_string(),
        "```rust\nx\n```"
    );
    assert_eq!(code_block("").to_string(), "```\n\n```");
}

#[test]
fn links() {
    assert_eq!(hyperlink("a", "https://b").to_string(), "[a](https://b)");
    assert_eq!(
        hyperlink("a", "https://b").title("t").to_string(),
        "[a](https://b \"t\")"
    );
    assert_eq!(
        hyperlink("a", hide_link_embed("https://b")).to_string(),
        "[a](<https://b>)"
    );
    let (guild, channel, message) = (GuildId::new(1), ChannelId::new(2), MessageId::new(3));
    assert_eq!(
        channel_link(channel).to_string(),
        "https://discord.com/channels/@me/2"
    );
    assert_eq!(
        channel_link(channel).guild(guild).to_string(),
        "https://discord.com/channels/1/2"
    );
    assert_eq!(
        message_link(channel, message).to_string(),
        "https://discord.com/channels/@me/2/3"
    );
    assert_eq!(
        message_link(channel, message).guild(guild).to_string(),
        "https://discord.com/channels/1/2/3"
    );
    assert_eq!(
        application_directory(ApplicationId::new(4))
            .sku(SkuId::new(5))
            .to_string(),
        "https://discord.com/application-directory/4/store/5"
    );
}

#[test]
fn email_headers_are_encoded_like_a_query() {
    assert_eq!(email("a@b.c").to_string(), "<a@b.c>");
    assert_eq!(
        email("a@b.c")
            .headers(&[("SUBJECT", "Zażółć & co"), ("body", "*-._~!")])
            .to_string(),
        "<a@b.c?subject=Za%C5%BC%C3%B3%C5%82%C4%87+%26+co&body=*-._%7E%21>"
    );
}

#[test]
fn phone_numbers_must_be_international() {
    assert_eq!(
        phone_number("+48 123").map(|number| number.to_string()),
        Some("<+48 123>".to_owned())
    );
    assert!(phone_number("48 123").is_none());
    assert!(phone_number("").is_none());
}

#[test]
fn timestamps() {
    assert_eq!(time(0).to_string(), "<t:0>");
    assert_eq!(
        time(-5).style(TimestampStyle::ShortTime).to_string(),
        "<t:-5:t>"
    );
    let styles = [
        (TimestampStyle::ShortTime, 't'),
        (TimestampStyle::MediumTime, 'T'),
        (TimestampStyle::ShortDate, 'd'),
        (TimestampStyle::LongDate, 'D'),
        (TimestampStyle::LongDateShortTime, 'f'),
        (TimestampStyle::FullDateShortTime, 'F'),
        (TimestampStyle::ShortDateShortTime, 's'),
        (TimestampStyle::ShortDateMediumTime, 'S'),
        (TimestampStyle::RelativeTime, 'R'),
    ];
    for (style, letter) in styles {
        assert_eq!(style.as_char(), letter);
        assert_eq!(time(1).style(style).to_string(), format!("<t:1:{letter}>"));
    }
    // Fractions are dropped, rounding down — also before 1970.
    let timestamp = Timestamp::from_unix_millis(1_618_953_630_999).unwrap();
    assert_eq!(time(timestamp).seconds(), 1_618_953_630);
    let after = UNIX_EPOCH + Duration::from_millis(1500);
    assert_eq!(time(after).seconds(), 1);
    let before = UNIX_EPOCH - Duration::from_millis(500);
    assert_eq!(time(before).seconds(), -1);
    let before = UNIX_EPOCH - Duration::from_secs(2);
    assert_eq!(time(before).seconds(), -2);
    let now = SystemTime::now();
    // A reference works too.
    let reference = &now;
    #[expect(clippy::needless_borrows_for_generic_args, reason = "what is tested")]
    let borrowed = time(&reference);
    assert_eq!(time(now).seconds(), borrowed.seconds());
}

#[test]
fn lists() {
    assert_eq!(unordered_list::<&str>(&[]).to_string(), "");
    assert_eq!(unordered_list(&["a", "b"]).to_string(), "- a\n- b");
    assert_eq!(ordered_list(&["a", "b"]).to_string(), "1. a\n2. b");
    assert_eq!(
        ordered_list(&["a", "b"]).start(9).to_string(),
        "9. a\n10. b"
    );
    assert_eq!(ordered_list(&["a"]).start(0).to_string(), "1. a");
    let nested = [
        ListItem::Text("a"),
        ListItem::Nested(&[
            ListItem::Text("b"),
            ListItem::Nested(&[ListItem::Text("c")]),
            ListItem::Text("d"),
        ]),
        ListItem::Text("e"),
    ];
    assert_eq!(
        unordered_list(&nested).to_string(),
        "- a\n  - b\n    - c\n  - d\n- e"
    );
    assert_eq!(
        ordered_list(&nested).to_string(),
        "1. a\n  1. b\n    1. c\n  2. d\n2. e"
    );
    // A list that starts nested has no line before it.
    let indented = [ListItem::Nested(&[ListItem::Text("x")])];
    assert_eq!(unordered_list(&indented).to_string(), "  - x");
    // Any Display items, such as other formatters.
    let people = [user_mention(UserId::new(1)), user_mention(UserId::new(2))];
    assert_eq!(ordered_list(&people).to_string(), "1. <@1>\n2. <@2>");
}

#[test]
fn faces() {
    assert_eq!(Face::Shrug.to_string(), r"¯\_(ツ)_/¯");
    assert_eq!(Face::Tableflip.to_string(), "(╯°□°)╯︵ ┻━┻");
    assert_eq!(Face::Unflip.to_string(), "┬─┬ノ( º _ ºノ)");
}

#[test]
fn formatters_write_into_an_existing_buffer() {
    let mut message = String::with_capacity(64);
    write!(
        message,
        "{} {} {}",
        bold("Winner:"),
        user_mention(UserId::new(1)),
        time(0).style(TimestampStyle::RelativeTime)
    )
    .unwrap();
    assert_eq!(message, "**Winner:** <@1> <t:0:R>");
}
