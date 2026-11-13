//! Unknown-key diagnostics for the four authoring files. Serde supplies each
//! struct's accepted field names (including renames), so optional fields are
//! recognised even when absent or null. Only the nesting is described here.
//! Inspect raw JSON so untagged bool/object settings and invalid events receive
//! the same diagnostics as ordinary structs, even if typed parsing fails.
use crate::config_files::*;
use crate::event::EventConfig;
use crate::luma::LumaEvent;
use serde::de::{DeserializeOwned, Visitor};
use serde::Deserializer;
use serde_json::Value;
use std::path::Path;

struct Fields(&'static [&'static str]);

impl<'de> Deserializer<'de> for &mut Fields {
    type Error = serde::de::value::Error;

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0 = fields;
        Err(serde::de::Error::custom("field inspection only"))
    }

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(serde::de::Error::custom("expected a configuration struct"))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map enum identifier ignored_any
    }
}

fn fields<T: DeserializeOwned>() -> &'static [&'static str] {
    let mut fields = Fields(&[]);
    let _ = T::deserialize(&mut fields);
    assert!(!fields.0.is_empty(), "expected a configuration struct");
    fields.0
}

fn distance(a: &str, b: &str) -> usize {
    let mut row: Vec<_> = (0..=b.chars().count()).collect();
    for (i, a) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, b) in b.chars().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(a != b));
            diagonal = above;
        }
    }
    *row.last().unwrap()
}

fn inspect<T: DeserializeOwned>(value: &Value, path: &str, findings: &mut Vec<String>) {
    let Some(object) = value.as_object() else {
        return;
    };
    let accepted = fields::<T>();
    for key in object
        .keys()
        .filter(|key| !accepted.contains(&key.as_str()))
    {
        let location = format!("{path}/{key}");
        let nearest = accepted
            .iter()
            .min_by_key(|candidate| (distance(key, candidate), **candidate));
        let suggestion = nearest.filter(|candidate| distance(key, candidate) <= 2);
        findings.push(match suggestion {
            Some(candidate) => format!("{location}; did you mean {candidate}?"),
            None => location,
        });
    }
}

fn list<T: DeserializeOwned>(value: &Value, path: &str, findings: &mut Vec<String>) {
    if let Some(items) = value.as_array() {
        for (index, item) in items.iter().enumerate() {
            inspect::<T>(item, &format!("{path}/{index}"), findings);
        }
    }
}

