use crate::macros::str_enum;

str_enum! {
    /// A [locale](https://docs.discord.com/developers/reference#locales): a language Discord's
    /// client is available in.
    ///
    /// Used for user and guild language settings and for localized command names and
    /// descriptions. Variant names follow the language names in the Discord docs.
    ///
    /// discord.js counterpart: `Locale`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::util::Locale;
    ///
    /// let locale: Locale = serde_json::from_str(r#""pl""#).unwrap();
    /// assert_eq!(locale, Locale::Polish);
    /// assert_eq!(Locale::EnglishUs.as_str(), "en-US");
    /// // Locales added by Discord later are kept as they are.
    /// assert!(Locale::from("xx-YY").is_unknown());
    /// ```
    pub enum Locale {
        /// Indonesian (Bahasa Indonesia).
        Indonesian = "id",
        /// Danish (Dansk).
        Danish = "da",
        /// German (Deutsch).
        German = "de",
        /// English, United Kingdom.
        EnglishUk = "en-GB",
        /// English, United States.
        EnglishUs = "en-US",
        /// Spanish, Spain (Español).
        Spanish = "es-ES",
        /// Spanish, Latin America (Español, LATAM).
        SpanishLatam = "es-419",
        /// French (Français).
        French = "fr",
        /// Croatian (Hrvatski).
        Croatian = "hr",
        /// Italian (Italiano).
        Italian = "it",
        /// Lithuanian (Lietuviškai).
        Lithuanian = "lt",
        /// Hungarian (Magyar).
        Hungarian = "hu",
        /// Dutch (Nederlands).
        Dutch = "nl",
        /// Norwegian (Norsk).
        Norwegian = "no",
        /// Polish (Polski).
        Polish = "pl",
        /// Portuguese, Brazil (Português do Brasil).
        PortugueseBrazilian = "pt-BR",
        /// Romanian (Română).
        Romanian = "ro",
        /// Finnish (Suomi).
        Finnish = "fi",
        /// Swedish (Svenska).
        Swedish = "sv-SE",
        /// Vietnamese (Tiếng Việt).
        Vietnamese = "vi",
        /// Turkish (Türkçe).
        Turkish = "tr",
        /// Czech (Čeština).
        Czech = "cs",
        /// Greek (Ελληνικά).
        Greek = "el",
        /// Bulgarian (български).
        Bulgarian = "bg",
        /// Russian (Pусский).
        Russian = "ru",
        /// Ukrainian (Українська).
        Ukrainian = "uk",
        /// Hindi (हिन्दी).
        Hindi = "hi",
        /// Thai (ไทย).
        Thai = "th",
        /// Chinese, China (中文).
        ChineseChina = "zh-CN",
        /// Japanese (日本語).
        Japanese = "ja",
        /// Chinese, Taiwan (繁體中文).
        ChineseTaiwan = "zh-TW",
        /// Korean (한국어).
        Korean = "ko",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_unknown() {
        assert_eq!(
            serde_json::to_string(&Locale::SpanishLatam).unwrap(),
            r#""es-419""#
        );
        let unknown: Locale = serde_json::from_str(r#""tlh""#).unwrap();
        assert_eq!(unknown, Locale::Unknown("tlh".into()));
        assert_eq!(serde_json::to_string(&unknown).unwrap(), r#""tlh""#);
        // Equality is by value, also against `Unknown` holding a known string.
        assert_eq!(Locale::Unknown("pl".into()), Locale::Polish);
    }
}
