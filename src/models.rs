use crate::schema::fetched_files;

#[derive(Queryable, Clone)]
pub struct FetchedFile {
    pub n: i64,
    pub idx: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "fetched_files"]
pub struct NewFetchedFile {
    pub n: i64,
    pub idx: i64,
}
