use git2::Oid;
use serde::{ser::Serialize, Serializer};
use std::{cmp::Eq, collections::HashSet};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub(crate) struct SessionId(pub [u8; 20]);

impl ToString for SessionId {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .fold("".to_string(), |acc, cur| acc + &format!("{:02x}", cur))
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        let mut ret = [0; 20];
        for (i, c) in s.bytes().enumerate() {
            let c = if '0' as u8 <= c && c <= '9' as u8 {
                c - '0' as u8
            } else if 'a' as u8 <= c && c <= 'f' as u8 {
                c - 'a' as u8 + 10
            } else {
                panic!();
            };
            ret[i / 2] |= c << ((1 - i % 2) * 4);
        }
        Self(ret)
    }
}

impl Serialize for SessionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string();
        s.serialize(serializer)
    }
}

pub(crate) struct Session {
    pub checked_commits: HashSet<Oid>,
    pub continue_commits: HashSet<Oid>,
}

#[cfg(test)]
mod test {
    use super::SessionId;
    use rand::prelude::*;
    use std::collections::HashMap;

    #[test]
    fn test_eq() {
        let s = random();
        assert_eq!(SessionId(s), SessionId(s));
        let s_id = SessionId(s);
        assert_eq!(s_id, SessionId::from(&s_id.to_string() as &str));
    }

    #[test]
    fn test_hashmap() {
        let mut map = HashMap::<SessionId, i32>::new();
        let s = random();
        map.insert(SessionId(s), 42);
        assert_eq!(map.get_mut(&SessionId(s)), Some(&mut 42));
        assert_eq!(
            map.contains_key(&SessionId::from(&SessionId(s).to_string() as &str)),
            true
        );
    }
}
