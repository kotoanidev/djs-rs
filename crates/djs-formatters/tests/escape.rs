//! Markdown escaping (D-212): the same results as discord.js for ordinary text, links and
//! tokens left intact, and the documented differences.

use std::borrow::Cow;

use djs_formatters::{
    EscapeOptions, Face, escape_bold, escape_bulleted_list, escape_code_block, escape_escape,
    escape_heading, escape_inline_code, escape_italic, escape_markdown, escape_masked_link,
    escape_numbered_list, escape_spoiler, escape_strikethrough, escape_underline,
};

#[test]
fn text_without_markdown_is_borrowed() {
    for text in [
        "",
        "plain text",
        "żółć 🔥 ~ | a<b",
        "1. not a list by default",
        "# nor a heading",
    ] {
        assert!(
            matches!(escape_markdown(text), Cow::Borrowed(_)),
            "{text:?}"
        );
    }
}

#[test]
fn emphasis_is_escaped() {
    let cases = [
        ("**bold**", r"\*\*bold\*\*"),
        ("*italic*", r"\*italic\*"),
        ("_italic_", r"\_italic\_"),
        ("__underline__", r"\_\_underline\_\_"),
        ("***both***", r"\*\*\*both\*\*\*"),
        ("___both___", r"\_\_\_both\_\_\_"),
        ("snake_case_name", r"snake\_case\_name"),
        ("~~strike~~", r"\~\~strike\~\~"),
        ("a ~ b ~~~ c", r"a ~ b \~\~~ c"),
        ("||spoiler||", r"\|\|spoiler\|\|"),
        ("a | b", "a | b"),
        (r"back\slash", r"back\\slash"),
        ("żółć **gęś** 🔥", r"żółć \*\*gęś\*\* 🔥"),
    ];
    for (text, escaped) in cases {
        assert_eq!(escape_markdown(text), escaped, "{text:?}");
    }
}

#[test]
fn code_is_escaped() {
    let cases = [
        ("`code`", r"\`code\`"),
        ("``code``", r"\`\`code\`\`"),
        ("```block```", r"\`\`\`block\`\`\`"),
        // Every backtick of a longer run (discord.js leaves the fourth alone).
        ("````", r"\`\`\`\`"),
    ];
    for (text, escaped) in cases {
        assert_eq!(escape_markdown(text), escaped, "{text:?}");
    }
}

#[test]
fn links_and_tokens_are_left_intact() {
    let intact = [
        "https://example.com/some_path_here?q=*x*&y=__z__",
        "HTTP://EXAMPLE.COM/A_B",
        "see http://a.b/c_d, ok",
        "<@123> <@!123> <#1> <@&2> <@$3>",
        "<:a_b:3> <a:c_d:4>",
        "<t:1618953630:R> <t:5>",
        "<id:linked-roles:6> <id:browse>",
        "</my_cmd:7> </my_cmd sub_x:7> </a b_c d_e:7>",
        "<https://x.y/a_b> <mail_x@example.com> <+1_555>",
    ];
    for text in intact {
        assert_eq!(escape_markdown(text), text, "{text:?}");
    }
    // Text around them is still escaped.
    assert_eq!(
        escape_markdown("**hi** <@1> _x_ https://a.b/_c_ **bye**"),
        r"\*\*hi\*\* <@1> \_x\_ https://a.b/_c_ \*\*bye\*\*"
    );
    // After a custom emoji the line is still escaped (discord.js stops escaping `_` there).
    assert_eq!(escape_markdown("<:a:1> snake_case"), r"<:a:1> snake\_case");
}

#[test]
fn angle_brackets_that_are_not_tokens_are_escaped_inside() {
    let cases = [
        ("<b_c>", r"<b\_c>"),
        ("a < b_c > d", r"a < b\_c > d"),
        ("<a_b <@1>", r"<a\_b <@1>"),
        ("</not a mention>", "</not a mention>"),
        ("</_x_>", r"</\_x\_>"),
        ("<_x_", r"<\_x\_"),
    ];
    for (text, escaped) in cases {
        assert_eq!(escape_markdown(text), escaped, "{text:?}");
    }
}

#[test]
fn code_content_can_be_left_as_written() {
    let skip_blocks = EscapeOptions::new().code_block_content(false);
    assert_eq!(
        skip_blocks.escape("```**raw**``` **x**"),
        r"\`\`\`**raw**\`\`\` \*\*x\*\*"
    );
    assert_eq!(
        skip_blocks.code_block(false).escape("```**raw**``` **x**"),
        r"```**raw**``` \*\*x\*\*"
    );
    // An unclosed block: the rest is escaped.
    assert_eq!(skip_blocks.escape("```**open"), r"\`\`\`\*\*open");
    // Blocks alternate with escaped text.
    assert_eq!(
        skip_blocks.escape("a```_b_```_c_```_d_"),
        r"a\`\`\`_b_\`\`\`\_c\_\`\`\`\_d\_"
    );

    let skip_inline = EscapeOptions::new().inline_code_content(false);
    assert_eq!(
        skip_inline.escape("`**raw**` **x**"),
        r"\`**raw**\` \*\*x\*\*"
    );
    assert_eq!(skip_inline.escape("`**open"), r"\`\*\*open");
    // A double backtick is not a delimiter.
    assert_eq!(
        skip_inline.escape("``_a_`` `_b_`"),
        r"\`\`\_a\_\`\` \`_b_\`"
    );

    // Inline code does not reach into a skipped block.
    let skip_both = skip_blocks.inline_code_content(false);
    assert_eq!(skip_both.escape("`a ```b``` c`"), r"\`a \`\`\`b\`\`\` c\`");
}

