use git2::Oid;
use serde::{ser::Serialize, Serializer};
use std::{cmp::Eq, collections::HashSet};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub(crate) struct SessionId(pub [u8; 20]);

impl ToString for SessionId {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .fold("".to_string(), |acc, cur| acc + &format!("{:x}", cur))
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        let mut ret = [0; 20];
        for (i, c) in s.bytes().enumerate() {
            ret[i / 2] |= c << (i % 2 * 4);
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
        assert_eq!(SessionId(s), SessionId::from(&SessionId(s).to_string() as &str));
    }

    #[test]
    fn test_hashmap() {
        let mut map = HashMap::<SessionId, i32>::new();
        let s = random();
        map.insert(SessionId(s), 42);
        assert_eq!(map.get_mut(&SessionId(s)), Some(&mut 42));
        assert_eq!(map.contains_key(&SessionId::from(&SessionId(s).to_string() as &str)), true);
    }
}