pub(crate) fn unknown_keys(path: &Path, value: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    match path.file_name().and_then(|name| name.to_str()) {
        Some("main.json") => {
            inspect::<MainConfig>(value, "", &mut findings);
            inspect::<AuthorConfig>(&value["author"], "/author", &mut findings);
            inspect::<SearchConfig>(&value["search"], "/search", &mut findings);
            inspect::<SpareTile>(&value["spareTile"], "/spareTile", &mut findings);
            list::<SpareTileLink>(
                &value["spareTile"]["links"],
                "/spareTile/links",
                &mut findings,
            );
        }
        Some("menu.json") => {
            inspect::<MenuConfig>(value, "", &mut findings);
            list::<RawMenuItem>(&value["mainMenu"], "/mainMenu", &mut findings);
            list::<RawSocialItem>(&value["socials"], "/socials", &mut findings);
        }
        Some("page.json") => {
            inspect::<PageConfig>(value, "", &mut findings);
            inspect::<EventConfig>(&value["event"], "/event", &mut findings);
            inspect::<FeedConfig>(&value["feed"], "/feed", &mut findings);
            inspect::<AgendaConfig>(&value["agenda"], "/agenda", &mut findings);
            inspect::<LatestConfig>(&value["latest"], "/latest", &mut findings);
            list::<LumaEvent>(&value["lumaEvents"], "/lumaEvents", &mut findings);
            list::<Notification>(
                &value["leftNotifications"],
                "/leftNotifications",
                &mut findings,
            );
            list::<Notification>(
                &value["rightNotifications"],
                "/rightNotifications",
                &mut findings,
            );
        }
        Some("images.json") => {
            inspect::<ImageList>(value, "", &mut findings);
            list::<ImageReference>(&value["list"], "/list", &mut findings);
        }
        _ => {}
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn typos_at_every_configuration_level_have_paths_and_suggestions() {
        let cases = [
            (
                "main.json",
                json!({"siteTtle":"x", "author":{"nme":"x"},
                "search":{"deph":1}, "spareTile":{"titel":"x", "links":[{"ttle":"x"}]}}),
                vec![
                    "/siteTtle",
                    "/author/nme",
                    "/search/deph",
                    "/spareTile/titel",
                    "/spareTile/links/0/ttle",
                ],
            ),
            (
                "menu.json",
                json!({"mainMen":[], "mainMenu":[{"ur":"/"}], "socials":[{"labl":"x"}]}),
                vec!["/mainMen", "/mainMenu/0/ur", "/socials/0/labl"],
            ),
            (
                "page.json",
                json!({"sumary":"x", "event":{"timeZon":"UTC"},
                "feed":{"limt":2}, "agenda":{"deph":2}, "latest":{"frm":"/"},
                "lumaEvents":[{"eventIdd":"evt-x"}], "leftNotifications":[{"conten":"x"}],
                "rightNotifications":[{"imge":"x"}]}),
                vec![
                    "/sumary",
                    "/event/timeZon",
                    "/feed/limt",
                    "/agenda/deph",
                    "/latest/frm",
                    "/lumaEvents/0/eventIdd",
                    "/leftNotifications/0/conten",
                    "/rightNotifications/0/imge",
                ],
            ),
            (
                "images.json",
                json!({"titel":"x", "list":[{"altTet":"x"}]}),
                vec!["/titel", "/list/0/altTet"],
            ),
        ];
        for (file, value, expected) in cases {
            let findings = unknown_keys(Path::new(file), &value);
            assert_eq!(findings.len(), expected.len(), "{file}: {findings:?}");
            for (finding, path) in findings.iter().zip(expected) {
                assert!(
                    finding.starts_with(&format!("{path}; did you mean ")),
                    "{finding}"
                );
            }
        }
        assert_eq!(
            unknown_keys(Path::new("page.json"), &json!({"sumary":null})),
            ["/sumary; did you mean summary?"]
        );
    }

    #[test]
    fn optional_nulls_boolean_settings_and_opaque_text_are_not_unknown_keys() {
        for (file, value) in [
            (
                "main.json",
                json!({"description":null,"search":true,"spareTile":null}),
            ),
            (
                "page.json",
                json!({"summary":null,"event":null,"feed":false,"agenda":true,
                "content":"{\"sumary\": 1}","latest":{"from":["/a","/b"]}}),
            ),
        ] {
            assert!(unknown_keys(Path::new(file), &value).is_empty());
        }
        let findings = unknown_keys(
            Path::new("page.json"),
            &json!({"_note":1,"x-note":2,"$schema":"url"}),
        );
        assert_eq!(findings.len(), 3);
    }

    #[test]
    fn readme_configuration_examples_use_accepted_keys() {
        let mut tested = 0;
        for block in include_str!("../README.md").split("```json\n").skip(1) {
            let text = block.split("```").next().unwrap();
            let Ok(value) = serde_json::from_str::<Value>(text)
                .or_else(|_| serde_json::from_str(&format!("{{{text}}}")))
            else {
                continue;
            };
            let file = if value.get("siteTitle").is_some() {
                "main.json"
            } else if value.get("mainMenu").is_some() {
                "menu.json"
            } else if value.get("list").is_some() {
                "images.json"
            } else if value.get("content").is_some()
                || value.get("feed").is_some()
                || value.get("event").is_some()
                || value.get("lumaEvents").is_some()
            {
                "page.json"
            } else {
                continue;
            };
            assert!(
                unknown_keys(Path::new(file), &value).is_empty(),
                "{file}: {text}"
            );
            tested += 1;
        }
        assert!(tested > 0);
    }
}
