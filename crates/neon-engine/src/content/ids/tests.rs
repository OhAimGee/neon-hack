use proptest::prelude::*;

use super::*;

#[test]
fn every_namespace_checks_the_shared_alphabet() {
    assert_eq!(QuestId::new("m05").unwrap().as_str(), "m05");
    assert_eq!(SiteId::new(""), Err(IdError::Empty));
    assert_eq!(FileId::new(&"a".repeat(49)), Err(IdError::TooLong));
    assert_eq!(ItemId::new("Stealth"), Err(IdError::ForbiddenChar('S')));
    assert!("echo7".parse::<ContactId>().is_ok());
    assert!("echo 7".parse::<ContactId>().is_err());
}

#[test]
fn deserialization_validates_and_the_text_form_is_the_plain_id() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Holder {
        quest: QuestId,
    }
    let ok: Holder = toml::from_str("quest = \"m05\"").unwrap();
    assert_eq!(ok.quest.to_string(), "m05");
    assert_eq!(toml::to_string(&ok).unwrap().trim_end(), "quest = \"m05\"");
    assert!(toml::from_str::<Holder>("quest = \"M 05\"").is_err());
}

#[test]
fn the_content_shape_refuses_a_leading_dash_or_underscore() {
    for good in ["m05", "0day", "a-b_c", &"a".repeat(48)] {
        assert!(valid_id(good), "{good}");
    }
    for bad in ["", "-a", "_a", "A", "a b", &"a".repeat(49)] {
        assert!(!valid_id(bad), "{bad}");
    }
}

proptest! {
    #[test]
    fn a_valid_content_id_is_always_a_valid_typed_id(id in "[a-z0-9][a-z0-9_-]{0,47}") {
        prop_assert!(valid_id(&id));
        prop_assert_eq!(QuestId::new(&id).unwrap().to_string(), id);
    }

    #[test]
    fn no_text_makes_a_typed_id_panic(id in ".{0,60}") {
        let _ = SiteId::new(&id);
        let _ = id.parse::<ContactId>();
    }
}
