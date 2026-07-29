#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashType {
    System,
    Rapid,
    Murmur3,
    Sip,
}

pub trait IHash {
    fn hash_calc(&self, hash_type: HashType) -> u64;

    fn hash(&self) -> u64 {
        self.hash_calc(self.hash_type())
    }

    fn hash_get(&self) -> u64 {
        self.hash()
    }

    fn hash_get_as(&self, hash_type: HashType) -> u64 {
        self.hash_calc(hash_type)
    }

    fn hash_type(&self) -> HashType {
        HashType::Rapid
    }
}
