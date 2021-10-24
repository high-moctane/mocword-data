create table one_grams (
    id       integer primary key,
    word     text    not null,
    score    integer not null
);

create table two_grams (
    id       integer primary key,
    word1_id integer not null,
    word2_id integer not null,
    score    integer not null
);

create table three_grams (
    id       integer primary key,
    word1_id integer not null,
    word2_id integer not null,
    word3_id integer not null,
    score    integer not null
);

create table four_grams (
    id       integer primary key,
    word1_id integer not null,
    word2_id integer not null,
    word3_id integer not null,
    word4_id integer not null,
    score    integer not null
);

create table five_grams (
    id       integer primary key,
    word1_id integer not null,
    word2_id integer not null,
    word3_id integer not null,
    word4_id integer not null,
    word5_id integer not null,
    score    integer not null
);

create table fetched_files (
    n   integer not null,
    idx integer not null,

    primary key(n, idx)
);