#[test]
fn line_starts_are_escaped_on_request() {
    let headings = EscapeOptions::none().heading(true);
    assert_eq!(
        headings.escape("# Title\n## Sub\n#### no\n  ### ok\n- # in a bullet\n#no space"),
        "\\# Title\n\\## Sub\n#### no\n  \\### ok\n- \\# in a bullet\n#no space"
    );
    let bullets = EscapeOptions::none().bulleted_list(true);
    assert_eq!(
        bullets.escape("- a\n* b\n  - c\n-not\nx - y"),
        "\\- a\n\\* b\n  \\- c\n-not\nx - y"
    );
    // A bullet `*` that italics also escapes gets one backslash.
    assert_eq!(
        EscapeOptions::new().bulleted_list(true).escape("* b"),
        r"\* b"
    );
    let numbers = EscapeOptions::none().numbered_list(true);
    assert_eq!(
        numbers.escape("1. a\n10. b\n  3. c\n1.5\nx 1. y"),
        "1\\. a\n10\\. b\n  3\\. c\n1\\.5\nx 1. y"
    );
    // Line starts after skipped code are found again.
    assert_eq!(
        EscapeOptions::none()
            .heading(true)
            .code_block_content(false)
            .escape("```x\n# y```\n# z"),
        "```x\n# y```\n\\# z"
    );
}

#[test]
fn masked_links_are_escaped_on_request() {
    let links = EscapeOptions::none().masked_link(true);
    let cases = [
        ("[a](https://b.c)", r"\[a](https://b.c)"),
        // Every link of the line (discord.js only escapes the first).
        ("[a](b) and [c](d)", r"\[a](b) and \[c](d)"),
        ("[]()", "[]()"),
        ("[a]()", "[a]()"),
        ("[](b)", "[](b)"),
        ("[a]\n(b)", "[a]\n(b)"),
        ("[x [a](b)", r"\[x \[a](b)"),
    ];
    for (text, escaped) in cases {
        assert_eq!(links.escape(text), escaped, "{text:?}");
    }
}

#[test]
fn single_kinds_can_be_escaped() {
    assert_eq!(escape_bold("**a** *b*"), r"\*\*a\*\* *b*");
    assert_eq!(escape_bold("***a***"), r"\*\**a\*\**");
    assert_eq!(escape_italic("*a* **b** ***c***"), r"\*a\* **b** \***c\***");
    assert_eq!(escape_underline("__a__ _b_"), r"\_\_a\_\_ _b_");
    assert_eq!(escape_strikethrough("~~a~~"), r"\~\~a\~\~");
    assert_eq!(escape_spoiler("||a||"), r"\|\|a\|\|");
    assert_eq!(escape_inline_code("`a` ```b```"), r"\`a\` ```b```");
    assert_eq!(escape_code_block("`a` ```b```"), r"`a` \`\`\`b\`\`\`");
    assert_eq!(escape_escape(r"\*"), r"\\*");
    assert_eq!(escape_heading("# a"), r"\# a");
    assert_eq!(escape_bulleted_list("- a"), r"\- a");
    assert_eq!(escape_numbered_list("2. a"), r"2\. a");
    assert_eq!(escape_masked_link("[a](b)"), r"\[a](b)");
}

#[test]
fn everything_on_at_once() {
    let all = EscapeOptions::new()
        .heading(true)
        .bulleted_list(true)
        .numbered_list(true)
        .masked_link(true);
    assert_eq!(
        all.escape("# Rules\n- be _nice_\n1. no **spam** [here](https://x.y)\n> ok"),
        "\\# Rules\n\\- be \\_nice\\_\n1\\. no \\*\\*spam\\*\\* \\[here](https://x.y)\n> ok"
    );
    // The shrug face: its backslash doubled and both underscores escaped.
    assert_eq!(escape_markdown(Face::Shrug.as_str()), r"¯\\\_(ツ)\_/¯");
}

#[test]
fn nothing_escaped_with_no_options() {
    let none = EscapeOptions::none();
    let text = "**a** _b_ `c` ~~d~~ ||e|| \\ # f\n- g\n1. h [i](j)";
    assert!(matches!(none.escape(text), Cow::Borrowed(_)));
}
