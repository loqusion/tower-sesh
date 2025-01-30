use time::OffsetDateTime;

#[derive(Clone, Debug)]
pub struct Record {
    expiry: OffsetDateTime,
}

impl Record {
    pub fn unix_timestamp(&self) -> i64 {
        self.expiry.unix_timestamp()
    }
}
